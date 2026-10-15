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

pub fn load_stress(material: MaterialDef, load_units: u32, current_integrity: u32) -> u32 {
    if load_units == 0 {
        return 0;
    }
    let capacity = material.toughness.saturating_mul(8).saturating_add(200);
    if load_units <= capacity / 10 {
        return 0;
    }
    let excess = load_units.saturating_sub(capacity / 10);
    let stress = excess.saturating_mul(100) / (material.toughness + material.hardness);
    let worn_factor = 1 + (1000 - current_integrity.min(1000)) / 250;
    (stress * worn_factor).max(0).min(40)
}

pub fn repair_restoration(material: MaterialDef, labor: u32, supplies: u32) -> u32 {
    let work = labor.saturating_mul(material.toughness / 10 + 2);
    let supply = supplies.saturating_mul(material.hardness / 8 + 1);
    work.saturating_add(supply).min(120)
}

pub fn weather_decay(material: MaterialDef) -> u32 {
    (120u32.saturating_sub(material.weather_resistance) / 15).max(0).min(3)
}
