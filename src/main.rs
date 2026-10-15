use living_world::scenarios::{run_all_scenarios, ScenarioKind, SIM_DAYS};
use std::env;
use std::path::PathBuf;

fn main() {
    let mut seed: u64 = 42;
    let mut output = PathBuf::from("runs");
    let mut scenario: Option<ScenarioKind> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                seed = args
                    .next()
                    .expect("--seed value")
                    .parse()
                    .expect("seed integer");
            }
            "--output" => {
                output = PathBuf::from(args.next().expect("--output path"));
            }
            "--scenario" => {
                scenario = Some(parse_scenario(&args.next().expect("--scenario name")));
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => panic!("unknown argument: {other} (try --help)"),
        }
    }

    std::fs::create_dir_all(&output).expect("create output dir");

    if let Some(kind) = scenario {
        let report = living_world::scenarios::run_scenario(kind, seed, Some(&output));
        print_report(&report);
        return;
    }

    let comparison = run_all_scenarios(seed, Some(&output));
    println!(
        "=== The Living World — valley experiment ({} days) ===\n",
        SIM_DAYS
    );
    print_report(&comparison.baseline);
    print_report(&comparison.disruption);
    print_report(&comparison.intervention_repair);
    print_report(&comparison.intervention_haul);

    println!("=== Acceptance checks ===");
    println!(
        "Disruption differs from baseline: {}",
        comparison.disruption_differs_from_baseline
    );
    println!(
        "Interventions differ from disruption: {}",
        comparison.interventions_differ_from_disruption
    );

    if !comparison.disruption_differs_from_baseline {
        eprintln!("WARNING: disruption scenario matched baseline — check bridge/trade coupling.");
    }
    if !comparison.interventions_differ_from_disruption {
        eprintln!("WARNING: interventions did not change outcomes vs disruption.");
    }
}

fn print_help() {
    println!(
        "living-world [--seed N] [--output DIR] [--scenario baseline|disruption|repair|haul]"
    );
}

fn parse_scenario(name: &str) -> ScenarioKind {
    match name.to_lowercase().as_str() {
        "baseline" => ScenarioKind::Baseline,
        "disruption" => ScenarioKind::Disruption,
        "intervention-repair" | "repair" => ScenarioKind::InterventionRepair,
        "intervention-haul" | "haul" => ScenarioKind::InterventionHaul,
        other => panic!("unknown scenario: {other}"),
    }
}

fn print_report(report: &living_world::scenarios::ScenarioReport) {
    println!("--- {:?} (seed {}) ---", report.kind, report.seed);
    let m = &report.metrics;
    println!(
        "  grain: village {} (min {}), town {} (min {})",
        m.final_village_grain, m.min_village_grain, m.final_town_grain, m.min_town_grain
    );
    println!(
        "  merchant: {} departures, {} sales, {} transfers, {} cancellations",
        m.merchant_departures,
        m.merchant_sales,
        m.merchant_transfers,
        m.merchant_trip_cancellations
    );
    println!(
        "  alt hauls: {}, starved event days: {}, history digest: {:x}",
        m.alternative_hauls, m.starved_events, m.history_digest
    );
    println!(
        "  bridge blocked span: {} days, repaired: {}",
        m.route_blocked_days, m.bridge_repaired
    );
    println!(
        "  produced {} / consumed {} grain over year",
        m.total_grain_produced, m.total_grain_consumed
    );
    for line in &report.causal_summary {
        println!("  · {}", line);
    }
    if let Some(path) = &report.sqlite_path {
        println!("  sqlite: {}", path);
    }
    println!();
}
