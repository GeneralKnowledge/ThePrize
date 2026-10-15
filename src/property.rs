use crate::ids::{PersonId, PropertyId, SettlementId};
use crate::npc::Job;
use crate::world::{Settlement, WorldState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PropertyKind {
    Mill,
    FarmEstate,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Property {
    pub id: PropertyId,
    pub name: String,
    pub settlement: SettlementId,
    pub kind: PropertyKind,
    pub owner: PersonId,
    pub operational: bool,
    /// 0 = intact, 100 = ruined.
    pub damage: u32,
}

impl WorldState {
    pub fn property(&self, id: PropertyId) -> &Property {
        self.properties
            .iter()
            .find(|p| p.id == id)
            .expect("property id")
    }

    pub fn property_mut(&mut self, id: PropertyId) -> &mut Property {
        self.properties
            .iter_mut()
            .find(|p| p.id == id)
            .expect("property id")
    }
}

pub fn initial_properties(world: &WorldState) -> Vec<Property> {
    let village = SettlementId(0);
    let town = SettlementId(1);
    let miller = world
        .npcs
        .iter()
        .find(|n| n.job == Job::Miller && n.home == town)
        .expect("miller");
    let farmer = world
        .npcs
        .iter()
        .find(|n| n.job == Job::Farmer && n.home == village)
        .expect("farmer");
    vec![
        Property {
            id: PropertyId(0),
            name: "Millford Mill".into(),
            settlement: town,
            kind: PropertyKind::Mill,
            owner: miller.id,
            operational: true,
            damage: 0,
        },
        Property {
            id: PropertyId(1),
            name: "East Farm".into(),
            settlement: village,
            kind: PropertyKind::FarmEstate,
            owner: farmer.id,
            operational: true,
            damage: 0,
        },
    ]
}

/// Apply property damage to settlement production capacity (called after damage events).
pub fn apply_property_effects_to_settlement(settlement: &mut Settlement, properties: &[Property]) {
    let mut farm_capacity = settlement.base_farm_daily_capacity;
    for prop in properties.iter().filter(|p| p.settlement == settlement.id) {
        if prop.kind != PropertyKind::FarmEstate {
            continue;
        }
        let factor = 1.0 - (prop.damage.min(100) as f64 / 100.0);
        if !prop.operational {
            farm_capacity = farm_capacity.saturating_sub(8);
        } else {
            farm_capacity = ((farm_capacity as f64) * (0.85 + 0.15 * factor)).floor() as u32;
        }
    }
    settlement.farm_daily_capacity = farm_capacity.max(1);
}

pub fn mill_operational(world: &WorldState) -> bool {
    world
        .properties
        .iter()
        .find(|p| p.kind == PropertyKind::Mill)
        .map(|p| p.operational && p.damage < 80)
        .unwrap_or(true)
}
