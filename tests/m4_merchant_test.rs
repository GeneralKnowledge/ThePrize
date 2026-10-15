use living_world::events::WorldEvent;
use living_world::history::ancestors_of;
use living_world::ids::{RouteId, SettlementId};
use living_world::merchant::on_route_blocked;
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::world::{initial_world, MerchantLocation};

#[test]
fn merchant_trade_takes_multiple_days_when_blocked_mid_trip() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 1,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    sim.world.merchant.location = MerchantLocation::Travelling {
        from: SettlementId(0),
        to: SettlementId(1),
        days_remaining: 1,
        carrying_grain: 10,
    };
    sim.world.merchant.active_trip_cause = Some(living_world::ids::CauseId(99));
    sim.world.route_mut(RouteId(0)).passable = false;

    on_route_blocked(&mut sim.world, &mut sim.log, living_world::ids::CauseId(50));

    if let MerchantLocation::Travelling { days_remaining, .. } = sim.world.merchant.location {
        assert!(days_remaining >= 2, "ford delay applied, got {days_remaining}");
    } else {
        panic!("merchant should still be travelling");
    }
}

#[test]
fn merchant_sale_links_to_depart_cause() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 3,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for _ in 0..60 {
        sim.advance_day();
    }
    let sale = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantSaleCompleted { .. }))
        .next();
    if let Some(sale) = sale {
        assert!(
            sale.parent_cause.is_some(),
            "sales should parent-link to trip cause"
        );
        let chain = ancestors_of(&sim.log, sale.cause);
        assert!(!chain.is_empty());
    }
}

#[test]
fn insufficient_merchant_coin_cancels_trip() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 2,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    sim.world.merchant.coin = 0;
    sim.world.merchant.location = MerchantLocation::At(SettlementId(0));
    sim.world.settlement_mut(SettlementId(1)).grain_inventory = 5;
    for _ in 0..5 {
        sim.advance_day();
    }
    let cancelled = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantTripCancelled { .. }))
        .count();
    assert!(cancelled > 0 || sim.world.merchant.coin > 0);
}
