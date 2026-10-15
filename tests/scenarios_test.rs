use living_world::scenarios::{run_all_scenarios, ScenarioKind, BRIDGE_BLOCK_DAY, SIM_DAYS};

#[test]
fn settlements_remain_viable_in_baseline_year() {
    let c = run_all_scenarios(42, None);
    let m = &c.baseline.metrics;
    assert!(
        m.min_village_grain > 5 && m.final_village_grain > 5,
        "village min {} final {}",
        m.min_village_grain,
        m.final_village_grain
    );
    assert!(m.final_town_grain > 10, "town final {}", m.final_town_grain);
    assert!(
        m.merchant_transfers >= 10 || m.total_grain_consumed > 500,
        "trade or npc food purchases should move grain"
    );
    assert!(m.starved_events < 120);
    assert!(m.total_grain_produced > m.total_grain_consumed / 2);
}

#[test]
fn bridge_disruption_changes_outcomes() {
    let c = run_all_scenarios(42, None);
    assert!(
        c.disruption_differs_from_baseline,
        "baseline {:?} vs disruption {:?}",
        c.baseline.metrics, c.disruption.metrics
    );
    assert!(
        c.disruption.metrics.merchant_transfers < c.baseline.metrics.merchant_transfers
            || c.disruption.metrics.npc_travel_events < c.baseline.metrics.npc_travel_events
            || c.disruption.metrics.min_town_grain < c.baseline.metrics.min_town_grain
            || c.disruption.metrics.starved_events > c.baseline.metrics.starved_events,
        "disruption should reduce trade, travel, or town reserves"
    );
}

#[test]
fn interventions_offer_recovery_paths() {
    let c = run_all_scenarios(42, None);
    assert!(c.interventions_differ_from_disruption);
    assert!(c.intervention_repair.metrics.bridge_repaired);
    assert!(c.intervention_haul.metrics.alternative_hauls > 0);
}

#[test]
fn deterministic_replay() {
    let a = run_all_scenarios(7, None);
    let b = run_all_scenarios(7, None);
    assert_eq!(
        a.baseline.metrics.final_town_grain,
        b.baseline.metrics.final_town_grain
    );
    assert_eq!(
        a.disruption.metrics.starved_events,
        b.disruption.metrics.starved_events
    );
}

#[test]
fn simulation_runs_full_year() {
    let r = living_world::scenarios::run_scenario(ScenarioKind::Baseline, 1, None);
    assert_eq!(r.metrics.total_grain_produced > 0, true);
    let _ = (SIM_DAYS, BRIDGE_BLOCK_DAY);
}
