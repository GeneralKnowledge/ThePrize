use crate::events::EventLog;
use crate::history::canonical_history_digest;
use crate::simulation::WorldSimulator;
use crate::structures::ComponentId;
use crate::world::{MerchantLocation, WorldState};
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NpcSnapshot {
    pub id: u32,
    pub alive: bool,
    pub home: u32,
    pub presence: String,
    pub coin: u32,
    pub grain: u32,
    pub hunger_bits: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SettlementSnapshot {
    pub id: u32,
    pub population: u32,
    pub grain: u32,
    pub treasury: u32,
    pub price_bits: u64,
    pub construction_timber: u32,
    pub construction_stone: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MerchantSnapshot {
    pub location: String,
    pub coin: u32,
    pub in_transit_grain: u32,
    pub active_trip_cause: Option<u32>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RouteSnapshot {
    pub id: u32,
    pub passable: bool,
    pub repair_in_progress: bool,
    pub repair_progress: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StructuralComponentSnapshot {
    pub component_id: u8,
    pub material: u8,
    pub role: u8,
    pub integrity: u32,
    pub accumulated_stress: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CanonicalSnapshot {
    pub day: u32,
    pub settlements: Vec<SettlementSnapshot>,
    pub npcs: Vec<NpcSnapshot>,
    pub merchant: MerchantSnapshot,
    pub routes: Vec<RouteSnapshot>,
    pub history_digest: u64,
    pub event_count: u32,
    pub bridge_integrity: u32,
    pub bridge_safety: String,
    pub bridge_components: Vec<StructuralComponentSnapshot>,
}

pub fn bridge_component_snapshots(world: &WorldState) -> Vec<StructuralComponentSnapshot> {
    let mut out: Vec<_> = world
        .bridge_structure
        .components
        .iter()
        .map(|c| StructuralComponentSnapshot {
            component_id: component_id_discriminant(c.id),
            material: material_discriminant(c.material),
            role: role_discriminant(c.role),
            integrity: c.integrity,
            accumulated_stress: c.accumulated_stress,
        })
        .collect();
    out.sort_by_key(|c| c.component_id);
    out
}

fn component_id_discriminant(id: ComponentId) -> u8 {
    match id {
        ComponentId::Deck => 0,
        ComponentId::NorthSupport => 1,
        ComponentId::SouthSupport => 2,
        ComponentId::TrussEast => 3,
    }
}

fn material_discriminant(m: crate::materials::MaterialId) -> u8 {
    match m {
        crate::materials::MaterialId::Oak => 0,
        crate::materials::MaterialId::Limestone => 1,
        crate::materials::MaterialId::Iron => 2,
    }
}

fn role_discriminant(r: crate::structures::ComponentRole) -> u8 {
    match r {
        crate::structures::ComponentRole::Deck => 0,
        crate::structures::ComponentRole::Support => 1,
        crate::structures::ComponentRole::Connection => 2,
    }
}

pub fn canonical_snapshot(sim: &WorldSimulator) -> CanonicalSnapshot {
    canonical_snapshot_from(&sim.world, &sim.log)
}

pub fn canonical_snapshot_from(world: &WorldState, log: &EventLog) -> CanonicalSnapshot {
    let mut settlements: Vec<_> = world
        .settlements
        .iter()
        .map(|s| SettlementSnapshot {
            id: s.id.0,
            population: s.population,
            grain: s.grain_inventory,
            treasury: s.treasury_coin,
            price_bits: s.market.grain_price.to_bits(),
            construction_timber: s.construction_timber,
            construction_stone: s.construction_stone,
        })
        .collect();
    settlements.sort_by_key(|s| s.id);

    let mut npcs: Vec<_> = world
        .npcs
        .iter()
        .map(|n| NpcSnapshot {
            id: n.id.0,
            alive: n.alive,
            home: n.home.0,
            presence: format!("{:?}", n.presence),
            coin: n.coin,
            grain: n.grain,
            hunger_bits: n.hunger.to_bits(),
        })
        .collect();
    npcs.sort_by_key(|n| n.id);

    let mut routes: Vec<_> = world
        .routes
        .iter()
        .map(|r| RouteSnapshot {
            id: r.id.0,
            passable: r.passable,
            repair_in_progress: r.repair_in_progress,
            repair_progress: r.repair_progress,
        })
        .collect();
    routes.sort_by_key(|r| r.id);

    let (location, in_transit_grain) = match world.merchant.location {
        MerchantLocation::At(s) => (format!("At({})", s.0), 0),
        MerchantLocation::Travelling {
            from,
            to,
            days_remaining,
            carrying_grain,
        } => (
            format!(
                "Travelling({from:?}->{to:?},d={days_remaining})",
                from = from.0,
                to = to.0
            ),
            carrying_grain,
        ),
    };

    CanonicalSnapshot {
        day: world.day,
        settlements,
        npcs,
        merchant: MerchantSnapshot {
            location,
            coin: world.merchant.coin,
            in_transit_grain,
            active_trip_cause: world.merchant.active_trip_cause.map(|c| c.0),
        },
        routes,
        history_digest: canonical_history_digest(log),
        event_count: log.records.len() as u32,
        bridge_integrity: world.bridge_structure.aggregate_integrity(),
        bridge_safety: format!("{:?}", world.bridge_structure.crossing_safety()),
        bridge_components: bridge_component_snapshots(world),
    }
}

pub fn snapshot_digest(snapshot: &CanonicalSnapshot) -> u64 {
    let json = serde_json::to_string(snapshot).expect("snapshot json");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    hasher.finish()
}

pub fn assert_snapshots_equal(a: &CanonicalSnapshot, b: &CanonicalSnapshot) {
    if a != b {
        let diff = serde_json::to_string_pretty(a).unwrap();
        let diff_b = serde_json::to_string_pretty(b).unwrap();
        panic!(
            "canonical snapshots diverged\nA:\n{diff}\nB:\n{diff_b}"
        );
    }
}
