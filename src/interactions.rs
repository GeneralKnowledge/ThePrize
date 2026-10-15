//! Typed structural interaction inputs — bridge is the first adapter.

use crate::materials::{self, MaterialDef};
use crate::structures::{
    BridgeStructure, ComponentChange, ComponentId, ComponentRole, StructureInteractionOutcome,
    INTEGRITY_MAX,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureInteractionKind {
    Impact,
    SustainedLoad,
    EnvironmentalWear,
    Repair,
}

#[derive(Clone, Copy, Debug)]
pub struct ImpactInput {
    pub force: u32,
    /// Primary strike location; `None` spreads force across all components (e.g. flood).
    pub primary_target: Option<ComponentId>,
}

#[derive(Clone, Copy, Debug)]
pub struct LoadInput {
    pub load_units: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct RepairInput {
    pub labor: u32,
    pub timber: u32,
    pub stone: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum BridgeInteractionInput {
    Impact(ImpactInput),
    Load(LoadInput),
    Repair(RepairInput),
    Weather,
}

fn role_load_share(role: ComponentRole) -> u32 {
    match role {
        ComponentRole::Deck => 4,
        ComponentRole::Support => 3,
        ComponentRole::Connection => 2,
    }
}

fn spillover_targets(primary: ComponentId) -> &'static [ComponentId] {
    match primary {
        ComponentId::Deck => &[ComponentId::TrussEast],
        ComponentId::NorthSupport | ComponentId::SouthSupport => &[ComponentId::Deck],
        ComponentId::TrussEast => &[ComponentId::Deck],
    }
}

fn impact_on_component(mat: MaterialDef, force: u32, integrity: u32) -> u32 {
    materials::impact_damage(mat, force, integrity)
}

pub fn resolve_bridge(
    bridge: &mut BridgeStructure,
    kind: StructureInteractionKind,
    input: BridgeInteractionInput,
) -> StructureInteractionOutcome {
    match kind {
        StructureInteractionKind::Impact => {
            let ImpactInput {
                force,
                primary_target,
            } = match input {
                BridgeInteractionInput::Impact(i) => i,
                _ => panic!("impact input required"),
            };
            resolve_impact(bridge, force, primary_target)
        }
        StructureInteractionKind::SustainedLoad => {
            let load_units = match input {
                BridgeInteractionInput::Load(l) => l.load_units,
                _ => panic!("load input required"),
            };
            resolve_load(bridge, load_units)
        }
        StructureInteractionKind::EnvironmentalWear => resolve_weather(bridge),
        StructureInteractionKind::Repair => {
            let RepairInput {
                labor,
                timber,
                stone,
            } = match input {
                BridgeInteractionInput::Repair(r) => r,
                _ => panic!("repair input required"),
            };
            resolve_repair(bridge, labor, timber, stone)
        }
    }
}

fn resolve_impact(
    bridge: &mut BridgeStructure,
    force: u32,
    primary_target: Option<ComponentId>,
) -> StructureInteractionOutcome {
    let before = bridge.crossing_safety();
    let mut changes = vec![];

    let mut apply_to = |id: ComponentId, effective_force: u32, changes: &mut Vec<ComponentChange>| {
        if effective_force == 0 {
            return;
        }
        let component = bridge.component_mut(id);
        if component.integrity == 0 {
            return;
        }
        let mat = materials::material(component.material);
        let loss = impact_on_component(mat, effective_force, component.integrity);
        if loss == 0 {
            return;
        }
        let before_i = component.integrity;
        component.integrity = component.integrity.saturating_sub(loss);
        changes.push(ComponentChange {
            component: id,
            integrity_before: before_i,
            integrity_after: component.integrity,
            stress_added: 0,
        });
    };

    match primary_target {
        None => {
            for id in [
                ComponentId::Deck,
                ComponentId::NorthSupport,
                ComponentId::SouthSupport,
                ComponentId::TrussEast,
            ] {
                apply_to(id, force, &mut changes);
            }
        }
        Some(primary) => {
            apply_to(primary, force, &mut changes);
            let spill = force / 3;
            for &linked in spillover_targets(primary) {
                if linked != primary {
                    apply_to(linked, spill, &mut changes);
                }
            }
        }
    }

    let after = bridge.crossing_safety();
    StructureInteractionOutcome {
        applied: !changes.is_empty(),
        explanation: format!(
            "impact force {force} on {} (target {:?})",
            bridge.name, primary_target
        ),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
}

fn resolve_load(bridge: &mut BridgeStructure, load_units: u32) -> StructureInteractionOutcome {
    let before = bridge.crossing_safety();
    let mut changes = vec![];
    let mut stress_applied = false;
    for component in &mut bridge.components {
        let mat = materials::material(component.material);
        let share = role_load_share(component.role);
        let stress =
            materials::load_stress(mat, load_units, share, component.integrity);
        if stress == 0 {
            continue;
        }
        stress_applied = true;
        component.accumulated_stress = component.accumulated_stress.saturating_add(stress);
        let loss = if component.accumulated_stress >= 160 {
            let convert = component.accumulated_stress / 16;
            component.accumulated_stress %= 16;
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
        applied: stress_applied || !changes.is_empty(),
        explanation: format!("sustained load {load_units}"),
        safety_before: before,
        safety_after: after,
        component_changes: changes,
    }
}

fn resolve_weather(bridge: &mut BridgeStructure) -> StructureInteractionOutcome {
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

fn resolve_repair(
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
            crate::materials::MaterialId::Oak => timber_each,
            crate::materials::MaterialId::Limestone => stone_each,
            crate::materials::MaterialId::Iron => timber_each / 2 + stone_each / 2,
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
