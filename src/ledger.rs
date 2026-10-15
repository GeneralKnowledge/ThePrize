use crate::events::{EventLog, WorldEvent};
use crate::ids::SettlementId;
use crate::world::WorldState;
use serde::Serialize;

/// Grain flows recorded in the event log (authoritative for reporting).
#[derive(Default, Clone, Debug, Serialize)]
pub struct GrainLedger {
    pub produced: u64,
    /// Grain removed from npc personal stores and eaten.
    pub npc_personal_eaten: u64,
    /// Grain moved from settlement market stock to NPC inventory (not yet eaten).
    pub market_purchased: u64,
    /// Merchant inter-settlement deliveries (export from world boundary perspective).
    pub merchant_exported: u64,
    pub merchant_imported: u64,
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
                WorldEvent::GoodsTransferred { from, to, quantity, .. } => {
                    ledger.merchant_exported += *quantity as u64;
                    let _ = (from, to);
                }
                WorldEvent::TheftCommitted { quantity, .. } => ledger.thefts += *quantity as u64,
                WorldEvent::AlternativeRouteUsed { quantity, .. } => {
                    ledger.alt_hauls += *quantity as u64
                }
                WorldEvent::GoodsConsumed { .. } => {}
                _ => {}
            }
        }
        ledger
    }

    pub fn food_actually_consumed(&self) -> u64 {
        self.npc_personal_eaten
    }

    pub fn total_market_activity(&self) -> u64 {
        self.market_purchased
    }

    /// Purchased + eaten (purchases are not consumption until eaten).
    pub fn total_npc_food_events(&self) -> u64 {
        self.npc_personal_eaten + self.market_purchased
    }
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct MoneyLedger {
    pub wages_from_treasury: u64,
    pub npc_market_spent: u64,
    pub merchant_buy_at_source: u64,
    pub merchant_sale_proceeds: u64,
    pub abstract_treasury_inflow: u64,
}

impl MoneyLedger {
    pub fn from_event_log(log: &EventLog) -> Self {
        let mut ledger = Self::default();
        for record in &log.records {
            match &record.event {
                WorldEvent::WagePaidFromTreasury { amount, .. } => {
                    ledger.wages_from_treasury += *amount as u64;
                }
                WorldEvent::NpcBoughtFood { coin_spent, .. } => {
                    ledger.npc_market_spent += *coin_spent as u64;
                }
                WorldEvent::MerchantSaleCompleted { coin, .. } => {
                    ledger.merchant_sale_proceeds += *coin as u64;
                }
                WorldEvent::MerchantDeparted { coin_spent, .. } => {
                    ledger.merchant_buy_at_source += *coin_spent as u64;
                }
                _ => {}
            }
        }
        ledger
    }

    pub fn with_treasury_inflow(mut self, total_inflow: u64) -> Self {
        self.abstract_treasury_inflow = total_inflow;
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GrainConservationReport {
    pub opening_settlement_grain: u64,
    pub opening_npc_grain: u64,
    pub opening_in_transit: u64,
    pub closing_settlement_grain: u64,
    pub closing_npc_grain: u64,
    pub closing_in_transit: u64,
    pub produced: u64,
    pub food_consumed: u64,
    pub market_purchased: u64,
    pub merchant_net_export: u64,
    pub alt_hauls: u64,
    pub thefts: u64,
    pub imbalance: i64,
}

pub fn grain_in_transit(world: &WorldState) -> u64 {
    use crate::world::MerchantLocation;
    match world.merchant.location {
        MerchantLocation::Travelling { carrying_grain, .. } => carrying_grain as u64,
        MerchantLocation::At(_) => 0,
    }
}

pub fn total_settlement_grain(world: &WorldState) -> u64 {
    world
        .settlements
        .iter()
        .map(|s| s.grain_inventory as u64)
        .sum()
}

pub fn total_npc_grain(world: &WorldState) -> u64 {
    world
        .npcs
        .iter()
        .filter(|n| n.alive)
        .map(|n| n.grain as u64)
        .sum()
}

/// Reconcile valley-wide grain (both settlements + NPC personal + in-transit merchant cargo).
pub fn reconcile_grain_conservation(
    opening: &WorldState,
    closing: &WorldState,
    ledger: &GrainLedger,
) -> GrainConservationReport {
    let opening_total = total_settlement_grain(opening)
        + total_npc_grain(opening)
        + grain_in_transit(opening);
    let closing_total = total_settlement_grain(closing)
        + total_npc_grain(closing)
        + grain_in_transit(closing);

    let lhs = opening_total + ledger.produced;
    let rhs = closing_total + ledger.food_actually_consumed();

    GrainConservationReport {
        opening_settlement_grain: total_settlement_grain(opening),
        opening_npc_grain: total_npc_grain(opening),
        opening_in_transit: grain_in_transit(opening),
        closing_settlement_grain: total_settlement_grain(closing),
        closing_npc_grain: total_npc_grain(closing),
        closing_in_transit: grain_in_transit(closing),
        produced: ledger.produced,
        food_consumed: ledger.food_actually_consumed(),
        market_purchased: ledger.market_purchased,
        merchant_net_export: ledger.merchant_exported,
        alt_hauls: ledger.alt_hauls,
        thefts: ledger.thefts,
        imbalance: lhs as i64 - rhs as i64,
    }
}

pub fn transfer_settlement_grain(
    world: &mut WorldState,
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
