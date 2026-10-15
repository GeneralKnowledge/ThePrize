use crate::events::WorldEvent;
use crate::ids::{CauseId, RouteId};
use crate::npc::inform_bridge_blocked;
use crate::simulation::WorldSimulator;
use crate::structures::StructureInteractionOutcome;

pub fn record_structure_outcome(
    sim: &mut WorldSimulator,
    outcome: &StructureInteractionOutcome,
    cause: CauseId,
    parent: Option<CauseId>,
) {
    let route = sim.world.bridge_structure.route_id;
    if outcome.safety_before != outcome.safety_after {
        let (_, cond_cause) = sim.log.emit(
            sim.world.day,
            WorldEvent::StructuralConditionChanged {
                route,
                safety: format!("{:?}", outcome.safety_after),
                aggregate_integrity: sim.world.bridge_structure.aggregate_integrity(),
                cause: CauseId(0),
            },
            parent.or(Some(cause)),
        );
        if let Some(rec) = sim.log.records.last_mut() {
            if let WorldEvent::StructuralConditionChanged { cause, .. } = &mut rec.event {
                *cause = cond_cause;
            }
        }
    }
    for change in &outcome.component_changes {
        sim.log.emit(
            sim.world.day,
            WorldEvent::ComponentIntegrityChanged {
                route,
                component: format!("{:?}", change.component),
                integrity_before: change.integrity_before,
                integrity_after: change.integrity_after,
                cause,
            },
            Some(cause),
        );
    }
}

pub fn sync_bridge_route(sim: &mut WorldSimulator, cause: CauseId, parent: Option<CauseId>) {
    let route_id = RouteId(0);
    let was_passable = sim.world.route(route_id).passable;
    let passable = sim.world.bridge_structure.bridge_passable();
    sim.world.route_mut(route_id).passable = passable;
    let now_passable = passable;
    let day = sim.world.day;

    if was_passable && !now_passable {
        let (_, block_cause) = sim.log.emit(
            day,
            WorldEvent::RouteBlocked {
                route: route_id,
                cause: CauseId(0),
            },
            parent.or(Some(cause)),
        );
        if let Some(rec) = sim.log.records.last_mut() {
            if let WorldEvent::RouteBlocked { cause, .. } = &mut rec.event {
                *cause = block_cause;
            }
        }
        inform_bridge_blocked(&mut sim.world, day);
        crate::merchant::on_route_blocked(&mut sim.world, &mut sim.log, block_cause);
        for settlement in [crate::ids::SettlementId(0), crate::ids::SettlementId(1)] {
            sim.log.emit(
                day,
                WorldEvent::RumourSpread {
                    settlement,
                    topic: "bridge_unsafe".into(),
                    cause: block_cause,
                },
                Some(block_cause),
            );
        }
    } else if !was_passable && now_passable {
        let repair_parent = sim
            .log
            .events_of_kind(|e| matches!(e, WorldEvent::BridgeRepairStarted { .. }))
            .last()
            .map(|r| r.cause);
        sim.log.emit(
            day,
            WorldEvent::RouteRepaired {
                route: route_id,
                cause: CauseId(0),
            },
            repair_parent,
        );
        if let Some(rec) = sim.log.records.last_mut() {
            if let WorldEvent::RouteRepaired { cause, .. } = &mut rec.event {
                *cause = rec.cause;
            }
        }
    }
}

pub fn apply_scheduled_collapse(sim: &mut WorldSimulator, cause: CauseId) {
    let outcome = crate::structures::apply_impact(&mut sim.world.bridge_structure, 920);
    record_structure_outcome(sim, &outcome, cause, Some(cause));
    sync_bridge_route(sim, cause, Some(cause));
}
