use crate::events::{EventLog, WorldEvent};
use crate::ids::SettlementId;

/// Grain flows recorded in the event log (authoritative for reporting).
#[derive(Default, Clone, Debug)]
pub struct GrainLedger {
    pub produced: u64,
    pub npc_personal_eaten: u64,
    pub market_purchased: u64,
    pub merchant_delivered: u64,
    pub thefts: u64,
    pub alt_hauls: u64,
    pub production_events: u32,
}

impl GrainLedger {
    pub fn from_event_log(log: &EventLog) -> Self {
        let mut ledger = Self::default();
        for record in &log.records {
            match &record.event {
                WorldEvent::GoodsProduced { quantity, .. } => {
                    ledger.produced += *quantity as u64;
                    ledger.production_events += 1;
                }
                WorldEvent::NpcAte { grain, .. } => ledger.npc_personal_eaten += *grain as u64,
                WorldEvent::NpcBoughtFood { grain, .. } => ledger.market_purchased += *grain as u64,
                WorldEvent::GoodsTransferred { quantity, .. } => {
                    ledger.merchant_delivered += *quantity as u64
                }
                WorldEvent::TheftCommitted { quantity, .. } => ledger.thefts += *quantity as u64,
                WorldEvent::AlternativeRouteUsed { quantity, .. } => {
                    ledger.alt_hauls += *quantity as u64
                }
                WorldEvent::GoodsConsumed { .. } => {
                    // Legacy aggregate path — must not appear in normal runs.
                }
                _ => {}
            }
        }
        ledger
    }

    pub fn total_npc_food(&self) -> u64 {
        self.npc_personal_eaten + self.market_purchased
    }
}

pub fn transfer_settlement_grain(
    world: &mut crate::world::WorldState,
    from: SettlementId,
    to: SettlementId,
    quantity: u32,
) -> u32 {
    let available = world.settlement(from).grain_inventory;
    let moved = quantity.min(available);
    if moved == 0 {
        return 0;
    }
    world.settlement_mut(from).grain_inventory -= moved;
    world.settlement_mut(to).grain_inventory += moved;
    moved
}
