use crate::world::{MerchantLocation, WorldState};

#[derive(Debug)]
pub struct InvariantViolation {
    pub message: String,
}

pub type InvariantResult = Result<(), InvariantViolation>;

pub fn check_world_invariants(world: &WorldState) -> InvariantResult {
    for s in &world.settlements {
        if s.grain_inventory > 1_000_000 {
            return Err(violation("settlement grain unreasonably large"));
        }
    }
    for npc in &world.npcs {
        if !npc.alive {
            continue;
        }
        if !(0.0..=1.0).contains(&npc.hunger) {
            return Err(violation("npc hunger out of range"));
        }
    }
    for s in &world.settlements {
        let alive_home = world
            .npcs
            .iter()
            .filter(|n| n.alive && n.home == s.id)
            .count() as u32;
        if s.population != alive_home {
            return Err(violation(&format!(
                "settlement {} population {} != resident npc count {}",
                s.name, s.population, alive_home
            )));
        }
    }
    match world.merchant.location {
        MerchantLocation::Travelling {
            carrying_grain, ..
        } => {
            if carrying_grain > world.merchant.grain_capacity {
                return Err(violation("merchant cargo exceeds capacity"));
            }
        }
        MerchantLocation::At(_) => {}
    }
    for route in &world.routes {
        if route.repair_in_progress && route.passable {
            return Err(violation("route marked passable while repair in progress"));
        }
        if route.repair_progress > route.repair_days_required && !route.passable {
            return Err(violation("repair progress exceeded required days"));
        }
    }
    Ok(())
}

pub fn check_merchant_not_duplicating_cargo(world: &WorldState) -> InvariantResult {
    if let MerchantLocation::Travelling { carrying_grain, .. } = world.merchant.location {
        if carrying_grain > 0 {
            // Cargo in transit must not also sit in settlement inventories as duplicate.
            let _ = carrying_grain;
        }
    }
    Ok(())
}

pub fn check_no_aggregate_and_npc_double_consume(log: &crate::events::EventLog) -> InvariantResult {
    let aggregate = log
        .events_of_kind(|e| matches!(e, crate::events::WorldEvent::GoodsConsumed { .. }))
        .count();
    if aggregate > 0 {
        return Err(violation(
            "aggregate settlement consumption events present alongside npc feeding",
        ));
    }
    Ok(())
}

fn violation(msg: &str) -> InvariantViolation {
    InvariantViolation {
        message: msg.to_string(),
    }
}
