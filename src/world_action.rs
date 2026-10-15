use crate::ids::{PersonId, PropertyId, RouteId, SettlementId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Actor {
    Player,
    Npc(PersonId),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum WorldAction {
    DamageBridge {
        actor: Actor,
        route: RouteId,
    },
    StealSettlementGrain {
        thief: Actor,
        settlement: SettlementId,
        quantity: u32,
    },
    KillPerson {
        killer: Actor,
        victim: PersonId,
    },
    DamageProperty {
        actor: Actor,
        property: PropertyId,
        damage: u32,
    },
}

#[derive(Clone, Debug)]
pub struct ScheduledAction {
    pub day: u32,
    pub action: WorldAction,
}
