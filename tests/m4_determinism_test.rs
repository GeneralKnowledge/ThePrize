use living_world::scenarios::{run_scenario, ScenarioKind, BRIDGE_BLOCK_DAY, SIM_DAYS};
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::snapshot::{assert_snapshots_equal, canonical_snapshot};
use living_world::world::initial_world;

#[test]
fn same_seed_same_snapshot_at_checkpoints() {
    let run = |seed: u64| {
        let mut sim = WorldSimulator::new(
            initial_world(),
            SimulationConfig {
                seed,
                bridge_block_day: Some(BRIDGE_BLOCK_DAY),
                interventions: vec![],
                scheduled_actions: vec![],
            },
        );
        let mut snaps = vec![];
        for day in 1..=SIM_DAYS {
            sim.advance_day();
            if day % 50 == 0 || day == SIM_DAYS {
                snaps.push((day, canonical_snapshot(&sim)));
            }
        }
        snaps
    };
    let a = run(42);
    let b = run(42);
    assert_eq!(a.len(), b.len());
    for ((day_a, snap_a), (_, snap_b)) in a.iter().zip(b.iter()) {
        assert_snapshots_equal(snap_a, snap_b);
        let _ = day_a;
    }
}

#[test]
fn scenario_report_digest_stable() {
    let r1 = run_scenario(ScenarioKind::Disruption, 7, None);
    let r2 = run_scenario(ScenarioKind::Disruption, 7, None);
    assert_eq!(r1.metrics.snapshot_digest, r2.metrics.snapshot_digest);
    assert_eq!(r1.metrics.history_digest, r2.metrics.history_digest);
}
