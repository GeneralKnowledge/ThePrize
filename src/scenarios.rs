use crate::economy::snapshot_settlement;
use crate::events::{InterventionKind, WorldEvent};
use crate::history::{ancestors_of, canonical_history_digest, descendants_of, find_cause_for_event};
use crate::ledger::GrainLedger;
use crate::persistence::{persist_run, RunPersistence};
use crate::simulation::{ScheduledIntervention, SimulationConfig, WorldSimulator};
use crate::world::initial_world;
use serde::Serialize;
use std::path::Path;

pub const SIM_DAYS: u32 = 365;
pub const BRIDGE_BLOCK_DAY: u32 = 182;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ScenarioKind {
    Baseline,
    Disruption,
    InterventionRepair,
    InterventionHaul,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioMetrics {
    pub merchant_transfers: u32,
    pub npc_travel_events: u32,
    pub alternative_hauls: u32,
    pub starved_events: u32,
    pub route_blocked_days: u32,
    pub bridge_repaired: bool,
    pub min_village_grain: u32,
    pub min_town_grain: u32,
    pub final_village_grain: u32,
    pub final_town_grain: u32,
    pub total_grain_produced: u64,
    pub total_grain_consumed: u64,
    pub merchant_departures: u32,
    pub merchant_arrivals: u32,
    pub merchant_sales: u32,
    pub merchant_trip_cancellations: u32,
    pub history_digest: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioReport {
    pub kind: ScenarioKind,
    pub seed: u64,
    pub metrics: ScenarioMetrics,
    pub causal_summary: Vec<String>,
    pub sqlite_path: Option<String>,
}

pub fn run_scenario(kind: ScenarioKind, seed: u64, output_dir: Option<&Path>) -> ScenarioReport {
    let config = config_for(kind, seed);
    let mut sim = WorldSimulator::new(initial_world(), config);

    let mut min_village_grain = u32::MAX;
    let mut min_town_grain = u32::MAX;

    for _ in 0..SIM_DAYS {
        sim.advance_day();
        let v = sim.world.settlement(crate::ids::SettlementId(0)).grain_inventory;
        let t = sim.world.settlement(crate::ids::SettlementId(1)).grain_inventory;
        min_village_grain = min_village_grain.min(v);
        min_town_grain = min_town_grain.min(t);
    }

    let metrics = compute_metrics(&sim, min_village_grain, min_town_grain);
    let causal_summary = summarize_causal_chain(&sim);

    let sqlite_path = output_dir.map(|dir| {
        let path = dir.join(format!("{:?}_{}.sqlite", kind, seed));
        let persistence = RunPersistence {
            scenario: kind,
            seed,
            world: &sim.world,
            log: &sim.log,
        };
        persist_run(&path, &persistence).expect("persist run");
        path.display().to_string()
    });

    ScenarioReport {
        kind,
        seed,
        metrics,
        causal_summary,
        sqlite_path,
    }
}

fn config_for(kind: ScenarioKind, seed: u64) -> SimulationConfig {
    match kind {
        ScenarioKind::Baseline => SimulationConfig {
            seed,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
        ScenarioKind::Disruption => SimulationConfig {
            seed,
            bridge_block_day: Some(BRIDGE_BLOCK_DAY),
            interventions: vec![],
            scheduled_actions: vec![],
        },
        ScenarioKind::InterventionRepair => SimulationConfig {
            seed,
            bridge_block_day: Some(BRIDGE_BLOCK_DAY),
            interventions: vec![ScheduledIntervention {
                day: BRIDGE_BLOCK_DAY + 14,
                kind: InterventionKind::RepairBridge,
            }],
            scheduled_actions: vec![],
        },
        ScenarioKind::InterventionHaul => SimulationConfig {
            seed,
            bridge_block_day: Some(BRIDGE_BLOCK_DAY),
            interventions: weekly_hauls(BRIDGE_BLOCK_DAY + 7, BRIDGE_BLOCK_DAY + 120),
            scheduled_actions: vec![],
        },
    }
}

fn weekly_hauls(start: u32, end: u32) -> Vec<ScheduledIntervention> {
    let mut v = vec![];
    let mut day = start;
    while day <= end {
        v.push(ScheduledIntervention {
            day,
            kind: InterventionKind::HaulGrainAlternative,
        });
        day += 7;
    }
    v
}

fn compute_metrics(sim: &WorldSimulator, min_village: u32, min_town: u32) -> ScenarioMetrics {
    let merchant_transfers = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count() as u32;
    let npc_travel_events = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::NpcTravelStarted { .. }))
        .count() as u32;
    let alternative_hauls = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::AlternativeRouteUsed { .. }))
        .count() as u32;
    let starved_events = sim
        .log
        .events_of_kind(|e| {
            matches!(
                e,
                WorldEvent::PopulationStarved { .. } | WorldEvent::NpcStarving { .. }
            )
        })
        .count() as u32;

    let mut route_blocked_days = 0u32;
    let mut bridge_repaired = false;
    for record in &sim.log.records {
        match &record.event {
            WorldEvent::RouteBlocked { .. } => route_blocked_days += 1,
            WorldEvent::RouteRepaired { .. } => bridge_repaired = true,
            _ => {}
        }
    }
    // Approximate blocked span: count days after block until repair or end
    let block_day = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::RouteBlocked { .. }))
        .next()
        .map(|r| r.day);
    let repair_day = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::RouteRepaired { .. }))
        .next()
        .map(|r| r.day);
    route_blocked_days = match (block_day, repair_day) {
        (Some(b), Some(r)) => r.saturating_sub(b),
        (Some(b), None) => SIM_DAYS.saturating_sub(b),
        _ => 0,
    };

    let mut produced = 0u64;
    let mut consumed = 0u64;
    for record in &sim.log.records {
        match &record.event {
            WorldEvent::GoodsProduced { quantity, .. } => produced += *quantity as u64,
            WorldEvent::GoodsConsumed { quantity, .. } => consumed += *quantity as u64,
            WorldEvent::NpcAte { grain, .. } => consumed += *grain as u64,
            WorldEvent::NpcBoughtFood { grain, .. } => consumed += *grain as u64,
            _ => {}
        }
    }

    let merchant_departures = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantDeparted { .. }))
        .count() as u32;
    let merchant_arrivals = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantArrived { .. }))
        .count() as u32;
    let merchant_sales = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantSaleCompleted { .. }))
        .count() as u32;
    let merchant_trip_cancellations = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantTripCancelled { .. }))
        .count() as u32;

    ScenarioMetrics {
        merchant_transfers,
        npc_travel_events,
        alternative_hauls,
        starved_events,
        route_blocked_days,
        bridge_repaired,
        min_village_grain: min_village,
        min_town_grain: min_town,
        final_village_grain: sim.world.settlement(crate::ids::SettlementId(0)).grain_inventory,
        final_town_grain: sim.world.settlement(crate::ids::SettlementId(1)).grain_inventory,
        total_grain_produced: produced,
        total_grain_consumed: consumed,
        merchant_departures,
        merchant_arrivals,
        merchant_sales,
        merchant_trip_cancellations,
        history_digest: canonical_history_digest(&sim.log),
    }
}

fn summarize_causal_chain(sim: &WorldSimulator) -> Vec<String> {
    let mut lines = vec![];
    let ledger = GrainLedger::from_event_log(&sim.log);
    lines.push(format!(
        "Grain ledger: produced {} | npc food {} | merchant delivered {} | alt hauls {} | thefts {}",
        ledger.produced,
        ledger.total_npc_food(),
        ledger.merchant_delivered,
        ledger.alt_hauls,
        ledger.thefts
    ));

    if let Some(block_cause) =
        find_cause_for_event(&sim.log, |e| matches!(e, WorldEvent::RouteBlocked { .. }))
    {
        if let Some(block) = sim
            .log
            .events_of_kind(|e| matches!(e, WorldEvent::RouteBlocked { .. }))
            .next()
        {
            lines.push(format!(
                "Day {}: bridge blocked (cause {:?})",
                block.day, block.cause
            ));
        }
        let effects = descendants_of(&sim.log, block_cause);
        let notable: Vec<_> = effects
            .iter()
            .filter(|l| {
                l.description.contains("Merchant")
                    || l.description.contains("RouteTransit")
                    || l.description.contains("PriceAdjusted")
                    || l.description.contains("NpcTravel")
            })
            .take(8)
            .map(|l| format!("  day {}: {}", l.day, l.description))
            .collect();
        if !notable.is_empty() {
            lines.push("Downstream of block (sample):".into());
            lines.extend(notable);
        }
        if let Some(transfer) = sim
            .log
            .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
            .find(|r| r.day > BRIDGE_BLOCK_DAY)
        {
            if let Some(anc) = transfer.parent_cause {
                let chain = ancestors_of(&sim.log, anc);
                if chain.len() > 1 {
                    lines.push(format!(
                        "Sample delivery day {} linked through {} causal steps",
                        transfer.day,
                        chain.len()
                    ));
                }
            }
        }
    }

    let departures = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantDeparted { .. }))
        .count();
    let sales = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::MerchantSaleCompleted { .. }))
        .count();
    lines.push(format!(
        "Merchant: {} departures, {} completed sales, {} trip cancellations",
        departures,
        sales,
        sim.log
            .events_of_kind(|e| matches!(e, WorldEvent::MerchantTripCancelled { .. }))
            .count()
    ));

    let hauls = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::AlternativeRouteUsed { .. }))
        .count();
    if hauls > 0 {
        lines.push(format!("Alternative grain hauls (player): {}", hauls));
    }
    if sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::RouteRepaired { .. }))
        .next()
        .is_some()
    {
        lines.push("Bridge was repaired before simulation end.".into());
    }
    let town = snapshot_settlement(sim.world.settlement(crate::ids::SettlementId(1)));
    let village = snapshot_settlement(sim.world.settlement(crate::ids::SettlementId(0)));
    lines.push(format!(
        "End state — Hollowbrook: {} grain @ {:.2}; Millford: {} grain @ {:.2}",
        village.grain, village.price, town.grain, town.price
    ));
    lines
}

#[derive(Debug)]
pub struct ScenarioComparison {
    pub baseline: ScenarioReport,
    pub disruption: ScenarioReport,
    pub intervention_repair: ScenarioReport,
    pub intervention_haul: ScenarioReport,
    pub disruption_differs_from_baseline: bool,
    pub interventions_differ_from_disruption: bool,
}

pub fn run_all_scenarios(seed: u64, output_dir: Option<&Path>) -> ScenarioComparison {
    let baseline = run_scenario(ScenarioKind::Baseline, seed, output_dir);
    let disruption = run_scenario(ScenarioKind::Disruption, seed, output_dir);
    let intervention_repair = run_scenario(ScenarioKind::InterventionRepair, seed, output_dir);
    let intervention_haul = run_scenario(ScenarioKind::InterventionHaul, seed, output_dir);

    let disruption_differs = metrics_differ(&baseline.metrics, &disruption.metrics);
    let interventions_differ = metrics_differ(&disruption.metrics, &intervention_repair.metrics)
        || metrics_differ(&disruption.metrics, &intervention_haul.metrics);

    ScenarioComparison {
        baseline,
        disruption,
        intervention_repair,
        intervention_haul,
        disruption_differs_from_baseline: disruption_differs,
        interventions_differ_from_disruption: interventions_differ,
    }
}

fn metrics_differ(a: &ScenarioMetrics, b: &ScenarioMetrics) -> bool {
    a.merchant_transfers != b.merchant_transfers
        || a.npc_travel_events != b.npc_travel_events
        || a.starved_events != b.starved_events
        || a.final_town_grain != b.final_town_grain
        || a.final_village_grain != b.final_village_grain
        || a.min_town_grain != b.min_town_grain
        || a.alternative_hauls != b.alternative_hauls
        || a.bridge_repaired != b.bridge_repaired
        || a.route_blocked_days != b.route_blocked_days
}
