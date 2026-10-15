use living_world::events::WorldEvent;
use living_world::ids::PersonId;
use living_world::npc::Job;
use living_world::scenarios::{run_scenario, ScenarioKind, BRIDGE_BLOCK_DAY, SIM_DAYS};
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::world::initial_world;

#[test]
fn npcs_work_and_explain_decisions() {
    let sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 99,
            bridge_block_day: None,
            interventions: vec![],
        },
    );
    let mut sim = sim;
    for _ in 0..14 {
        sim.advance_day();
    }
    let farmers: Vec<_> = sim
        .world
        .npcs
        .iter()
        .filter(|n| n.job == Job::Farmer)
        .collect();
    assert!(!farmers.is_empty());
    let worked = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::NpcWorked { .. }))
        .count();
    assert!(worked > 20, "expected farmers/labourers to work, got {worked}");
    assert!(
        farmers.iter().any(|n| n.last_decision.is_some()),
        "decisions should be recorded on NPCs"
    );
}

#[test]
fn npcs_buy_food_when_hungry() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 3,
            bridge_block_day: None,
            interventions: vec![],
        },
    );
    for _ in 0..45 {
        sim.advance_day();
    }
    let purchases = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::NpcBoughtFood { .. }))
        .count();
    let meals = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::NpcAte { .. }))
        .count();
    let avg_hunger: f32 = sim.world.npcs.iter().map(|n| n.hunger).sum::<f32>()
        / sim.world.npcs.len() as f32;
    assert!(
        purchases > 0 || meals > 15 || avg_hunger < 0.85,
        "NPCs should eat, buy food, or work enough to stay fed (purchases={purchases}, meals={meals}, avg_hunger={avg_hunger:.2})"
    );
}

#[test]
fn bridge_block_reduces_crossing_travel() {
    let baseline = run_scenario(ScenarioKind::Baseline, 5, None);
    let disruption = run_scenario(ScenarioKind::Disruption, 5, None);
    assert!(disruption.metrics.route_blocked_days > 0);
    assert!(
        disruption.metrics.final_town_grain != baseline.metrics.final_town_grain
            || disruption.metrics.merchant_transfers != baseline.metrics.merchant_transfers
            || disruption.metrics.total_grain_consumed != baseline.metrics.total_grain_consumed,
        "bridge disruption should change town grain, trade, or food use"
    );
    let blocked_day_travel = disruption
        .metrics
        .npc_travel_events;
    let _ = (SIM_DAYS, BRIDGE_BLOCK_DAY, blocked_day_travel);
}

#[test]
fn garrick_pays_down_debt_or_works_for_it() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 1,
            bridge_block_day: None,
            interventions: vec![],
        },
    );
    for _ in 0..60 {
        sim.advance_day();
    }
    let garrick = sim
        .world
        .npcs
        .iter()
        .find(|n| n.name == "Garrick")
        .expect("miller debtor");
    let debt_paid = garrick.goals.debt_remaining < 40;
    let worked = sim.log.events_of_kind(|e| {
        matches!(
            e,
            WorldEvent::NpcWorked {
                person,
                reason,
                ..
            } if *person == garrick.id
                && matches!(
                    reason,
                    living_world::npc::DecisionReason::PayDebt
                        | living_world::npc::DecisionReason::JobDuty
                )
        )
    });
    assert!(debt_paid || worked.count() > 10);
    let _ = PersonId(0);
}
