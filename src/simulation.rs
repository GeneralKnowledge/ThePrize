use crate::consequences::apply_world_action;
use crate::economy::{run_production, update_prices, ConsumptionSummary};
use crate::invariants::{check_no_aggregate_and_npc_double_consume, validate_world, ViolationSeverity};
use crate::merchant::tick_merchant;
use crate::npc::decisions::{run_npc_decisions, tick_npc_hunger};
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
        self.tick_bridge_environmental_load();
        self.tick_bridge_repair();

        debug_assert!(check_no_aggregate_and_npc_double_consume(&self.log).is_ok());
        if cfg!(debug_assertions) {
            let report = validate_world(&self.world, &self.log);
            debug_assert!(
                !report
                    .violations
                    .iter()
                    .any(|v| v.severity == ViolationSeverity::Error),
                "integrity errors: {:?}",
                report.violations
            );
        }

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
            crate::structure_sync::apply_scheduled_collapse(self, cause);
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
        if route != RouteId(0) {
            return;
        }
        let outcome = crate::structures::apply_impact(&mut self.world.bridge_structure, 900);
        crate::structure_sync::record_structure_outcome(self, &outcome, cause, Some(cause));
        crate::structure_sync::sync_bridge_route(self, cause, Some(cause));
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
        if !route.repair_in_progress {
            return;
        }
        let village = SettlementId(0);
        let material_cost = 14u32;
        if self.world.settlement(village).treasury_coin < material_cost {
            return;
        }
        self.world.settlement_mut(village).treasury_coin -= material_cost;
        let parent = self
            .log
            .events_of_kind(|e| matches!(e, WorldEvent::BridgeRepairStarted { .. }))
            .last()
            .map(|r| r.cause);
        let outcome = crate::structures::apply_repair(
            &mut self.world.bridge_structure,
            12,
            8,
            6,
        );
        let (_, cause) = self.log.emit(
            self.world.day,
            WorldEvent::StructureLoadApplied {
                route: route_id,
                load_units: 0,
                cause: CauseId(0),
            },
            parent,
        );
        if let Some(rec) = self.log.records.last_mut() {
            if let WorldEvent::StructureLoadApplied { cause, .. } = &mut rec.event {
                *cause = rec.cause;
            }
        }
        crate::structure_sync::record_structure_outcome(self, &outcome, cause, parent);
        crate::structure_sync::sync_bridge_route(self, cause, parent);
        let passable = self.world.bridge_structure.bridge_passable();
        let route = self.world.route_mut(route_id);
        route.repair_progress += 1;
        if passable && route.repair_progress >= route.repair_days_required {
            route.repair_in_progress = false;
            route.repair_progress = 0;
        }
    }

    fn tick_bridge_environmental_load(&mut self) {
        let route_id = RouteId(0);
        let day = self.world.day;
        if self.world.route(route_id).passable {
            let load_units = 12u32;
            let outcome =
                crate::structures::apply_sustained_load(&mut self.world.bridge_structure, load_units);
            if outcome.applied {
                let (_, cause) = self.log.emit(
                    day,
                    WorldEvent::StructureLoadApplied {
                        route: route_id,
                        load_units,
                        cause: CauseId(0),
                    },
                    None,
                );
                if let Some(rec) = self.log.records.last_mut() {
                    if let WorldEvent::StructureLoadApplied { cause, .. } = &mut rec.event {
                        *cause = rec.cause;
                    }
                }
                crate::structure_sync::record_structure_outcome(self, &outcome, cause, None);
                crate::structure_sync::sync_bridge_route(self, cause, None);
            }
        }
        if day % 7 == 0 {
            let outcome = crate::structures::apply_weather(&mut self.world.bridge_structure);
            if outcome.applied {
                let (_, cause) = self.log.emit(
                    day,
                    WorldEvent::StructureLoadApplied {
                        route: route_id,
                        load_units: 0,
                        cause: CauseId(0),
                    },
                    None,
                );
                if let Some(rec) = self.log.records.last_mut() {
                    if let WorldEvent::StructureLoadApplied { cause, .. } = &mut rec.event {
                        *cause = rec.cause;
                    }
                }
                crate::structure_sync::record_structure_outcome(self, &outcome, cause, None);
                crate::structure_sync::sync_bridge_route(self, cause, None);
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
