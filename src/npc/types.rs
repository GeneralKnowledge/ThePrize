use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum NpcAction {
    Eat,
    BuyFood,
    Work,
    Travel,
    WaitOutTravel,
    Idle,
    StealFood,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DecisionReason {
    CriticalHunger,
    LowPersonalGrain,
    NeedIncomeForFood,
    PayDebt,
    JobDuty,
    SeekBetterWages,
    BridgeKnownUnsafe,
    BridgeBlocked,
    MarketUnreliable,
    Travelling,
    Rested,
    DesperateTheft,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RecordedDecision {
    pub day: u32,
    pub action: NpcAction,
    pub reason: DecisionReason,
    pub detail: String,
}
