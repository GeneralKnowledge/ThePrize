use crate::events::{EventLog, WorldEvent};
use crate::ids::{PersonId, SettlementId};
use crate::npc::{Job, Npc, NpcPresence};
use crate::npc::types::{DecisionReason, NpcAction, RecordedDecision};
use crate::property::mill_operational;
use crate::world::WorldState;
use crate::world_action::{Actor, WorldAction};

#[derive(Clone, Copy, Debug)]
struct Choice {
    action: NpcAction,
    reason: DecisionReason,
    score: i32,
    detail: &'static str,
}

pub fn run_npc_decisions(world: &mut WorldState, log: &mut EventLog) {
    let day = world.day;
    for npc in &mut world.npcs {
        npc.worked_today = false;
    }

    let mut order: Vec<PersonId> = world.npcs.iter().map(|n| n.id).collect();
    order.sort_by_key(|id| id.0);

    for person_id in order {
        if !world.npc(person_id).alive {
            continue;
        }
        let choice = {
            let npc = world.npc(person_id);
            evaluate(npc, world)
        };
        apply_decision(world, log, person_id, day, choice);
    }
}

fn evaluate(npc: &Npc, world: &WorldState) -> Choice {
    if !npc.alive {
        return Choice {
            action: NpcAction::Idle,
            reason: DecisionReason::Rested,
            score: 0,
            detail: "deceased",
        };
    }
    let mut choices = vec![];

    if matches!(npc.presence, NpcPresence::Travelling { .. }) {
        choices.push(Choice {
            action: NpcAction::WaitOutTravel,
            reason: DecisionReason::Travelling,
            score: 1000,
            detail: "on the road",
        });
    } else     if npc.grain >= 1 && npc.hunger >= 0.2 {
        choices.push(Choice {
            action: NpcAction::Eat,
            reason: if npc.hunger > 0.75 {
                DecisionReason::CriticalHunger
            } else {
                DecisionReason::LowPersonalGrain
            },
            score: (npc.hunger * 300.0) as i32 + 200,
            detail: "eat from personal stores",
        });
    }

    if npc.hunger > 0.88 && npc.grain == 0 && npc.coin == 0 {
        if npc.location().is_some() {
            choices.push(Choice {
                action: NpcAction::StealFood,
                reason: DecisionReason::DesperateTheft,
                score: 480,
                detail: "steal grain out of desperation",
            });
        }
    }

    if npc.hunger >= 0.45 {
        if let Some(loc) = npc.location() {
            let price = world.settlement(loc).market.grain_price;
            let stock = world.settlement(loc).grain_inventory;
            let affordable = (npc.coin as f64 / price).floor() as u32;
            if affordable > 0 && stock > 5 {
                choices.push(Choice {
                    action: NpcAction::BuyFood,
                    reason: DecisionReason::NeedIncomeForFood,
                    score: (npc.hunger * 180.0) as i32 + 40,
                    detail: "buy grain at local market",
                });
            }
        }
    }

    if npc.goals.debt_remaining > 0 && npc.coin > 0 {
        choices.push(Choice {
            action: NpcAction::Work,
            reason: DecisionReason::PayDebt,
            score: 120 + npc.hunger as i32 * 20,
            detail: "work to pay debt",
        });
    }

    choices.push(Choice {
        action: NpcAction::Work,
        reason: DecisionReason::JobDuty,
        score: match npc.job {
            Job::Farmer => 90,
            Job::Guard => 85,
            Job::Miller | Job::Baker => 80,
            Job::Labourer => 70,
            Job::Carter => 60,
        },
        detail: "ordinary work",
    });

    if npc.job == Job::Labourer && npc.hunger > 0.35 {
        let home = npc.home;
        let other = if home == SettlementId(0) {
            SettlementId(1)
        } else {
            SettlementId(0)
        };
        let bridge = world.bridge_between(SettlementId(0), SettlementId(1));
        let passable = bridge.map(|r| r.passable).unwrap_or(false);
        if passable && !npc.knows_bridge_unsafe() {
            let wage_here = npc.job.daily_wage();
            let wage_there = Job::Labourer.daily_wage() + if other == SettlementId(1) { 1 } else { 0 };
            if wage_there > wage_here && npc.location() == Some(home) {
                choices.push(Choice {
                    action: NpcAction::Travel,
                    reason: DecisionReason::SeekBetterWages,
                    score: 75 + (npc.hunger * 50.0) as i32,
                    detail: "travel for better wages",
                });
            }
        } else if !passable && npc.location() == Some(other) {
            choices.push(Choice {
                action: NpcAction::Travel,
                reason: DecisionReason::BridgeBlocked,
                score: 110,
                detail: "return home while bridge is down",
            });
        }
    }

    choices.push(Choice {
        action: NpcAction::Idle,
        reason: DecisionReason::Rested,
        score: 10,
        detail: "no pressing need",
    });

    choices
        .into_iter()
        .max_by_key(|c| c.score)
        .unwrap_or(Choice {
            action: NpcAction::Idle,
            reason: DecisionReason::Rested,
            score: 0,
            detail: "fallback",
        })
}

fn apply_decision(
    world: &mut WorldState,
    log: &mut EventLog,
    person_id: PersonId,
    day: u32,
    choice: Choice,
) {
    world.npc_mut(person_id).last_decision = Some(RecordedDecision {
        day,
        action: choice.action,
        reason: choice.reason,
        detail: choice.detail.into(),
    });

    match choice.action {
        NpcAction::Eat => {
            let npc = world.npc_mut(person_id);
            let eaten = npc.grain.min(1);
            let home = npc.home;
            let loc = npc.location().unwrap_or(home);
            npc.grain -= eaten;
            npc.hunger = (npc.hunger - 0.55).max(0.0);
            log.emit(
                day,
                WorldEvent::NpcAte {
                    person: person_id,
                    grain: eaten,
                    settlement: loc,
                },
                None,
            );
        }
        NpcAction::BuyFood => {
            let purchase = world.npc(person_id).location().and_then(|loc| {
                let price = world.settlement(loc).market.grain_price;
                let coin = world.npc(person_id).coin;
                let max_buy = ((coin as f64 / price).floor() as u32).clamp(1, 3);
                let stock = world.settlement(loc).grain_inventory;
                let qty = max_buy.min(stock.saturating_sub(1));
                if qty == 0 {
                    return None;
                }
                let cost = (qty as f64 * price).ceil() as u32;
                if cost > coin {
                    return None;
                }
                Some((loc, qty, cost))
            });
            if let Some((loc, qty, cost)) = purchase {
                let npc = world.npc_mut(person_id);
                npc.coin -= cost;
                npc.grain += qty;
                world.settlement_mut(loc).grain_inventory -= qty;
                log.emit(
                    day,
                    WorldEvent::NpcBoughtFood {
                        person: person_id,
                        settlement: loc,
                        grain: qty,
                        coin_spent: cost,
                    },
                    None,
                );
            }
        }
        NpcAction::Work => execute_work(world, log, person_id, day, choice.reason),
        NpcAction::Travel => execute_travel(world, log, person_id, day, choice.reason),
        NpcAction::WaitOutTravel => advance_travel(world, log, person_id, day),
        NpcAction::StealFood => {
            if let Some(loc) = world.npc(person_id).location() {
                world.pending_actions.push(WorldAction::StealSettlementGrain {
                    thief: Actor::Npc(person_id),
                    settlement: loc,
                    quantity: 8,
                });
            }
        }
        NpcAction::Idle => {}
    }
}

fn execute_work(
    world: &mut WorldState,
    log: &mut EventLog,
    person_id: PersonId,
    day: u32,
    reason: DecisionReason,
) {
    let job = world.npc(person_id).job;
    if job == Job::Miller && !mill_operational(world) {
        return;
    }
    let npc = world.npc_mut(person_id);
    let loc = match npc.location() {
        Some(s) => s,
        None => return,
    };
    npc.worked_today = true;
    let wage = npc.job.daily_wage();
    npc.coin += wage;

    if npc.goals.debt_remaining > 0 {
        let payment = wage.min(npc.coin).min(npc.goals.debt_remaining);
        npc.coin -= payment;
        npc.goals.debt_remaining -= payment;
    }

    log.emit(
        day,
        WorldEvent::NpcWorked {
            person: person_id,
            settlement: loc,
            job: npc.job,
            wage,
            reason,
        },
        None,
    );
}

fn execute_travel(
    world: &mut WorldState,
    log: &mut EventLog,
    person_id: PersonId,
    day: u32,
    reason: DecisionReason,
) {
    let (from, home) = {
        let npc = world.npc(person_id);
        (
            match npc.location() {
                Some(s) => s,
                None => return,
            },
            npc.home,
        )
    };
    let to = if reason == DecisionReason::BridgeBlocked {
        home
    } else if from == SettlementId(0) {
        SettlementId(1)
    } else {
        SettlementId(0)
    };
    if to == from {
        return;
    }

    let passable = world
        .bridge_between(SettlementId(0), SettlementId(1))
        .map(|r| r.passable)
        .unwrap_or(false);
    if !passable && reason != DecisionReason::BridgeBlocked {
        let npc = world.npc_mut(person_id);
        if !npc.beliefs.iter().any(|b| b == "bridge_unsafe") {
            npc.beliefs.push("bridge_unsafe".into());
        }
        return;
    }

    let npc = world.npc_mut(person_id);
    npc.presence = NpcPresence::Travelling {
        to,
        days_remaining: if passable { 1 } else { 3 },
    };
    log.emit(
        day,
        WorldEvent::NpcTravelStarted {
            person: person_id,
            from,
            to,
            reason,
        },
        None,
    );
}

fn advance_travel(world: &mut WorldState, log: &mut EventLog, person_id: PersonId, day: u32) {
    let npc = world.npc_mut(person_id);
    if let NpcPresence::Travelling {
        to,
        days_remaining,
    } = npc.presence
    {
        if days_remaining > 1 {
            npc.presence = NpcPresence::Travelling {
                to,
                days_remaining: days_remaining - 1,
            };
        } else {
            npc.presence = NpcPresence::At(to);
            log.emit(
                day,
                WorldEvent::NpcTravelArrived {
                    person: person_id,
                    settlement: to,
                },
                None,
            );
        }
    }
}

pub fn tick_npc_hunger(world: &mut WorldState, log: &mut EventLog) {
    let day = world.day;
    let rate = world.grain_per_person_per_day as f32 * 0.12;
    for npc in world.npcs.iter_mut().filter(|n| n.alive) {
        npc.hunger = (npc.hunger + rate).min(1.0);
        if npc.hunger >= 0.98 {
            log.emit(
                day,
                WorldEvent::NpcStarving { person: npc.id },
                None,
            );
        }
    }
}

pub fn inform_bridge_blocked(world: &mut WorldState, day: u32) {
    for npc in &mut world.npcs {
        if matches!(npc.job, Job::Guard | Job::Carter) {
            if !npc.beliefs.iter().any(|b| b == "bridge_unsafe") {
                npc.beliefs.push("bridge_unsafe".into());
                npc.remember(day, "Stoneford Bridge is impassable");
            }
        } else if npc.location() == Some(SettlementId(0)) || npc.location() == Some(SettlementId(1))
        {
            // Witnesses at either end of the bridge learn the same day.
            if !npc.beliefs.iter().any(|b| b == "bridge_unsafe") {
                npc.beliefs.push("bridge_unsafe".into());
                npc.remember(day, "heard the bridge was damaged");
            }
        }
    }
}

pub fn farmer_work_fraction(world: &WorldState, settlement: SettlementId) -> f64 {
    let farmers: Vec<_> = world
        .npcs
        .iter()
        .filter(|n| n.alive && n.job == Job::Farmer && n.home == settlement)
        .collect();
    if farmers.is_empty() {
        return 1.0;
    }
    let worked = farmers.iter().filter(|n| n.worked_today).count();
    (worked as f64 / farmers.len() as f64).clamp(0.35, 1.0)
}
