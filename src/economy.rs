use crate::events::{EventLog, GoodId, Season, WorldEvent};
use crate::ids::SettlementId;
use crate::world::{Settlement, WorldState};

const MIN_PRICE: f64 = 0.25;
const MAX_PRICE: f64 = 8.0;

pub fn update_prices(world: &mut WorldState, log: &mut EventLog) {
    for settlement in &mut world.settlements {
        let desired = settlement.market.desired_grain_stock.max(1);
        let stock = settlement.grain_inventory;
        let ratio = stock as f64 / desired as f64;
        let old = settlement.market.grain_price;
        let target = if ratio < 0.5 {
            old * 1.08
        } else if ratio < 0.85 {
            old * 1.03
        } else if ratio > 1.5 {
            old * 0.97
        } else if ratio > 2.0 {
            old * 0.92
        } else {
            old
        };
        let new_price = target.clamp(MIN_PRICE, MAX_PRICE);
        if (new_price - old).abs() > 0.001 {
            settlement.market.grain_price = new_price;
            log.emit(
                world.day,
                WorldEvent::PriceAdjusted {
                    settlement: settlement.id,
                    good: GoodId::Grain,
                    old_price: old,
                    new_price,
                },
                None,
            );
        }
    }
}

pub fn run_production(world: &mut WorldState, log: &mut EventLog, season: Season) {
    let mult = season.farm_yield_multiplier();
    let props = world.properties.clone();
    for settlement in &mut world.settlements {
        settlement.farm_daily_capacity = settlement.base_farm_daily_capacity;
        crate::property::apply_property_effects_to_settlement(settlement, &props);
    }
    let labor: Vec<(SettlementId, f64)> = world
        .settlements
        .iter()
        .map(|s| {
            (
                s.id,
                crate::npc::decisions::farmer_work_fraction(world, s.id),
            )
        })
        .collect();
    for settlement in &mut world.settlements {
        let labor = labor
            .iter()
            .find(|(id, _)| *id == settlement.id)
            .map(|(_, l)| *l)
            .unwrap_or(1.0);
        let base = settlement.farm_daily_capacity;
        let produced = ((base as f64) * mult * labor).floor() as u32;
        if produced > 0 {
            settlement.grain_inventory = settlement.grain_inventory.saturating_add(produced);
            log.emit(
                world.day,
                WorldEvent::GoodsProduced {
                    settlement: settlement.id,
                    good: GoodId::Grain,
                    quantity: produced,
                },
                None,
            );
        }
    }
}

pub fn run_consumption(world: &mut WorldState, log: &mut EventLog) -> ConsumptionSummary {
    let mut summary = ConsumptionSummary::default();
    let day = world.day;
    let settlement_ids: Vec<_> = world.settlements.iter().map(|s| s.id).collect();
    for sid in settlement_ids {
        let need = world.daily_consumption(sid);
        let population = world.settlement(sid).population;
        let available = world.settlement(sid).grain_inventory;
        let eaten = need.min(available);
        world.settlement_mut(sid).grain_inventory -= eaten;
        if eaten > 0 {
            log.emit(
                day,
                WorldEvent::GoodsConsumed {
                    settlement: sid,
                    good: GoodId::Grain,
                    quantity: eaten,
                },
                None,
            );
            log.emit(
                day,
                WorldEvent::PopulationFed {
                    settlement: sid,
                    people: population,
                    grain_used: eaten,
                },
                None,
            );
            summary.total_fed += eaten;
        }
        if eaten < need {
            let shortfall = need - eaten;
            summary.total_starved += shortfall;
            log.emit(
                day,
                WorldEvent::PopulationStarved {
                    settlement: sid,
                    people: population,
                },
                None,
            );
        }
    }
    summary
}

#[derive(Default, Clone, Debug)]
pub struct ConsumptionSummary {
    pub total_fed: u32,
    pub total_starved: u32,
}

pub fn snapshot_settlement(s: &Settlement) -> SettlementSnapshot {
    SettlementSnapshot {
        name: s.name.clone(),
        population: s.population,
        grain: s.grain_inventory,
        price: s.market.grain_price,
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SettlementSnapshot {
    pub name: String,
    pub population: u32,
    pub grain: u32,
    pub price: f64,
}
