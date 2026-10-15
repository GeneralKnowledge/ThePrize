use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MaterialId {
    Oak,
    Limestone,
    Iron,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MaterialDef {
    pub id: MaterialId,
    pub name: &'static str,
    /// Relative scale 1–100 (not SI).
    pub hardness: u32,
    pub toughness: u32,
    pub brittleness: u32,
    /// Resistance to weather/decay per tick (higher = slower decay).
    pub weather_resistance: u32,
}

pub fn material(id: MaterialId) -> MaterialDef {
    match id {
        MaterialId::Oak => MaterialDef {
            id,
            name: "oak",
            hardness: 42,
            toughness: 55,
            brittleness: 35,
            weather_resistance: 28,
        },
        MaterialId::Limestone => MaterialDef {
            id,
            name: "limestone",
            hardness: 48,
            toughness: 62,
            brittleness: 40,
            weather_resistance: 55,
        },
        MaterialId::Iron => MaterialDef {
            id,
            name: "wrought iron",
            hardness: 72,
            toughness: 78,
            brittleness: 25,
            weather_resistance: 45,
        },
    }
}

/// How much integrity loss an impact transfers given material response.
pub fn impact_damage(material: MaterialDef, force: u32, current_integrity: u32) -> u32 {
    if force == 0 || current_integrity == 0 {
        return 0;
    }
    let weakness = (100u32.saturating_sub(material.toughness)).max(5);
    let brittleness_bonus = material.brittleness / 4;
    let worn = (1000u32.saturating_sub(current_integrity)) / 20;
    let raw = force
        .saturating_mul(weakness + brittleness_bonus + worn)
        / (material.hardness + material.toughness);
    raw.max(1).min(current_integrity)
}

/// Daily traffic stress on one component. `role_share` weights deck vs pier vs truss (see interactions).
/// Calibrated so `load_units = 12` (normal bridge traffic) adds ~3–8 stress/day depending on material.
pub fn load_stress(
    material: MaterialDef,
    load_units: u32,
    role_share: u32,
    current_integrity: u32,
) -> u32 {
    if load_units == 0 || role_share == 0 {
        return 0;
    }
    let flex = material.toughness.saturating_add(material.hardness / 2).max(1);
    let on_component = load_units.saturating_mul(role_share) / 4;
    let base = on_component.saturating_mul(48) / flex;
    let worn_factor = 1 + (1000 - current_integrity.min(1000)) / 250;
    (base.saturating_mul(worn_factor)).max(1).min(28)
}

pub fn repair_restoration(material: MaterialDef, labor: u32, supplies: u32) -> u32 {
    let work = labor.saturating_mul(material.toughness / 10 + 2);
    let supply = supplies.saturating_mul(material.hardness / 8 + 1);
    work.saturating_add(supply).min(120)
}

/// Weekly environmental integrity loss (material-specific; slow enough for ~1yr baseline viability).
pub fn weather_decay(material: MaterialDef) -> u32 {
    let raw = 1 + (85u32.saturating_sub(material.weather_resistance)) / 22;
    raw.max(1).min(5)
}
