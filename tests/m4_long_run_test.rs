use living_world::invariants::{validate_world, ViolationSeverity};
use living_world::ledger::{reconcile_grain_conservation, GrainLedger};
use living_world::scenarios::{ScenarioKind, BRIDGE_BLOCK_DAY};
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::world::initial_world;

const SEEDS: &[u64] = &[1, 2, 7, 42, 99];
const DAYS: u32 = 400;

fn run_long(seed: u64, block: bool) {
    let opening = initial_world();
    let mut sim = WorldSimulator::new(
        opening.clone(),
        SimulationConfig {
            seed,
            bridge_block_day: if block {
                Some(BRIDGE_BLOCK_DAY)
            } else {
                None
            },
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for day in 1..=DAYS {
        sim.advance_day();
        if day % 50 == 0 {
            let report = validate_world(&sim.world, &sim.log);
            assert!(
                !report
                    .violations
                    .iter()
                    .any(|v| v.severity == ViolationSeverity::Error),
                "seed {seed} day {day}: {:?}",
                report.violations
            );
            let ledger = GrainLedger::from_event_log(&sim.log);
            let conservation = reconcile_grain_conservation(&opening, &sim.world, &ledger);
            assert!(
                conservation.imbalance.abs() <= 5,
                "seed {seed} day {day} grain imbalance {}",
                conservation.imbalance
            );
        }
    }
}

#[test]
fn long_run_baseline_seeds() {
    for &seed in SEEDS {
        run_long(seed, false);
    }
}

#[test]
fn long_run_disruption_seeds() {
    for &seed in SEEDS {
        run_long(seed, true);
    }
}

#[test]
fn all_scenario_kinds_finish_without_integrity_errors() {
    for kind in [
        ScenarioKind::Baseline,
        ScenarioKind::Disruption,
        ScenarioKind::InterventionRepair,
        ScenarioKind::InterventionHaul,
    ] {
        let report = living_world::scenarios::run_scenario(kind, 42, None);
        assert_eq!(
            report.metrics.integrity_errors, 0,
            "{kind:?} had integrity errors"
        );
    }
}
