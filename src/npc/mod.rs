pub mod decisions;
pub mod population;
pub mod types;

use crate::ids::{PersonId, SettlementId};
pub use decisions::inform_bridge_blocked;
pub use types::{DecisionReason, NpcAction, RecordedDecision};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Job {
    Farmer,
    Miller,
    Baker,
    Guard,
    Labourer,
    Carter,
}

impl Job {
    pub fn daily_wage(self) -> u32 {
        match self {
            Job::Farmer => 2,
            Job::Miller => 4,
            Job::Baker => 3,
            Job::Guard => 3,
            Job::Labourer => 2,
            Job::Carter => 3,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct NpcGoals {
    /// Coin still owed on a personal debt.
    pub debt_remaining: u32,
    pub save_coin_target: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MemoryEntry {
    pub day: u32,
    pub note: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum NpcPresence {
    At(SettlementId),
    Travelling {
        to: SettlementId,
        days_remaining: u32,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Npc {
    pub id: PersonId,
    pub name: String,
    pub home: SettlementId,
    pub presence: NpcPresence,
    pub job: Job,
    pub coin: u32,
    pub grain: u32,
    /// 0 = fully fed, 1 = starving.
    pub hunger: f32,
    pub goals: NpcGoals,
    /// PersonId -> affinity (-100..100)
    pub relationships: Vec<(PersonId, i8)>,
    /// Subjective beliefs (limited knowledge).
    pub beliefs: Vec<String>,
    pub memories: Vec<MemoryEntry>,
    pub worked_today: bool,
    pub last_decision: Option<RecordedDecision>,
}

impl Npc {
    pub fn location(&self) -> Option<SettlementId> {
        match self.presence {
            NpcPresence::At(s) => Some(s),
            NpcPresence::Travelling { .. } => None,
        }
    }

    pub fn knows_bridge_unsafe(&self) -> bool {
        self.beliefs.iter().any(|b| b == "bridge_unsafe")
    }

    pub fn remember(&mut self, day: u32, note: impl Into<String>) {
        self.memories.push(MemoryEntry {
            day,
            note: note.into(),
        });
        if self.memories.len() > 12 {
            self.memories.remove(0);
        }
    }
}

pub fn sync_settlement_population(world: &mut crate::world::WorldState) {
    for settlement in &mut world.settlements {
        let count = world
            .npcs
            .iter()
            .filter(|n| n.home == settlement.id)
            .count() as u32;
        settlement.population = count.max(1);
    }
}
