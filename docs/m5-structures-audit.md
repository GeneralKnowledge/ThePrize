# Simulationist structures — Phase A audit

**Branch:** `cursor/simulationist-structures-aa9f`  
**Baseline before M5:** 31 passed (`cargo test` on M4).  
**After M5:** 37 passed (`cargo test`).

## Day order (`advance_day`)

Disruptions → interventions → actions → treasury → hunger → NPC decisions → pending actions → production → merchant → prices → bridge repair.

## Direct outcome mutations (targets for generalisation)

| Location | Behaviour |
|----------|-----------|
| `consequences::damage_bridge` | Emits damage then `block_route` (instant `passable = false`) |
| `simulation::block_route` | Sets route flags directly |
| `simulation::apply_scheduled_disruptions` | Calls `block_route` on day 182 |
| `simulation::tick_bridge_repair` | Increments counter; sets `passable = true` at threshold |
| `Route.passable` | Authoritative for travel but not derived from structure |

## Strengths to preserve

Typed `WorldEvent` + `CauseId`, merchant lifecycle, invariants, snapshots, NPC decision scores, property damage pattern.

## M5 scope (implemented)

Minimal materials + bridge components + interaction resolver; derive route passability and repair from structure; wire player/scenario damage through interactions; daily load + decay; tests + invariants. See `docs/m5-interaction-model.md`.

## Highest-value follow-ups

- Persist `bridge_structure` in SQLite runs.
- NPC travel scoring already respects `passable` and `knows_bridge_unsafe`; deepen belief updates from inspection rather than global inform where possible.
- Additional interaction kinds (cutting, pulling) when tools/combat need them.
