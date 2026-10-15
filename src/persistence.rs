use crate::events::EventRecord;
use crate::scenarios::ScenarioKind;
use crate::world::WorldState;
use rusqlite::{params, Connection};
use serde_json;
use std::path::Path;

pub struct RunPersistence<'a> {
    pub scenario: ScenarioKind,
    pub seed: u64,
    pub world: &'a WorldState,
    pub log: &'a crate::events::EventLog,
}

pub fn persist_run(path: &Path, run: &RunPersistence<'_>) -> rusqlite::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::remove_file(path);
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "
        CREATE TABLE run_meta (
            scenario TEXT NOT NULL,
            seed INTEGER NOT NULL,
            final_day INTEGER NOT NULL
        );
        CREATE TABLE settlements (
            id INTEGER NOT NULL,
            name TEXT NOT NULL,
            population INTEGER NOT NULL,
            grain INTEGER NOT NULL,
            grain_price REAL NOT NULL
        );
        CREATE TABLE events (
            id INTEGER NOT NULL,
            day INTEGER NOT NULL,
            parent_cause INTEGER,
            cause INTEGER NOT NULL,
            payload TEXT NOT NULL
        );
        ",
    )?;

    conn.execute(
        "INSERT INTO run_meta (scenario, seed, final_day) VALUES (?1, ?2, ?3)",
        params![
            format!("{:?}", run.scenario),
            run.seed as i64,
            run.world.day as i64
        ],
    )?;

    for s in &run.world.settlements {
        conn.execute(
            "INSERT INTO settlements (id, name, population, grain, grain_price) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                s.id.0 as i64,
                s.name,
                s.population as i64,
                s.grain_inventory as i64,
                s.market.grain_price
            ],
        )?;
    }

    for record in &run.log.records {
        insert_event(&conn, record)?;
    }

    Ok(())
}

fn insert_event(conn: &Connection, record: &EventRecord) -> rusqlite::Result<()> {
    let payload = serde_json::to_string(&record.event).expect("event json");
    conn.execute(
        "INSERT INTO events (id, day, parent_cause, cause, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            record.id.0 as i64,
            record.day as i64,
            record.parent_cause.map(|c| c.0 as i64),
            record.cause.0 as i64,
            payload
        ],
    )?;
    Ok(())
}
