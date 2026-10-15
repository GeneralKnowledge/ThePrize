use crate::ids::{CauseId, EventId, EventRecordId, PersonId, RouteId, SettlementId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum GoodId {
    Grain,
    Coin,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum RelationshipChange {
    Improved,
    Worsened,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum WorldEvent {
    DayAdvanced {
        day: u32,
        season: Season,
    },
    GoodsProduced {
        settlement: SettlementId,
        good: GoodId,
        quantity: u32,
    },
    GoodsConsumed {
        settlement: SettlementId,
        good: GoodId,
        quantity: u32,
    },
    GoodsTransferred {
        from: SettlementId,
        to: SettlementId,
        good: GoodId,
        quantity: u32,
        via_merchant: PersonId,
    },
    RouteBlocked {
        route: RouteId,
        cause: CauseId,
    },
    RouteRepaired {
        route: RouteId,
        cause: CauseId,
    },
    AlternativeRouteUsed {
        from: SettlementId,
        to: SettlementId,
        good: GoodId,
        quantity: u32,
        cause: CauseId,
    },
    BridgeRepairStarted {
        route: RouteId,
        cause: CauseId,
    },
    PriceAdjusted {
        settlement: SettlementId,
        good: GoodId,
        old_price: f64,
        new_price: f64,
    },
    PopulationFed {
        settlement: SettlementId,
        people: u32,
        grain_used: u32,
    },
    PopulationStarved {
        settlement: SettlementId,
        people: u32,
    },
    ScheduledDisruption {
        route: RouteId,
        day: u32,
    },
    PlayerIntervention {
        kind: InterventionKind,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum InterventionKind {
    RepairBridge,
    HaulGrainAlternative,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn from_day(day: u32) -> Self {
        let d = day % 365;
        if d < 91 {
            Season::Spring
        } else if d < 182 {
            Season::Summer
        } else if d < 273 {
            Season::Autumn
        } else {
            Season::Winter
        }
    }

    /// Multiplier on daily farm output (weather proxy for prototype).
    pub fn farm_yield_multiplier(self) -> f64 {
        match self {
            Season::Spring => 0.6,
            Season::Summer => 1.0,
            Season::Autumn => 1.2,
            Season::Winter => 0.15,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EventRecord {
    pub id: EventRecordId,
    pub day: u32,
    pub event: WorldEvent,
    pub parent_cause: Option<CauseId>,
    pub cause: CauseId,
}

#[derive(Default)]
pub struct EventLog {
    next_record: u32,
    next_cause: u32,
    pub records: Vec<EventRecord>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mint_cause(&mut self) -> CauseId {
        let id = CauseId(self.next_cause);
        self.next_cause += 1;
        id
    }

    pub fn emit(
        &mut self,
        day: u32,
        event: WorldEvent,
        parent_cause: Option<CauseId>,
    ) -> (EventRecordId, CauseId) {
        let cause = self.mint_cause();
        let id = EventRecordId(self.next_record);
        self.next_record += 1;
        self.records.push(EventRecord {
            id,
            day,
            event,
            parent_cause,
            cause,
        });
        (id, cause)
    }

    pub fn events_of_kind<'a>(
        &'a self,
        predicate: impl Fn(&WorldEvent) -> bool,
    ) -> impl Iterator<Item = &'a EventRecord> {
        self.records.iter().filter(move |r| predicate(&r.event))
    }
}

/// Stable identifier for scripted scenario hooks (not stored in WorldEvent).
#[derive(Clone, Copy, Debug)]
pub struct ScenarioEventMarker(pub EventId);
