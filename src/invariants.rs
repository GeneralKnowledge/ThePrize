use crate::events::{EventLog, WorldEvent};
use crate::world::{MerchantLocation, WorldState};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrityViolation {
    pub severity: ViolationSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationReport {
    pub violations: Vec<IntegrityViolation>,
}

impl ValidationReport {
    pub fn is_clean(&self) -> bool {
        !self.has_errors()
    }

    pub fn has_errors(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error)
    }

    pub fn push_error(&mut self, code: &'static str, message: impl Into<String>) {
        self.violations.push(IntegrityViolation {
            severity: ViolationSeverity::Error,
            code,
            message: message.into(),
        });
    }

    pub fn push_warning(&mut self, code: &'static str, message: impl Into<String>) {
        self.violations.push(IntegrityViolation {
            severity: ViolationSeverity::Warning,
            code,
            message: message.into(),
        });
    }
}

/// Full validation pass; does not mutate `world` or `log`.
pub fn validate_world(world: &WorldState, log: &EventLog) -> ValidationReport {
    let mut report = ValidationReport::default();
    check_entity_identity(world, &mut report);
    check_inventories_and_money(world, &mut report);
    check_population(world, &mut report);
    check_merchant_state(world, &mut report);
    check_routes(world, &mut report);
    check_npc_lifecycle(world, &mut report);
    check_event_log_consistency(log, &mut report);
    if let Err(v) = check_no_aggregate_and_npc_double_consume(log) {
        report.push_error("double_consume", v.message);
    }
    report
}

pub fn check_world_invariants(world: &WorldState) -> Result<(), InvariantViolation> {
    let report = validate_world(world, &EventLog::default());
    report
        .violations
        .into_iter()
        .find(|v| v.severity == ViolationSeverity::Error)
        .map(|v| InvariantViolation {
            message: format!("[{}] {}", v.code, v.message),
        })
        .map(Err)
        .unwrap_or(Ok(()))
}

#[derive(Debug)]
pub struct InvariantViolation {
    pub message: String,
}

pub type InvariantResult = Result<(), InvariantViolation>;

pub fn check_no_aggregate_and_npc_double_consume(log: &EventLog) -> InvariantResult {
    let aggregate = log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsConsumed { .. }))
        .count();
    if aggregate > 0 {
        return Err(violation(
            "aggregate settlement consumption events present alongside npc feeding",
        ));
    }
    Ok(())
}

fn check_entity_identity(world: &WorldState, report: &mut ValidationReport) {
    let mut npc_ids = HashSet::new();
    for npc in &world.npcs {
        if !npc_ids.insert(npc.id) {
            report.push_error(
                "duplicate_npc_id",
                format!("duplicate PersonId {:?}", npc.id),
            );
        }
    }
    if npc_ids.contains(&world.merchant.id) {
        report.push_error(
            "merchant_npc_id_collision",
            format!("merchant shares PersonId {:?}", world.merchant.id),
        );
    }
    for s in &world.settlements {
        if world
            .settlements
            .iter()
            .filter(|x| x.id == s.id)
            .count()
            > 1
        {
            report.push_error("duplicate_settlement_id", format!("{:?}", s.id));
        }
    }
}

fn check_inventories_and_money(world: &WorldState, report: &mut ValidationReport) {
    for s in &world.settlements {
        if s.grain_inventory > 10_000_000 {
            report.push_warning(
                "grain_suspiciously_high",
                format!("{} grain {}", s.name, s.grain_inventory),
            );
        }
    }
    for npc in &world.npcs {
        if npc.alive && npc.coin > 1_000_000 {
            report.push_warning(
                "npc_coin_high",
                format!("{} coin {}", npc.name, npc.coin),
            );
        }
    }
    if world.merchant.coin > 1_000_000 {
        report.push_warning(
            "merchant_coin_high",
            format!("merchant coin {}", world.merchant.coin),
        );
    }
}

fn check_population(world: &WorldState, report: &mut ValidationReport) {
    for s in &world.settlements {
        let alive_home = world
            .npcs
            .iter()
            .filter(|n| n.alive && n.home == s.id)
            .count() as u32;
        if s.population != alive_home {
            report.push_error(
                "population_mismatch",
                format!(
                    "{} population {} != resident npc count {}",
                    s.name, s.population, alive_home
                ),
            );
        }
    }
}

fn check_merchant_state(world: &WorldState, report: &mut ValidationReport) {
    match world.merchant.location {
        MerchantLocation::Travelling {
            carrying_grain,
            days_remaining,
            from,
            to,
        } => {
            if carrying_grain > world.merchant.grain_capacity {
                report.push_error(
                    "merchant_over_capacity",
                    format!("carrying {carrying_grain} > capacity {}", world.merchant.grain_capacity),
                );
            }
            if days_remaining == 0 {
                report.push_error(
                    "merchant_zero_travel_days",
                    "travelling merchant has days_remaining == 0",
                );
            }
            if from == to {
                report.push_error(
                    "merchant_invalid_trip",
                    "merchant travelling from settlement to itself",
                );
            }
        }
        MerchantLocation::At(_) => {}
    }
}

fn check_routes(world: &WorldState, report: &mut ValidationReport) {
    let bridge_passable = world.bridge_structure.bridge_passable();
    for route in &world.routes {
        if route.id == world.bridge_structure.route_id && route.passable != bridge_passable {
            report.push_error(
                "route_structure_passable_mismatch",
                format!(
                    "route passable {} != structure-derived {}",
                    route.passable, bridge_passable
                ),
            );
        }
        if route.repair_in_progress && route.passable && !bridge_passable {
            report.push_error(
                "route_repair_passable",
                format!(
                    "route {} marked passable while repair in progress but structure is not crossable",
                    route.name
                ),
            );
        }
        if route.repair_progress > route.repair_days_required && !route.passable {
            report.push_error(
                "route_repair_overflow",
                format!("route {} repair progress exceeds required days", route.name),
            );
        }
    }
}

fn check_npc_lifecycle(world: &WorldState, report: &mut ValidationReport) {
    for npc in &world.npcs {
        if !npc.alive {
            continue;
        }
        if !(0.0..=1.0).contains(&npc.hunger) {
            report.push_error(
                "npc_hunger_range",
                format!("{} hunger {}", npc.name, npc.hunger),
            );
        }
    }
}

fn check_event_log_consistency(log: &EventLog, report: &mut ValidationReport) {
    let mut known_causes = HashSet::new();
    for record in &log.records {
        known_causes.insert(record.cause);
    }
    for record in &log.records {
        if let Some(parent) = record.parent_cause {
            if !known_causes.contains(&parent) {
                report.push_error(
                    "orphan_parent_cause",
                    format!(
                        "event {:?} references unknown parent cause {:?}",
                        record.id, parent
                    ),
                );
            }
            if parent == record.cause {
                report.push_error(
                    "self_parent_cause",
                    format!("event {:?} is its own parent", record.id),
                );
            }
        }
    }

    let transfers = log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count();
    let sales = log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantSaleCompleted { .. }))
        .count();
    if transfers != sales {
        report.push_warning(
            "merchant_transfer_sale_mismatch",
            format!("GoodsTransferred ({transfers}) != MerchantSaleCompleted ({sales})"),
        );
    }

    let mut depart_causes = Vec::new();
    for record in &log.records {
        if matches!(record.event, WorldEvent::MerchantDeparted { .. }) {
            depart_causes.push(record.cause);
        }
    }
    for record in &log.records {
        if let WorldEvent::MerchantSaleCompleted { .. } = &record.event {
            if record.parent_cause.is_none() {
                report.push_warning(
                    "sale_missing_trip_cause",
                    format!("sale on day {} lacks parent trip cause", record.day),
                );
            }
        }
    }
    let _ = depart_causes;
}

fn violation(msg: &str) -> InvariantViolation {
    InvariantViolation {
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::initial_world;

    #[test]
    fn valid_initial_world_is_clean() {
        let world = initial_world();
        let report = validate_world(&world, &EventLog::new());
        assert!(report.is_clean(), "{:?}", report.violations);
    }

    #[test]
    fn detects_negative_travel_days() {
        let mut world = initial_world();
        world.merchant.location = MerchantLocation::Travelling {
            from: crate::ids::SettlementId(0),
            to: crate::ids::SettlementId(1),
            days_remaining: 0,
            carrying_grain: 0,
        };
        let report = validate_world(&world, &EventLog::new());
        assert!(report.has_errors());
    }
}
