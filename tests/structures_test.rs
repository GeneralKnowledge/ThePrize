use living_world::events::WorldEvent;
use living_world::ids::RouteId;
use living_world::materials::{self, MaterialId};
use living_world::simulation::{SimulationConfig, WorldSimulator};
use living_world::structures::{
    apply_impact, apply_repair, apply_sustained_load, apply_targeted_impact,
    BridgeStructure, ComponentId, ComponentRole, CrossingSafety,
};
use living_world::world::initial_world;
use living_world::world_action::{Actor, ScheduledAction, WorldAction};

fn deck_only_bridge(material: MaterialId, integrity: u32) -> BridgeStructure {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    for c in &mut bridge.components {
        if c.role == ComponentRole::Deck {
            c.material = material;
            c.integrity = integrity;
        } else {
            c.integrity = INTEGRITY_MAX;
        }
    }
    bridge
}

const INTEGRITY_MAX: u32 = living_world::structures::INTEGRITY_MAX;

#[test]
fn tougher_material_loses_less_on_same_impact() {
    let mut oak_deck = deck_only_bridge(MaterialId::Oak, 800);
    let mut iron_deck = deck_only_bridge(MaterialId::Iron, 800);
    let force = 200;
    let oak_before = oak_deck.component(ComponentId::Deck).integrity;
    let iron_before = iron_deck.component(ComponentId::Deck).integrity;
    apply_targeted_impact(&mut oak_deck, force, ComponentId::Deck);
    apply_targeted_impact(&mut iron_deck, force, ComponentId::Deck);
    let oak_loss = oak_before - oak_deck.component(ComponentId::Deck).integrity;
    let iron_loss = iron_before - iron_deck.component(ComponentId::Deck).integrity;
    assert!(
        oak_loss > iron_loss,
        "oak loss {oak_loss} should exceed iron loss {iron_loss}"
    );
}

#[test]
fn daily_traffic_load_adds_stress_at_12_units() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    let before_stress: u32 = bridge
        .components
        .iter()
        .map(|c| c.accumulated_stress)
        .sum();
    let outcome = apply_sustained_load(&mut bridge, 12);
    assert!(outcome.applied);
    let after_stress: u32 = bridge
        .components
        .iter()
        .map(|c| c.accumulated_stress)
        .sum();
    assert!(
        after_stress > before_stress,
        "expected stress accumulation from ordinary load"
    );
}

#[test]
fn weather_decay_differs_by_material() {
    let oak = materials::weather_decay(materials::material(MaterialId::Oak));
    let limestone = materials::weather_decay(materials::material(MaterialId::Limestone));
    let iron = materials::weather_decay(materials::material(MaterialId::Iron));
    assert!(oak >= limestone, "oak {oak} vs limestone {limestone}");
    assert!(oak > iron || limestone != iron, "materials should not all match");
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
fn snapshot_distinguishes_component_state() {
    let mut a = BridgeStructure::stoneford(RouteId(0));
    let b = BridgeStructure::stoneford(RouteId(0));
    apply_targeted_impact(&mut a, 150, ComponentId::Deck);
    let mut world_a = initial_world();
    let mut world_b = initial_world();
    world_a.bridge_structure = a;
    world_b.bridge_structure = b;
    let log = living_world::events::EventLog::new();
    let sa = living_world::snapshot::canonical_snapshot_from(&world_a, &log);
    let sb = living_world::snapshot::canonical_snapshot_from(&world_b, &log);
    assert_ne!(sa.bridge_components, sb.bridge_components);
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
    assert_eq!(sa.bridge_components, sb.bridge_components);
    assert_eq!(sa, sb);
}

#[test]
fn targeted_impact_hits_primary_more_than_distant_support() {
    let mut bridge = BridgeStructure::stoneford(RouteId(0));
    apply_targeted_impact(&mut bridge, 300, ComponentId::Deck);
    let deck_loss = INTEGRITY_MAX - bridge.component(ComponentId::Deck).integrity;
    let pier_loss = INTEGRITY_MAX - bridge.component(ComponentId::NorthSupport).integrity;
    assert!(deck_loss > pier_loss);
}
