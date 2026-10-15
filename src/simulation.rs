use crate::consequences::apply_world_action;
use crate::economy::{run_production, update_prices, ConsumptionSummary};
use crate::invariants::{check_no_aggregate_and_npc_double_consume, check_world_invariants};
use crate::merchant::{on_route_blocked, tick_merchant};
use crate::npc::decisions::{run_npc_decisions, tick_npc_hunger};
use crate::npc::inform_bridge_blocked;
use crate::events::{EventLog, GoodId, InterventionKind, Season, WorldEvent};
use crate::ids::{CauseId, RouteId, SettlementId};
use crate::ledger::transfer_settlement_grain;
use crate::world_action::ScheduledAction;
use crate::world::WorldState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub struct SimulationConfig {
    pub seed: u64,
    pub bridge_block_day: Option<u32>,
    pub interventions: Vec<ScheduledIntervention>,
    pub scheduled_actions: Vec<ScheduledAction>,
}

#[derive(Clone, Debug)]
pub struct ScheduledIntervention {
    pub day: u32,
    pub kind: InterventionKind,
}

pub struct SimTickResult {
    pub consumption: ConsumptionSummary,
}

pub struct WorldSimulator {
    pub world: WorldState,
    pub log: EventLog,
    pub _rng: ChaCha8Rng,
    config: SimulationConfig,
}

impl WorldSimulator {
    pub fn new(world: WorldState, config: SimulationConfig) -> Self {
        let _rng = ChaCha8Rng::seed_from_u64(config.seed);
        Self {
            world,
            log: EventLog::new(),
            _rng,
            config,
        }
    }

    pub fn advance_day(&mut self) -> SimTickResult {
        self.world.day += 1;
        let day = self.world.day;
        let season = Season::from_day(day);

        self.log.emit(
            day,
            WorldEvent::DayAdvanced { day, season },
            None,
        );

        self.apply_scheduled_disruptions(day);
        self.apply_scheduled_interventions(day);
        self.apply_scheduled_actions(day);

        credit_settlement_treasuries(&mut self.world);
        tick_npc_hunger(&mut self.world, &mut self.log);
        run_npc_decisions(&mut self.world, &mut self.log);
        self.flush_pending_actions();
        run_production(&mut self.world, &mut self.log, season);
        tick_merchant(&mut self.world, &mut self.log, day);
        update_prices(&mut self.world, &mut self.log);
        self.tick_bridge_repair();

        debug_assert!(check_world_invariants(&self.world).is_ok());
        debug_assert!(check_no_aggregate_and_npc_double_consume(&self.log).is_ok());

        SimTickResult {
            consumption: ConsumptionSummary::default(),
        }
    }

    fn apply_scheduled_disruptions(&mut self, day: u32) {
        if self.config.bridge_block_day == Some(day) {
            let route = RouteId(0);
            let (_, cause) = self.log.emit(
                day,
                WorldEvent::ScheduledDisruption { route, day },
                None,
            );
            self.block_route(route, cause);
        }
    }

    fn flush_pending_actions(&mut self) {
        let pending = std::mem::take(&mut self.world.pending_actions);
        for action in pending {
            apply_world_action(self, action, None);
        }
    }

    fn apply_scheduled_actions(&mut self, day: u32) {
        let actions: Vec<_> = self
            .config
            .scheduled_actions
            .iter()
            .filter(|a| a.day == day)
            .cloned()
            .collect();
        for scheduled in actions {
            apply_world_action(self, scheduled.action, None);
        }
    }

    fn apply_scheduled_interventions(&mut self, day: u32) {
        let planned: Vec<_> = self
            .config
            .interventions
            .iter()
            .filter(|i| i.day == day)
            .cloned()
            .collect();
        for intervention in planned {
            let (_, cause) = self.log.emit(
                day,
                WorldEvent::PlayerIntervention {
                    kind: intervention.kind,
                },
                None,
            );
            match intervention.kind {
                InterventionKind::RepairBridge => {
                    self.start_bridge_repair(RouteId(0), cause);
                }
                InterventionKind::HaulGrainAlternative => {
                    self.player_haul_grain_alternative(cause);
                }
            }
        }
    }

    pub fn block_route(&mut self, route: RouteId, cause: crate::ids::CauseId) {
        let r = self.world.route_mut(route);
        r.passable = false;
        r.repair_progress = 0;
        r.repair_in_progress = false;
        let (_, block_cause) = self.log.emit(
            self.world.day,
            WorldEvent::RouteBlocked {
                route,
                cause: CauseId(0),
            },
            Some(cause),
        );
        if let Some(rec) = self.log.records.last_mut() {
            if let WorldEvent::RouteBlocked { cause, .. } = &mut rec.event {
                *cause = rec.cause;
            }
        }
        let day = self.world.day;
        inform_bridge_blocked(&mut self.world, day);
        on_route_blocked(&mut self.world, &mut self.log, block_cause);
        for settlement in [SettlementId(0), SettlementId(1)] {
            self.log.emit(
                day,
                WorldEvent::RumourSpread {
                    settlement,
                    topic: "bridge_unsafe".into(),
                    cause: block_cause,
                },
                Some(block_cause),
            );
        }
    }

    pub fn start_bridge_repair(&mut self, route: RouteId, cause: crate::ids::CauseId) {
        let r = self.world.route_mut(route);
        r.repair_in_progress = true;
        r.repair_progress = 0;
        self.log.emit(
            self.world.day,
            WorldEvent::BridgeRepairStarted { route, cause },
            Some(cause),
        );
    }

    fn tick_bridge_repair(&mut self) {
        let route_id = RouteId(0);
        let route = self.world.route(route_id);
        if !route.repair_in_progress || route.passable {
            return;
        }
        let route = self.world.route_mut(route_id);
        route.repair_progress += 1;
        if route.repair_progress >= route.repair_days_required {
            route.passable = true;
            route.repair_in_progress = false;
            route.repair_progress = 0;
            let parent = self
                .log
                .events_of_kind(|e| matches!(e, WorldEvent::BridgeRepairStarted { .. }))
                .last()
                .map(|r| r.cause);
            self.log.emit(
                self.world.day,
                WorldEvent::RouteRepaired {
                    route: route_id,
                    cause: CauseId(0),
                },
                parent,
            );
            if let Some(rec) = self.log.records.last_mut() {
                if let WorldEvent::RouteRepaired { cause, .. } = &mut rec.event {
                    *cause = rec.cause;
                }
            }
        }
    }

    fn player_haul_grain_alternative(&mut self, cause: crate::ids::CauseId) {
        let village = SettlementId(0);
        let town = SettlementId(1);
        let qty = transfer_settlement_grain(&mut self.world, village, town, 30);
        if qty == 0 {
            return;
        }
        self.log.emit(
            self.world.day,
            WorldEvent::AlternativeRouteUsed {
                from: village,
                to: town,
                good: GoodId::Grain,
                quantity: qty,
                cause,
            },
            Some(cause),
        );
    }

    pub fn run_days(&mut self, days: u32) {
        for _ in 0..days {
            self.advance_day();
        }
    }
}

/// Abstract regional tax/tithing credited daily so local wages remain payable without minting coin on NPCs.
fn credit_settlement_treasuries(world: &mut WorldState) {
    for settlement in &mut world.settlements {
        let inflow = settlement.population.saturating_mul(2);
        settlement.treasury_coin = settlement.treasury_coin.saturating_add(inflow);
    }
}
