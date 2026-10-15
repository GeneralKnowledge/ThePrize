use crate::ids::RouteId;
use crate::materials::{self, MaterialId};
use serde::{Deserialize, Serialize};

pub const INTEGRITY_MAX: u32 = 1000;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ComponentRole {
    Deck,
    Support,
    Connection,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
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
    let before = bridge.crossing_safety();
    let mut changes = vec![];
    for component in &mut bridge.components {
        let mat = materials::material(component.material);
        let loss = materials::impact_damage(mat, force, component.integrity);
        if loss == 0 {
            continue;
        }
        let before_i = component.integrity;
        component.integrity = component.integrity.saturating_sub(loss);
        changes.push(ComponentChange {
            component: component.id,
            integrity_before: before_i,
            integrity_after: component.integrity,
            stress_added: 0,
        });
    }
    let after = bridge.crossing_safety();
    StructureInteractionOutcome {
        applied: !changes.is_empty(),
        explanation: format!("impact force {force} on {}", bridge.name),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
}

pub fn apply_sustained_load(bridge: &mut BridgeStructure, load_units: u32) -> StructureInteractionOutcome {
    let before = bridge.crossing_safety();
    let mut changes = vec![];
    for component in &mut bridge.components {
        let mat = materials::material(component.material);
        let stress = materials::load_stress(mat, load_units, component.integrity);
        if stress == 0 {
            continue;
        }
        component.accumulated_stress = component.accumulated_stress.saturating_add(stress);
        let loss = if component.accumulated_stress >= 100 {
            let convert = component.accumulated_stress / 10;
            component.accumulated_stress %= 10;
            convert
        } else {
            0
        };
        if loss == 0 {
            continue;
        }
        let before_i = component.integrity;
        component.integrity = component.integrity.saturating_sub(loss.min(before_i));
        changes.push(ComponentChange {
            component: component.id,
            integrity_before: before_i,
            integrity_after: component.integrity,
            stress_added: stress,
        });
    }
    let after = bridge.crossing_safety();
    StructureInteractionOutcome {
        applied: !changes.is_empty(),
        explanation: format!("sustained load {load_units}"),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
}

pub fn apply_weather(bridge: &mut BridgeStructure) -> StructureInteractionOutcome {
    let before = bridge.crossing_safety();
    let mut changes = vec![];
    for component in &mut bridge.components {
        let mat = materials::material(component.material);
        let loss = materials::weather_decay(mat);
        if loss == 0 {
            continue;
        }
        let before_i = component.integrity;
        component.integrity = component.integrity.saturating_sub(loss.min(before_i));
        changes.push(ComponentChange {
            component: component.id,
            integrity_before: before_i,
            integrity_after: component.integrity,
            stress_added: 0,
        });
    }
    let after = bridge.crossing_safety();
    StructureInteractionOutcome {
        applied: !changes.is_empty(),
        explanation: "environmental wear".into(),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
}

pub fn apply_repair(
    bridge: &mut BridgeStructure,
    labor: u32,
    timber: u32,
    stone: u32,
) -> StructureInteractionOutcome {
    let before = bridge.crossing_safety();
    let mut changes = vec![];
    let targets: Vec<ComponentId> = bridge
        .components
        .iter()
        .filter(|c| c.integrity < INTEGRITY_MAX)
        .map(|c| c.id)
        .collect();
    if targets.is_empty() {
        return StructureInteractionOutcome {
            applied: false,
            explanation: "nothing to repair".into(),
            safety_before: before,
            safety_after: before,
            component_changes: vec![],
        };
    }
    let per = targets.len() as u32;
    let timber_each = timber / per;
    let stone_each = stone / per;
    let labor_each = labor / per;
    for id in targets {
        let component = bridge.component_mut(id);
        let mat = materials::material(component.material);
        let supplies = match component.material {
            MaterialId::Oak => timber_each,
            MaterialId::Limestone => stone_each,
            MaterialId::Iron => timber_each / 2 + stone_each / 2,
        };
        let gain = materials::repair_restoration(mat, labor_each, supplies);
        if gain == 0 {
            continue;
        }
        let before_i = component.integrity;
        component.integrity = (component.integrity + gain).min(INTEGRITY_MAX);
        component.accumulated_stress = component.accumulated_stress.saturating_sub(gain / 2);
        changes.push(ComponentChange {
            component: id,
            integrity_before: before_i,
            integrity_after: component.integrity,
            stress_added: 0,
        });
    }
    let after = bridge.crossing_safety();
    StructureInteractionOutcome {
        applied: !changes.is_empty(),
        explanation: format!("repair labour {labor} timber {timber} stone {stone}"),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
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
