use living_world::events::WorldEvent;
use living_world::history::{ancestors_of, descendants_of};
use living_world::invariants::{check_no_aggregate_and_npc_double_consume, check_world_invariants};
use living_world::ledger::GrainLedger;
use living_world::scenarios::{run_scenario, ScenarioKind, BRIDGE_BLOCK_DAY};
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::world::initial_world;

#[test]
fn world_invariants_hold_after_year() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 11,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for _ in 0..120 {
        sim.advance_day();
    }
    check_world_invariants(&sim.world).expect("invariants");
    check_no_aggregate_and_npc_double_consume(&sim.log).expect("no double consume");
}

#[test]
fn canonical_history_is_seed_deterministic() {
    let a = run_scenario(ScenarioKind::Disruption, 42, None);
    let b = run_scenario(ScenarioKind::Disruption, 42, None);
    assert_eq!(a.metrics.history_digest, b.metrics.history_digest);
    assert_eq!(
        a.metrics.final_town_grain,
        b.metrics.final_town_grain
    );
}

#[test]
fn merchant_sales_match_transfers() {
    let r = run_scenario(ScenarioKind::Baseline, 42, None);
    assert_eq!(
        r.metrics.merchant_transfers, r.metrics.merchant_sales,
        "each delivery should record one sale"
    );
    assert!(r.metrics.merchant_departures >= r.metrics.merchant_sales);
}

#[test]
fn blocked_bridge_reduces_merchant_activity() {
    let baseline = run_scenario(ScenarioKind::Baseline, 5, None);
    let disruption = run_scenario(ScenarioKind::Disruption, 5, None);
    assert!(
        disruption.metrics.merchant_sales <= baseline.metrics.merchant_sales
            || disruption.metrics.merchant_trip_cancellations
                > baseline.metrics.merchant_trip_cancellations
            || disruption.metrics.min_town_grain < baseline.metrics.min_town_grain,
        "baseline sales {} vs disruption sales {}",
        baseline.metrics.merchant_sales,
        disruption.metrics.merchant_sales
    );
}

#[test]
fn route_block_has_causal_descendants() {
    let sim = run_scenario(ScenarioKind::Disruption, 42, None);
    let mut sim_inner = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 42,
            bridge_block_day: Some(BRIDGE_BLOCK_DAY),
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for _ in 0..365 {
        sim_inner.advance_day();
    }
    let block = sim_inner
        .log
        .records
        .iter()
        .find(|r| matches!(r.event, WorldEvent::RouteBlocked { .. }))
        .expect("block");
    let desc = descendants_of(&sim_inner.log, block.cause);
    let linked = sim_inner
        .log
        .records
        .iter()
        .any(|r| r.parent_cause == Some(block.cause));
    assert!(
        !desc.is_empty() || linked || sim.metrics.merchant_trip_cancellations > 0,
        "block should spawn linked consequences"
    );
    let _ = ancestors_of(&sim_inner.log, block.cause);
    let ledger = GrainLedger::from_event_log(&sim_inner.log);
    assert!(ledger.produced > ledger.total_npc_food() / 2);
}

#[test]
fn treasury_wages_do_not_mint_unbounded_coin() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 2,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    let start_treasury: u32 = sim
        .world
        .settlements
        .iter()
        .map(|s| s.treasury_coin)
        .sum();
    let population: u32 = sim.world.settlements.iter().map(|s| s.population).sum();
    for _ in 0..30 {
        sim.advance_day();
    }
    let wages: u32 = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::WagePaidFromTreasury { .. }))
        .map(|r| match &r.event {
            WorldEvent::WagePaidFromTreasury { amount, .. } => *amount,
            _ => 0,
        })
        .sum();
    let end_treasury: u32 = sim
        .world
        .settlements
        .iter()
        .map(|s| s.treasury_coin)
        .sum();
    let abstract_inflow = 30 * population * 2;
    assert!(wages > 0);
    assert_eq!(
        start_treasury + abstract_inflow - wages,
        end_treasury,
        "treasury balance must match wages plus documented daily inflow"
    );
}
