use crate::economy::{run_production, update_prices, ConsumptionSummary};
use crate::npc::decisions::{run_npc_decisions, tick_npc_hunger};
use crate::npc::inform_bridge_blocked;
use crate::events::{EventLog, GoodId, InterventionKind, Season, WorldEvent};
use crate::ids::{RouteId, SettlementId};
use crate::world::{MerchantLocation, WorldState};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub struct SimulationConfig {
    pub seed: u64,
    pub bridge_block_day: Option<u32>,
    pub interventions: Vec<ScheduledIntervention>,
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

        tick_npc_hunger(&mut self.world, &mut self.log);
        run_npc_decisions(&mut self.world, &mut self.log);
        run_production(&mut self.world, &mut self.log, season);
        self.tick_merchant(day);
        update_prices(&mut self.world, &mut self.log);
        let consumption = ConsumptionSummary::default();
        self.tick_bridge_repair();

        SimTickResult { consumption }
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
        self.log.emit(
            self.world.day,
            WorldEvent::RouteBlocked { route, cause },
            Some(cause),
        );
        let day = self.world.day;
        inform_bridge_blocked(&mut self.world, day);
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
            let cause = self.log.mint_cause();
            self.log.emit(
                self.world.day,
                WorldEvent::RouteRepaired {
                    route: route_id,
                    cause,
                },
                None,
            );
        }
    }

    fn player_haul_grain_alternative(&mut self, cause: crate::ids::CauseId) {
        let village = SettlementId(0);
        let town = SettlementId(1);
        let haul_amount = 30u32;
        let available = self.world.settlement(village).grain_inventory;
        let qty = haul_amount.min(available);
        if qty == 0 {
            return;
        }
        self.world.settlement_mut(village).grain_inventory -= qty;
        self.world.settlement_mut(town).grain_inventory += qty;
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

    /// Primary trade lane: export grain from Hollowbrook to Millford when the town is short.
    fn tick_merchant(&mut self, day: u32) {
        let merchant_id = self.world.merchant.id;
        let village = SettlementId(0);
        let town = SettlementId(1);

        if let MerchantLocation::Travelling {
            destination,
            days_remaining,
            carrying_grain,
        } = self.world.merchant.location
        {
            if days_remaining > 1 {
                self.world.merchant.location = MerchantLocation::Travelling {
                    destination,
                    days_remaining: days_remaining - 1,
                    carrying_grain,
                };
                return;
            }
            if days_remaining == 1 {
                if carrying_grain > 0 && destination == town {
                    self.world.settlement_mut(town).grain_inventory += carrying_grain;
                    let dest_price = self.world.settlement(town).market.grain_price;
                    let sale = (carrying_grain as f64 * dest_price * 0.95).floor() as u32;
                    self.world.merchant.coin += sale;
                    self.log.emit(
                        day,
                        WorldEvent::GoodsTransferred {
                            from: village,
                            to: town,
                            good: GoodId::Grain,
                            quantity: carrying_grain,
                            via_merchant: merchant_id,
                        },
                        None,
                    );
                }
                self.world.merchant.location = MerchantLocation::At(destination);
                return;
            }
        }

        let passable = self.world.bridge_between(village, town).expect("bridge").passable;

        let current = match self.world.merchant.location {
            MerchantLocation::At(s) => s,
            MerchantLocation::Travelling { .. } => return,
        };

        let town_stock = self.world.settlement(town).grain_inventory;
        let town_desired = self.world.settlement(town).market.desired_grain_stock;
        let village_stock = self.world.settlement(village).grain_inventory;
        let village_reserve = self.world.settlement(village).market.desired_grain_stock;

        let town_needs_imports = town_stock + 8 < town_desired;
        let village_can_export = village_stock > village_reserve + 20;

        if current == village {
            if !town_needs_imports || !village_can_export {
                return;
            }
            if !passable {
                return;
            }
            let town_deficit = town_desired.saturating_sub(town_stock);
            if town_deficit < 6 {
                return;
            }
            let max_export = village_stock.saturating_sub(village_reserve + 40);
            let load = self
                .world
                .merchant
                .grain_capacity
                .min(max_export)
                .min(town_deficit + 4);
            if load == 0 {
                return;
            }
            self.world.settlement_mut(village).grain_inventory -= load;
            self.world.settlement_mut(town).grain_inventory += load;
            let dest_price = self.world.settlement(town).market.grain_price;
            let sale = (load as f64 * dest_price * 0.92).floor() as u32;
            self.world.merchant.coin += sale.saturating_sub(2);
            self.log.emit(
                day,
                WorldEvent::GoodsTransferred {
                    from: village,
                    to: town,
                    good: GoodId::Grain,
                    quantity: load,
                    via_merchant: merchant_id,
                },
                None,
            );
            self.world.merchant.location = MerchantLocation::At(village);
            return;
        }

        // At town after a multi-day leg: head back to the village when imports are still needed.
        if town_needs_imports && village_can_export && passable {
            self.world.merchant.location = MerchantLocation::At(village);
        }
    }

    pub fn run_days(&mut self, days: u32) {
        for _ in 0..days {
            self.advance_day();
        }
    }
}
