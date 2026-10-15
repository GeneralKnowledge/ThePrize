use crate::events::{EventLog, EventRecord, WorldEvent};
use crate::ids::CauseId;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CausalLink {
    pub day: u32,
    pub cause: CauseId,
    pub parent: Option<CauseId>,
    pub description: String,
}

pub fn trace_from_cause(log: &EventLog, root: CauseId) -> Vec<CausalLink> {
    let mut chain = vec![];
    let mut frontier = vec![root];
    let mut seen = std::collections::HashSet::new();

    while let Some(cause) = frontier.pop() {
        if !seen.insert(cause) {
            continue;
        }
        for record in log.records.iter().filter(|r| r.cause == cause || r.parent_cause == Some(cause)) {
            if record.cause == cause || record.parent_cause == Some(cause) {
                push_record(&mut chain, record);
                if record.cause != cause && !seen.contains(&record.cause) {
                    frontier.push(record.cause);
                }
                if let Some(parent) = record.parent_cause {
                    if !seen.contains(&parent) {
                        frontier.push(parent);
                    }
                }
            }
        }
    }
    chain.sort_by_key(|l| l.day);
    chain
}

pub fn find_cause_for_event(log: &EventLog, predicate: impl Fn(&WorldEvent) -> bool) -> Option<CauseId> {
    log.records
        .iter()
        .find(|r| predicate(&r.event))
        .map(|r| r.cause)
}

fn push_record(chain: &mut Vec<CausalLink>, record: &EventRecord) {
    if chain.iter().any(|l| l.cause == record.cause && l.day == record.day) {
        return;
    }
    chain.push(CausalLink {
        day: record.day,
        cause: record.cause,
        parent: record.parent_cause,
        description: format!("{:?}", record.event),
    });
}

pub fn summarize_bridge_collapse(log: &EventLog) -> Vec<String> {
    let root = find_cause_for_event(log, |e| matches!(e, WorldEvent::RouteBlocked { .. }));
    let Some(root) = root else {
        return vec![];
    };
    trace_from_cause(log, root)
        .into_iter()
        .map(|l| format!("day {}: {}", l.day, l.description))
        .collect()
}
