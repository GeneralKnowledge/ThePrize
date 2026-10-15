use crate::events::{EventLog, GoodId, WorldEvent};
use crate::ids::{CauseId, PersonId, RouteId, SettlementId};
use crate::world::{MerchantLocation, WorldState};

const VILLAGE: SettlementId = SettlementId(0);
const TOWN: SettlementId = SettlementId(1);
const BRIDGE: RouteId = RouteId(0);

pub fn on_route_blocked(world: &mut WorldState, log: &mut EventLog, cause: CauseId) {
    let MerchantLocation::Travelling {
        from,
        to,
        days_remaining,
        carrying_grain,
    } = world.merchant.location
    else {
        return;
    };
    if days_remaining <= 1 && !world.route(BRIDGE).passable {
        let ford_days = world.route(BRIDGE).alternative_travel_days.max(2);
        world.merchant.location = MerchantLocation::Travelling {
            from,
            to,
            days_remaining: ford_days,
            carrying_grain,
        };
        log.emit(
            world.day,
            WorldEvent::RouteTransitAdjusted {
                route: BRIDGE,
                new_days: ford_days,
                cause,
            },
            Some(cause),
        );
    }
}

pub fn tick_merchant(world: &mut WorldState, log: &mut EventLog, day: u32) {
    let merchant_id = world.merchant.id;

    if let MerchantLocation::Travelling {
        from,
        to,
        days_remaining,
        carrying_grain,
    } = world.merchant.location
    {
        if days_remaining > 1 {
            world.merchant.location = MerchantLocation::Travelling {
                from,
                to,
                days_remaining: days_remaining - 1,
                carrying_grain,
            };
            return;
        }
        if days_remaining == 1 {
            arrive(
                world,
                log,
                day,
                merchant_id,
                from,
                to,
                carrying_grain,
            );
            return;
        }
    }

    let passable = world.bridge_between(VILLAGE, TOWN).expect("bridge").passable;
    let current = match world.merchant.location {
        MerchantLocation::At(s) => s,
        MerchantLocation::Travelling { .. } => return,
    };

    let town_stock = world.settlement(TOWN).grain_inventory;
    let town_desired = world.settlement(TOWN).market.desired_grain_stock;
    let village_stock = world.settlement(VILLAGE).grain_inventory;
    let village_reserve = world.settlement(VILLAGE).market.desired_grain_stock;

    let town_needs_imports = town_stock + 8 < town_desired;
    let village_can_export = village_stock > village_reserve + 20;

    if current == VILLAGE {
        if !town_needs_imports || !village_can_export || !passable {
            if town_needs_imports && village_can_export && !passable {
                let block_cause = log
                    .records
                    .iter()
                    .rev()
                    .find(|r| matches!(r.event, WorldEvent::RouteBlocked { .. }))
                    .map(|r| r.cause);
                log.emit(
                    day,
                    WorldEvent::MerchantTripCancelled {
                        reason: "bridge closed".into(),
                        cause: block_cause,
                    },
                    block_cause,
                );
            }
            return;
        }
        let town_deficit = town_desired.saturating_sub(town_stock);
        if town_deficit < 6 {
            return;
        }
        let max_export = village_stock.saturating_sub(village_reserve + 40);
        let load = world
            .merchant
            .grain_capacity
            .min(max_export)
            .min(town_deficit + 4);
        if load == 0 {
            return;
        }
        let buy_price = world.settlement(VILLAGE).market.grain_price;
        let cost = (load as f64 * buy_price).ceil() as u32;
        if world.merchant.coin < cost {
            return;
        }
        world.merchant.coin -= cost;
        world.settlement_mut(VILLAGE).grain_inventory -= load;
        let travel_days = if passable { 1 } else { world.route(BRIDGE).alternative_travel_days };
        world.merchant.location = MerchantLocation::Travelling {
            from: VILLAGE,
            to: TOWN,
            days_remaining: travel_days,
            carrying_grain: load,
        };
        log.emit(
            day,
            WorldEvent::MerchantDeparted {
                from: VILLAGE,
                to: TOWN,
                grain: load,
                via_merchant: merchant_id,
            },
            None,
        );
        return;
    }

    if current == TOWN && town_needs_imports && village_can_export && passable {
        world.merchant.location = MerchantLocation::Travelling {
            from: TOWN,
            to: VILLAGE,
            days_remaining: 1,
            carrying_grain: 0,
        };
        log.emit(
            day,
            WorldEvent::MerchantDeparted {
                from: TOWN,
                to: VILLAGE,
                grain: 0,
                via_merchant: merchant_id,
            },
            None,
        );
    }
}

fn arrive(
    world: &mut WorldState,
    log: &mut EventLog,
    day: u32,
    merchant_id: PersonId,
    from: SettlementId,
    to: SettlementId,
    carrying_grain: u32,
) {
    world.merchant.location = MerchantLocation::At(to);
    log.emit(
        day,
        WorldEvent::MerchantArrived {
            settlement: to,
            via_merchant: merchant_id,
        },
        None,
    );

    if carrying_grain == 0 {
        return;
    }

    let delivered = carrying_grain;
    world.settlement_mut(to).grain_inventory += delivered;

    let dest_price = world.settlement(to).market.grain_price;
    let sale = (delivered as f64 * dest_price * 0.95).floor() as u32;
    world.merchant.coin += sale;

    log.emit(
        day,
        WorldEvent::GoodsTransferred {
            from,
            to,
            good: GoodId::Grain,
            quantity: delivered,
            via_merchant: merchant_id,
        },
        None,
    );
    log.emit(
        day,
        WorldEvent::MerchantSaleCompleted {
            settlement: to,
            grain: delivered,
            coin: sale,
            via_merchant: merchant_id,
        },
        None,
    );
}
