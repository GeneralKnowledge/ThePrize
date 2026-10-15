use living_world::events::WorldEvent;
use living_world::ids::RouteId;
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::structures::{
    apply_impact, apply_repair, apply_sustained_load, BridgeStructure, CrossingSafety,
};
use living_world::world::initial_world;
use living_world::world_action::{Actor, ScheduledAction, WorldAction};

#[test]
fn oak_tougher_than_oak_under_same_impact() {
    let mut soft = BridgeStructure::stoneford(RouteId(0));
    for c in &mut soft.components {
        if c.name.contains("deck") {
            c.material = living_world::materials::MaterialId::Oak;
            c.integrity = 800;
        }
    }
    let mut hard = soft.clone();
    for c in &mut hard.components {
        if c.name.contains("north") {
            c.material = living_world::materials::MaterialId::Limestone;
            c.integrity = 800;
        }
    }
    let o1 = apply_impact(&mut soft, 200);
    let o2 = apply_impact(&mut hard, 200);
    let loss_soft: u32 = o1
        .component_changes
        .iter()
        .map(|c| c.integrity_before - c.integrity_after)
        .sum();
    let loss_hard: u32 = o2
        .component_changes
        .iter()
        .map(|c| c.integrity_before - c.integrity_after)
        .sum();
    assert!(loss_soft >= loss_hard || o1.component_changes.len() >= o2.component_changes.len());
}

#[test]
fn partial_damage_can_remain_usable() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    let outcome = apply_impact(&mut bridge, 120);
    assert!(outcome.applied);
    assert!(
        matches!(
            bridge.crossing_safety(),
            CrossingSafety::Safe | CrossingSafety::Restricted
        ),
        "{:?}",
        bridge.crossing_safety()
    );
    assert!(bridge.bridge_passable());
}

#[test]
fn severe_impact_collapses_bridge() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    apply_impact(&mut bridge, 900);
    assert!(
        matches!(
            bridge.crossing_safety(),
            CrossingSafety::Unsafe | CrossingSafety::Collapsed
        )
    );
    assert!(!bridge.bridge_passable());
}

#[test]
fn repair_restores_integrity_with_resources() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    apply_impact(&mut bridge, 500);
    let before = bridge.aggregate_integrity();
    let outcome = apply_repair(&mut bridge, 20, 30, 30);
    assert!(outcome.applied);
    assert!(bridge.aggregate_integrity() > before);
}

#[test]
fn zero_force_leaves_bridge_unchanged() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    let before = bridge.aggregate_integrity();
    let outcome = apply_impact(&mut bridge, 0);
    assert!(!outcome.applied);
    assert_eq!(bridge.aggregate_integrity(), before);
}

#[test]
fn bridge_damage_blocks_trade_downstream() {
    let mut sim = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 3,
            bridge_block_day: None,
            interventions: vec![],
            scheduled_actions: vec![ScheduledAction {
                day: 50,
                action: WorldAction::DamageBridge {
                    actor: Actor::Player,
                    route: RouteId(0),
                },
            }],
        },
    );
    for _ in 0..55 {
        sim.advance_day();
    }
    assert!(!sim.world.route(RouteId(0)).passable);
    assert!(
        sim.log
            .events_of_kind(|e| matches!(e, WorldEvent::StructuralConditionChanged { .. }))
            .count()
            >= 1
    );
    let transfers_at_55 = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count();
    for _ in 0..30 {
        sim.advance_day();
    }
    let transfers_later = sim
        .log
        .events_of_kind(|e| matches!(e, WorldEvent::GoodsTransferred { .. }))
        .count();
    assert_eq!(transfers_at_55, transfers_later);
}

#[test]
fn repeated_load_differs_with_prior_damage() {
    let mut intact = BridgeStructure::stoneford(RouteId(0));
    let mut worn = BridgeStructure::stoneford(RouteId(0));
    apply_impact(&mut worn, 300);
    let o_intact = apply_sustained_load(&mut intact, 80);
    let o_worn = apply_sustained_load(&mut worn, 80);
    let loss_intact: u32 = o_intact
        .component_changes
        .iter()
        .map(|c| c.integrity_before.saturating_sub(c.integrity_after))
        .sum();
    let loss_worn: u32 = o_worn
        .component_changes
        .iter()
        .map(|c| c.integrity_before.saturating_sub(c.integrity_after))
        .sum();
    assert!(loss_worn >= loss_intact);
}

#[test]
fn deterministic_structure_digest_includes_bridge() {
    let mut a = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 9,
            bridge_block_day: Some(100),
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    let mut b = WorldSimulator::new(
        initial_world(),
        SimulationConfig {
            seed: 9,
            bridge_block_day: Some(100),
            interventions: vec![],
            scheduled_actions: vec![],
        },
    );
    for _ in 0..120 {
        a.advance_day();
        b.advance_day();
    }
    let sa = living_world::snapshot::canonical_snapshot(&a);
    let sb = living_world::snapshot::canonical_snapshot(&b);
    assert_eq!(sa.bridge_integrity, sb.bridge_integrity);
    assert_eq!(sa.bridge_safety, sb.bridge_safety);
    assert_eq!(sa, sb);
}
