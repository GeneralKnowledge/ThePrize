use crate::ids::RouteId;
use crate::interactions::{
    resolve_bridge, BridgeInteractionInput, ImpactInput, LoadInput, RepairInput,
    StructureInteractionKind,
};
use crate::materials::MaterialId;
use serde::{Deserialize, Serialize};

pub const INTEGRITY_MAX: u32 = 1000;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ComponentRole {
    Deck,
    Support,
    Connection,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum ComponentId {
    Deck = 0,
    NorthSupport = 1,
    SouthSupport = 2,
    TrussEast = 3,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StructuralComponent {
    pub id: ComponentId,
    pub name: String,
    pub material: MaterialId,
    pub role: ComponentRole,
    pub integrity: u32,
    pub accumulated_stress: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum CrossingSafety {
    Safe,
    Restricted,
    Unsafe,
    Collapsed,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BridgeStructure {
    pub route_id: RouteId,
    pub name: String,
    pub components: Vec<StructuralComponent>,
}

impl BridgeStructure {
    pub fn stoneford(route: RouteId) -> Self {
        Self {
            route_id: route,
            name: "Stoneford Bridge".into(),
            components: vec![
                component(
                    ComponentId::Deck,
                    "deck planking",
                    MaterialId::Oak,
                    ComponentRole::Deck,
                    INTEGRITY_MAX,
                ),
                component(
                    ComponentId::NorthSupport,
                    "north pier",
                    MaterialId::Limestone,
                    ComponentRole::Support,
                    INTEGRITY_MAX,
                ),
                component(
                    ComponentId::SouthSupport,
                    "south pier",
                    MaterialId::Limestone,
                    ComponentRole::Support,
                    INTEGRITY_MAX,
                ),
                component(
                    ComponentId::TrussEast,
                    "east truss",
                    MaterialId::Iron,
                    ComponentRole::Connection,
                    INTEGRITY_MAX,
                ),
            ],
        }
    }

    pub fn component(&self, id: ComponentId) -> &StructuralComponent {
        self.components
            .iter()
            .find(|c| c.id == id)
            .expect("component id")
    }

    pub fn component_mut(&mut self, id: ComponentId) -> &mut StructuralComponent {
        self.components
            .iter_mut()
            .find(|c| c.id == id)
            .expect("component id")
    }

    pub fn aggregate_integrity(&self) -> u32 {
        let weights = |r: ComponentRole| match r {
            ComponentRole::Support => 3,
            ComponentRole::Connection => 2,
            ComponentRole::Deck => 2,
        };
        let mut sum = 0u64;
        let mut w = 0u64;
        for c in &self.components {
            let weight = weights(c.role) as u64;
            sum += c.integrity as u64 * weight;
            w += weight * INTEGRITY_MAX as u64;
        }
        if w == 0 {
            return 0;
        }
        ((sum * 1000) / w) as u32
    }

    /// Thresholds are tuned to the simplified `impact_damage` / load models (integrity 0–1000).
    pub fn crossing_safety(&self) -> CrossingSafety {
        let agg = self.aggregate_integrity();
        let min_support = self
            .components
            .iter()
            .filter(|c| c.role == ComponentRole::Support)
            .map(|c| c.integrity)
            .min()
            .unwrap_or(0);
        let min_deck = self
            .components
            .iter()
            .filter(|c| c.role == ComponentRole::Deck)
            .map(|c| c.integrity)
            .min()
            .unwrap_or(INTEGRITY_MAX);
        let critical = min_support.min(min_deck);
        if critical < 150 || agg < 180 {
            CrossingSafety::Collapsed
        } else if critical < 520 || agg < 700 {
            CrossingSafety::Unsafe
        } else if critical < 780 || agg < 820 {
            CrossingSafety::Restricted
        } else {
            CrossingSafety::Safe
        }
    }

    pub fn bridge_passable(&self) -> bool {
        !matches!(
            self.crossing_safety(),
            CrossingSafety::Unsafe | CrossingSafety::Collapsed
        )
    }
}

fn component(
    id: ComponentId,
    name: &str,
    material: MaterialId,
    role: ComponentRole,
    integrity: u32,
) -> StructuralComponent {
    StructuralComponent {
        id,
        name: name.into(),
        material,
        role,
        integrity,
        accumulated_stress: 0,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentChange {
    pub component: ComponentId,
    pub integrity_before: u32,
    pub integrity_after: u32,
    pub stress_added: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct StructureInteractionOutcome {
    pub applied: bool,
    pub explanation: String,
    pub safety_before: CrossingSafety,
    pub safety_after: CrossingSafety,
    pub component_changes: Vec<ComponentChange>,
}

pub fn apply_impact(bridge: &mut BridgeStructure, force: u32) -> StructureInteractionOutcome {
    resolve_bridge(
        bridge,
        StructureInteractionKind::Impact,
        BridgeInteractionInput::Impact(ImpactInput {
            force,
            primary_target: None,
        }),
    )
}

pub fn apply_targeted_impact(
    bridge: &mut BridgeStructure,
    force: u32,
    target: ComponentId,
) -> StructureInteractionOutcome {
    resolve_bridge(
        bridge,
        StructureInteractionKind::Impact,
        BridgeInteractionInput::Impact(ImpactInput {
            force,
            primary_target: Some(target),
        }),
    )
}

pub fn apply_sustained_load(bridge: &mut BridgeStructure, load_units: u32) -> StructureInteractionOutcome {
    resolve_bridge(
        bridge,
        StructureInteractionKind::SustainedLoad,
        BridgeInteractionInput::Load(LoadInput { load_units }),
    )
}

pub fn apply_weather(bridge: &mut BridgeStructure) -> StructureInteractionOutcome {
    resolve_bridge(
        bridge,
        StructureInteractionKind::EnvironmentalWear,
        BridgeInteractionInput::Weather,
    )
}

pub fn apply_repair(
    bridge: &mut BridgeStructure,
    labor: u32,
    timber: u32,
    stone: u32,
) -> StructureInteractionOutcome {
    resolve_bridge(
        bridge,
        StructureInteractionKind::Repair,
        BridgeInteractionInput::Repair(RepairInput {
            labor,
            timber,
            stone,
        }),
    )
}

pub fn sync_route_from_structure(
    bridge: &BridgeStructure,
    route: &mut crate::world::Route,
) -> bool {
    let passable = bridge.bridge_passable();
    let changed = route.passable != passable;
    route.passable = passable;
    changed
}
