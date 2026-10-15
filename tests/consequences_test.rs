use living_world::consequences::apply_world_action;
use living_world::events::WorldEvent;
use living_world::history::{find_cause_for_event, summarize_bridge_collapse, trace_from_cause};
use living_world::ids::{PropertyId, RouteId, SettlementId};
use living_world::npc::Job;
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::world::initial_world;
use living_world::world_action::{Actor, ScheduledAction, WorldAction};

#[test]
fn bridge_damage_blocks_trade_with_causal_chain() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 1,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![ScheduledAction {
                day: 100,
                action: WorldAction::DamageBridge {
                    actor: Actor::Player,
                    route: RouteId(0),
                },
            }],
        },
    );
    for _ in 0..120 {
        sim.advance_day();
    }
    assert!(!sim.world.route(RouteId(0)).passable);
    let transfers_before = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count();
    for _ in 0..30 {
        sim.advance_day();
    }
    let transfers_after = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count();
    assert_eq!(transfers_after, transfers_before);
    let summary = summarize_bridge_collapse(&sim.log);
    assert!(summary.iter().any(|s| s.contains("RouteBlocked")));
}

#[test]
fn theft_reduces_settlement_grain_and_leaves_trace() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 2,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for _ in 0..10 {
        sim.advance_day();
    }
    let before = sim.world.settlement(SettlementId(0)).grain_inventory;
    apply_world_action(
        &mut sim,
        WorldAction::StealSettlementGrain {
            thief: Actor::Player,
            settlement: SettlementId(0),
            quantity: 50,
        },
        None,
    );
    let after = sim.world.settlement(SettlementId(0)).grain_inventory;
    assert!(after < before);
    let theft_cause = find_cause_for_event(&sim.log, |e| matches!(e, WorldEvent::TheftCommitted { .. }))
        .expect("theft event");
    let chain = trace_from_cause(&sim.log, theft_cause);
    assert!(chain.iter().any(|l| l.description.contains("TheftCommitted")));
}

#[test]
fn killing_miller_transfers_mill_ownership() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 4,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    let miller_id = sim
        .world
        .npcs
        .iter()
        .find(|n| n.job == Job::Miller)
        .expect("miller")
        .id;
    apply_world_action(
        &mut sim,
        WorldAction::KillPerson {
            killer: Actor::Player,
            victim: miller_id,
        },
        None,
    );
    assert!(!sim.world.npc(miller_id).alive);
    let transfer = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::OwnershipTransferred { .. }))
        .count();
    assert!(transfer >= 1);
    let mill = sim
        .world
        .properties
        .iter()
        .find(|p| p.id == PropertyId(0))
        .expect("mill");
    assert_ne!(mill.owner, miller_id);
}

#[test]
fn farm_damage_reduces_village_production() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 5,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    let cap_before = sim.world.settlement(SettlementId(0)).farm_daily_capacity;
    apply_world_action(
        &mut sim,
        WorldAction::DamageProperty {
            actor: Actor::Player,
            property: PropertyId(1),
            damage: 85,
        },
        None,
    );
    let cap_after = sim.world.settlement(SettlementId(0)).farm_daily_capacity;
    assert!(cap_after < cap_before);
}
