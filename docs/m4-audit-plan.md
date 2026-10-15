# M4 Phase 0 audit (main @ M3 merge)

Baseline: `cargo test` — **19 passed** before M4 edits.

## 1. Time advancement

`WorldSimulator::advance_day`: increment day → `DayAdvanced` → scheduled disruptions/interventions/actions → treasury inflow → NPC hunger → NPC decisions → flush pending world actions → production → merchant tick → prices → bridge repair. Debug assertions on basic invariants.

## 2. NPC decisions

Sorted by `PersonId`; priority scores (travel lock, eat, steal, buy, work, migrate). Applied immediately; theft queued as `WorldAction`. Wages debit settlement `treasury_coin` (daily `population × 2` inflow — documented abstraction).

## 3. Inventory / transfers

Settlement grain via production, NPC buy (settlement−, npc.grain+), eat (npc.grain−), merchant (source− on depart, dest+ on arrive), player haul, theft via consequences. No spoilage model.

## 4. Merchant

Multi-tick `MerchantLocation::Travelling { from, to, days_remaining, carrying_grain }`. Load on depart; sale once on arrive. Return trips empty. Block before depart → cancel event; block mid-trip → ford delay only when `days_remaining <= 1` (**gap**: earlier legs of journey).

## 5. Money

Treasury wages, merchant village purchase + destination sale, NPC market purchases. Merchant coin not double-counted with settlement treasuries. Abstract tax inflow creates settlement coin (documented).

## 6. Food

NPC `NpcAte` (personal) and `NpcBoughtFood` (market purchase ≠ consumption until eaten). `run_consumption` unused in loop. Reports must not treat purchases as consumption.

## 7–8. Causality

Each `EventRecord` has `cause` + optional `parent_cause`. Bridge/repair/haul linked; merchant sale events often **lack** parent link to `MerchantDeparted` (**gap**). Repair completion parents to repair-start.

## 9. Determinism

`ChaCha8Rng` seeded but **unused** — behaviour is deterministic via sorted NPC order. `canonical_history_digest` exists.

## 10. Test coverage

Scenarios, NPC, consequences, integrity (merchant/treasury/digest). Gaps: invalid-state validator tests, grain conservation reconcile, merchant mid-travel block, snapshot equivalence, long-run multi-seed harness, expanded reports.

## Prioritised M4 plan

1. `ValidationReport` + expanded checks (no mutation).
2. Grain/money ledgers + conservation reconcile.
3. Merchant trip cause propagation + mid-trip block handling.
4. `snapshot` module + determinism/long-run tests.
5. Scenario report extensions + `docs/m4-*` deliverables.
