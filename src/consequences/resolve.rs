use crate::world_action::{Actor, WorldAction};
use crate::events::{GoodId, RelationshipChange, WorldEvent};
use crate::ids::{CauseId, PersonId, PropertyId, RouteId, SettlementId};
use crate::npc::{Job, sync_settlement_population};
use crate::property::{mill_operational, PropertyKind};
use crate::simulation::WorldSimulator;
use serde::{Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct ActionOutcome {
    pub succeeded: bool,
    pub cause: CauseId,
    pub summary: String,
}

pub fn apply_world_action(
    sim: &mut WorldSimulator,
    action: WorldAction,
    parent_cause: Option<CauseId>,
) -> ActionOutcome {
    let day = sim.world.day;
    let (_, cause) = sim.log.emit(
        day,
        WorldEvent::ActionAttempted { action },
        parent_cause,
    );

    let outcome = match action {
        WorldAction::DamageBridge { actor, route } => damage_bridge(sim, route, actor, cause),
        WorldAction::StealSettlementGrain {
            thief,
            settlement,
            quantity,
        } => steal_grain(sim, thief, settlement, quantity, cause),
        WorldAction::KillPerson { killer, victim } => kill_person(sim, killer, victim, cause),
        WorldAction::DamageProperty {
            actor,
            property,
            damage,
        } => damage_property(sim, actor, property, damage, cause),
    };

    outcome
}

fn damage_bridge(
    sim: &mut WorldSimulator,
    route: RouteId,
    actor: Actor,
    cause: CauseId,
) -> ActionOutcome {
    sim.log.emit(
        sim.world.day,
        WorldEvent::StructureDamaged {
            route: Some(route),
            property: None,
            actor,
            severity: 100,
            cause,
        },
        Some(cause),
    );
    sim.block_route(route, cause);
    ActionOutcome {
        succeeded: true,
        cause,
        summary: "Bridge damaged; route is blocked".into(),
    }
}

fn steal_grain(
    sim: &mut WorldSimulator,
    thief: Actor,
    settlement: SettlementId,
    quantity: u32,
    cause: CauseId,
) -> ActionOutcome {
    let available = sim.world.settlement(settlement).grain_inventory;
    let stolen = quantity.min(available);
    if stolen == 0 {
        return ActionOutcome {
            succeeded: false,
            cause,
            summary: "Nothing to steal".into(),
        };
    }
    sim.world.settlement_mut(settlement).grain_inventory -= stolen;

    if let Actor::Npc(id) = thief {
        if let Some(npc) = sim.world.npcs.iter_mut().find(|n| n.id == id && n.alive) {
            npc.grain += stolen;
            npc.remember(sim.world.day, "stole grain from communal stores");
        }
    }

    sim.log.emit(
        sim.world.day,
        WorldEvent::TheftCommitted {
            thief,
            settlement,
            good: GoodId::Grain,
            quantity: stolen,
            cause,
        },
        Some(cause),
    );

    spread_theft_rumour(sim, settlement, thief, stolen, cause);
    adjust_trust_after_theft(sim, thief, settlement, cause);

    ActionOutcome {
        succeeded: true,
        cause,
        summary: format!("Stole {stolen} grain from {settlement:?}"),
    }
}

fn kill_person(
    sim: &mut WorldSimulator,
    killer: Actor,
    victim: PersonId,
    cause: CauseId,
) -> ActionOutcome {
    let victim_snapshot = sim.world.npcs.iter().find(|n| n.id == victim);
    let Some(victim_npc) = victim_snapshot else {
        return ActionOutcome {
            succeeded: false,
            cause,
            summary: "Victim not found".into(),
        };
    };
    if !victim_npc.alive {
        return ActionOutcome {
            succeeded: false,
            cause,
            summary: "Already dead".into(),
        };
    }
    let victim_name = victim_npc.name.clone();
    let victim_home = victim_npc.home;
    let location = victim_npc.location().unwrap_or(victim_home);
    let heir = victim_npc
        .relationships
        .iter()
        .max_by_key(|(_, affinity)| *affinity)
        .map(|(id, _)| *id)
        .filter(|id| sim.world.npcs.iter().any(|n| n.id == *id && n.alive));

    if let Some(npc) = sim.world.npcs.iter_mut().find(|n| n.id == victim) {
        npc.alive = false;
    }

    sim.log.emit(
        sim.world.day,
        WorldEvent::PersonDied {
            person: victim,
            location,
            killer,
            cause,
        },
        Some(cause),
    );

    transfer_owned_properties(sim, victim, heir, cause);
    sync_settlement_population(&mut sim.world);
    spread_death_knowledge(sim, victim, &victim_name, location, killer, cause);

    ActionOutcome {
        succeeded: true,
        cause,
        summary: format!("{victim_name} died at {location:?}"),
    }
}

fn damage_property(
    sim: &mut WorldSimulator,
    actor: Actor,
    property: PropertyId,
    damage: u32,
    cause: CauseId,
) -> ActionOutcome {
    let (name, settlement, kind) = {
        let prop = sim.world.property(property);
        (prop.name.clone(), prop.settlement, prop.kind)
    };
    let prop_mut = sim.world.property_mut(property);
    prop_mut.damage = (prop_mut.damage + damage).min(100);
    if prop_mut.damage >= 80 {
        prop_mut.operational = false;
    }

    sim.log.emit(
        sim.world.day,
        WorldEvent::StructureDamaged {
            route: None,
            property: Some(property),
            actor,
            severity: damage,
            cause,
        },
        Some(cause),
    );

    if kind == PropertyKind::FarmEstate {
        let props = sim.world.properties.clone();
        crate::property::apply_property_effects_to_settlement(
            sim.world.settlement_mut(settlement),
            &props,
        );
    }

    let _ = mill_operational(&sim.world);
    ActionOutcome {
        succeeded: true,
        cause,
        summary: format!("{name} damaged (+{damage})"),
    }
}

fn transfer_owned_properties(
    sim: &mut WorldSimulator,
    deceased: PersonId,
    heir: Option<PersonId>,
    cause: CauseId,
) {
    let owned: Vec<PropertyId> = sim
        .world
        .properties
        .iter()
        .filter(|p| p.owner == deceased)
        .map(|p| p.id)
        .collect();

    for prop_id in owned {
        let settlement = sim.world.property(prop_id).settlement;
        let fallback = sim
            .world
            .npcs
            .iter()
            .find(|n| n.alive && n.home == settlement && n.job == Job::Guard)
            .map(|n| n.id);
        let new_owner = heir.or(fallback).unwrap_or(deceased);
        let prop = sim.world.property_mut(prop_id);
        let from = prop.owner;
        prop.owner = new_owner;
        if new_owner == deceased {
            prop.operational = false;
        }
        sim.log.emit(
            sim.world.day,
            WorldEvent::OwnershipTransferred {
                property: prop_id,
                from,
                to: new_owner,
                cause,
            },
            Some(cause),
        );
    }
}

fn spread_theft_rumour(
    sim: &mut WorldSimulator,
    settlement: SettlementId,
    thief: Actor,
    quantity: u32,
    cause: CauseId,
) {
    let day = sim.world.day;
    for npc in sim.world.npcs.iter_mut().filter(|n| n.alive) {
        if npc.job != Job::Guard && npc.location() != Some(settlement) {
            continue;
        }
        let note = match thief {
            Actor::Player => format!("guards report {quantity} grain stolen (outsider)"),
            Actor::Npc(id) => {
                if npc.id == id {
                    continue;
                }
                format!("witnessed theft of {quantity} grain near the stores")
            }
        };
        npc.remember(day, note);
    }
    sim.log.emit(
        day,
        WorldEvent::RumourSpread {
            settlement,
            topic: "theft".into(),
            cause,
        },
        Some(cause),
    );
}

fn spread_death_knowledge(
    sim: &mut WorldSimulator,
    victim: PersonId,
    name: &str,
    location: SettlementId,
    killer: Actor,
    cause: CauseId,
) {
    let day = sim.world.day;
    let killer_note = match killer {
        Actor::Player => "a stranger".to_string(),
        Actor::Npc(id) => sim
            .world
            .npcs
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "someone".into()),
    };
    for npc in sim.world.npcs.iter_mut().filter(|n| n.alive && n.id != victim) {
        if npc.location() != Some(location) && npc.home != location {
            continue;
        }
        npc.remember(day, format!("{name} was killed by {killer_note}"));
    }
    sim.log.emit(
        day,
        WorldEvent::RumourSpread {
            settlement: location,
            topic: format!("death of {name}"),
            cause,
        },
        Some(cause),
    );
}

fn adjust_trust_after_theft(
    sim: &mut WorldSimulator,
    thief: Actor,
    settlement: SettlementId,
    cause: CauseId,
) {
    if let Actor::Npc(thief_id) = thief {
        let mut first_target = None;
        for npc in sim.world.npcs.iter_mut().filter(|n| n.alive && n.home == settlement) {
            if npc.id == thief_id {
                continue;
            }
            first_target.get_or_insert(npc.id);
            let entry = npc.relationships.iter_mut().find(|(id, _)| *id == thief_id);
            if let Some((_, affinity)) = entry {
                *affinity = (*affinity - 15).max(-100);
            } else {
                npc.relationships.push((thief_id, -20));
            }
        }
        if let Some(target) = first_target {
            sim.log.emit(
                sim.world.day,
                WorldEvent::RelationshipChanged {
                    source: thief_id,
                    target,
                    change: RelationshipChange::Worsened,
                    cause,
                },
                Some(cause),
            );
        }
    }
}
