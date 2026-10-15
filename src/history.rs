use crate::events::{EventLog, EventRecord, WorldEvent};
use crate::ids::CauseId;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};

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
    let mut seen = HashSet::new();

    while let Some(cause) = frontier.pop() {
        if !seen.insert(cause) {
            continue;
        }
        for record in log
            .records
            .iter()
            .filter(|r| r.cause == cause || r.parent_cause == Some(cause))
        {
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

/// Walk parent_cause links from an event's cause toward initiating actions.
pub fn ancestors_of(log: &EventLog, start: CauseId) -> Vec<CausalLink> {
    let mut out = vec![];
    let mut seen = HashSet::new();
    let mut current = Some(start);

    while let Some(cause) = current {
        if !seen.insert(cause) {
            break;
        }
        let Some(record) = log.records.iter().find(|r| r.cause == cause) else {
            break;
        };
        push_record(&mut out, record);
        current = record.parent_cause;
    }
    out.sort_by_key(|l| l.day);
    out
}

/// Events whose parent_cause is in the frontier (breadth-first descendants).
pub fn descendants_of(log: &EventLog, root: CauseId) -> Vec<CausalLink> {
    let mut out = vec![];
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([root]);

    while let Some(cause) = queue.pop_front() {
        if !seen.insert(cause) {
            continue;
        }
        for record in &log.records {
            if record.parent_cause == Some(cause) {
                push_record(&mut out, record);
                if !seen.contains(&record.cause) {
                    queue.push_back(record.cause);
                }
            }
        }
    }
    out.sort_by_key(|l| l.day);
    out
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

/// Stable fingerprint for regression: day + event debug + parent cause per record.
/// Returns invalid parent references or self-parent links.
pub fn validate_cause_graph(log: &EventLog) -> Vec<String> {
    let mut known = std::collections::HashSet::new();
    for record in &log.records {
        known.insert(record.cause);
    }
    let mut problems = vec![];
    for record in &log.records {
        if let Some(parent) = record.parent_cause {
            if parent == record.cause {
                problems.push(format!("self-parent on cause {:?}", record.cause));
            }
            if !known.contains(&parent) {
                problems.push(format!(
                    "orphan parent {:?} on event {:?}",
                    parent, record.id
                ));
            }
        }
    }
    problems
}

pub fn canonical_history_digest(log: &EventLog) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for record in &log.records {
        record.day.hash(&mut hasher);
        format!("{:?}", record.event).hash(&mut hasher);
        record.parent_cause.map(|c| c.0).hash(&mut hasher);
    }
    hasher.finish()
}
