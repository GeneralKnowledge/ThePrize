use crate::ids::{PersonId, RouteId, SettlementId};
use crate::npc::Npc;
use crate::property::Property;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Settlement {
    pub id: SettlementId,
    pub name: String,
    pub population: u32,
    /// Grain stored for food (kg abstract units).
    pub grain_inventory: u32,
    /// Daily grain output capacity at full season multiplier (after property effects).
    pub farm_daily_capacity: u32,
    pub base_farm_daily_capacity: u32,
    /// Desired grain buffer for pricing (days of consumption at current pop).
    pub market: MarketState,
    /// Abstract employer/tax float used to pay local wages (not yet a full treasury sim).
    pub treasury_coin: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MarketState {
    pub grain_price: f64,
    pub desired_grain_stock: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Route {
    pub id: RouteId,
    pub name: String,
    pub from: SettlementId,
    pub to: SettlementId,
    pub passable: bool,
    /// Extra days of travel when using alternative ford (no bridge).
    pub alternative_travel_days: u32,
    pub repair_progress: u32,
    pub repair_days_required: u32,
    pub repair_in_progress: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum MerchantLocation {
    At(SettlementId),
    Travelling {
        from: SettlementId,
        to: SettlementId,
        days_remaining: u32,
        carrying_grain: u32,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Merchant {
    pub id: PersonId,
    pub name: String,
    pub coin: u32,
    pub location: MerchantLocation,
    pub grain_capacity: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WorldState {
    pub day: u32,
    pub settlements: Vec<Settlement>,
    pub routes: Vec<Route>,
    pub merchant: Merchant,
    pub npcs: Vec<Npc>,
    pub properties: Vec<Property>,
    /// Actions queued during NPC decisions, applied before production.
    pub pending_actions: Vec<crate::world_action::WorldAction>,
    /// Grain per person per day.
    pub grain_per_person_per_day: f64,
}

impl WorldState {
    pub fn settlement(&self, id: SettlementId) -> &Settlement {
        self.settlements
            .iter()
            .find(|s| s.id == id)
            .expect("settlement id")
    }

    pub fn settlement_mut(&mut self, id: SettlementId) -> &mut Settlement {
        self.settlements
            .iter_mut()
            .find(|s| s.id == id)
            .expect("settlement id")
    }

    pub fn route(&self, id: RouteId) -> &Route {
        self.routes.iter().find(|r| r.id == id).expect("route id")
    }

    pub fn route_mut(&mut self, id: RouteId) -> &mut Route {
        self.routes
            .iter_mut()
            .find(|r| r.id == id)
            .expect("route id")
    }

    pub fn bridge_between(&self, a: SettlementId, b: SettlementId) -> Option<&Route> {
        self.routes.iter().find(|r| {
            (r.from == a && r.to == b) || (r.from == b && r.to == a)
        })
    }

    pub fn daily_consumption(&self, settlement: SettlementId) -> u32 {
        let pop = self.settlement(settlement).population;
        (pop as f64 * self.grain_per_person_per_day).ceil() as u32
    }

    pub fn npc(&self, id: PersonId) -> &Npc {
        self.npcs.iter().find(|n| n.id == id).expect("npc id")
    }

    pub fn npc_mut(&mut self, id: PersonId) -> &mut Npc {
        self.npcs
            .iter_mut()
            .find(|n| n.id == id)
            .expect("npc id")
    }
}

pub fn initial_world() -> WorldState {
    let village = SettlementId(0);
    let town = SettlementId(1);
    let mut world = WorldState {
        day: 0,
        grain_per_person_per_day: 0.65,
        npcs: vec![],
        properties: vec![],
        pending_actions: vec![],
        settlements: vec![
            Settlement {
                id: village,
                name: "Hollowbrook".into(),
                population: 24,
                grain_inventory: 220,
                farm_daily_capacity: 32,
                base_farm_daily_capacity: 32,
                market: MarketState {
                    grain_price: 1.0,
                    desired_grain_stock: 100,
                },
                treasury_coin: 240,
            },
            Settlement {
                id: town,
                name: "Millford".into(),
                population: 16,
                grain_inventory: 70,
                farm_daily_capacity: 1,
                base_farm_daily_capacity: 1,
                market: MarketState {
                    grain_price: 1.35,
                    desired_grain_stock: 55,
                },
                treasury_coin: 180,
            },
        ],
        routes: vec![Route {
            id: RouteId(0),
            name: "Stoneford Bridge".into(),
            from: village,
            to: town,
            passable: true,
            alternative_travel_days: 3,
            repair_progress: 0,
            repair_days_required: 14,
            repair_in_progress: false,
        }],
        merchant: Merchant {
            id: PersonId(0),
            name: "Corvin the Carter".into(),
            coin: 120,
            location: MerchantLocation::At(village),
            grain_capacity: 18,
        },
    };
    crate::npc::population::populate_npcs(&mut world);
    world.properties = crate::property::initial_properties(&world);
    let props = world.properties.clone();
    for settlement in &mut world.settlements {
        settlement.farm_daily_capacity = settlement.base_farm_daily_capacity;
        crate::property::apply_property_effects_to_settlement(settlement, &props);
    }
    world
}
