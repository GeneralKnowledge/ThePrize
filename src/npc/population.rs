use crate::ids::{PersonId, SettlementId};
use crate::npc::{Job, Npc, NpcGoals, NpcPresence};
use crate::world::WorldState;

const VILLAGE_NAMES: &[&str] = &[
    "Aldric", "Branwen", "Cedric", "Dara", "Edwin", "Fenn", "Gwen", "Harald", "Iris", "Jory",
    "Kara", "Leif", "Maeve", "Noll", "Orla", "Piers", "Quinn", "Rook", "Sela", "Tomas", "Una",
    "Viggo", "Willa", "Yorick",
];

const TOWN_NAMES: &[&str] = &[
    "Alaric", "Bess", "Colm", "Delia", "Ewan", "Faye", "Garrick", "Helena", "Idris", "Jana",
    "Knox", "Lina", "Marten", "Nessa", "Oswin", "Petra",
];

fn village_job(index: usize) -> Job {
    if index < 14 {
        Job::Farmer
    } else {
        Job::Labourer
    }
}

fn town_job(index: usize) -> Job {
    match index {
        0 => Job::Miller,
        1 | 6 => Job::Baker,
        2 | 3 => Job::Guard,
        _ => Job::Labourer,
    }
}

pub fn populate_npcs(world: &mut WorldState) {
    let village = SettlementId(0);
    let town = SettlementId(1);
    let mut id = 1u32;

    for (i, name) in VILLAGE_NAMES.iter().enumerate() {
        world.npcs.push(make_npc(
            PersonId(id),
            name,
            village,
            village_job(i),
            8,
            4,
        ));
        id += 1;
    }

    for (i, name) in TOWN_NAMES.iter().enumerate() {
        let mut npc = make_npc(PersonId(id), name, town, town_job(i), 10, 3);
        if *name == "Garrick" {
            npc.goals.debt_remaining = 40;
            npc.goals.save_coin_target = 20;
            npc.coin = 12;
            npc.grain = 1;
        }
        if town_job(i) == Job::Miller {
            npc.coin = 25;
        }
        world.npcs.push(npc);
        id += 1;
    }

    crate::npc::sync_settlement_population(world);
}

fn make_npc(
    id: PersonId,
    name: &str,
    home: SettlementId,
    job: Job,
    coin: u32,
    grain: u32,
) -> Npc {
    Npc {
        id,
        name: name.into(),
        home,
        presence: NpcPresence::At(home),
        job,
        coin,
        grain,
        hunger: 0.15,
        goals: NpcGoals::default(),
        relationships: vec![],
        beliefs: vec![],
        memories: vec![],
        worked_today: false,
        last_decision: None,
    }
}
