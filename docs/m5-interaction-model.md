# M5 — Materials, structures, and interactions

## Overview

Stoneford Bridge is modelled as a **structure** with four **components** (deck, two piers, east truss), each with a **material** (`Oak`, `Limestone`, `Iron`) and **integrity** (0–1000). Route `passable` is **derived** from structural crossing safety, not set directly by damage actions.

## Authoritative vs derived

| Authoritative | Derived |
|---------------|---------|
| Component integrity, accumulated stress | Aggregate integrity (weighted) |
| Material id per component | Crossing safety (`Safe` … `Collapsed`) |
| Route admin flags (`repair_in_progress`) | `bridge_passable()` → route `passable` |

## Interaction entry points

| Function | Role |
|----------|------|
| `materials::impact_damage` | Material-specific loss from impact force |
| `materials::load_stress` | Daily traffic load → stress → integrity loss |
| `materials::weather_decay` | Weekly environmental wear |
| `materials::repair_restoration` | Labour + supplies → integrity gain |
| `structures::apply_impact` | Deliberate / catastrophic damage |
| `structures::apply_sustained_load` | Ordinary bridge traffic while open |
| `structures::apply_weather` | Periodic decay |
| `structures::apply_repair` | Funded repair ticks |

Forces (tuned to the simplified formulas):

- Player `DamageBridge`: impact **880**
- NPC sabotage: **280** (partial)
- Scheduled disruption (day 182): **920**
- Legacy `block_route`: **900**

## Crossing safety thresholds

Integrity is on a 0–1000 scale. Safety uses the minimum of support piers and deck plus weighted aggregate integrity. **Restricted** bridges remain crossable; **Unsafe** and **Collapsed** block the route.

## World integration

1. Interaction mutates `WorldState.bridge_structure`.
2. `structure_sync::record_structure_outcome` emits `StructuralConditionChanged` / `ComponentIntegrityChanged`.
3. `structure_sync::sync_bridge_route` updates route passability and emits `RouteBlocked` or `RouteRepaired` on transitions only.

## Assumptions (not physical simulation)

- Discrete daily steps; no continuous physics.
- Integer integrity; material stats are relative 1–100 scales.
- Same rules for player and NPC damage paths where both use `apply_impact`.

## Not in this milestone

- SQLite persistence of component-level bridge state (snapshots/digests include aggregate safety).
- Full combat / tool edge modelling.
- Per-NPC structural inspection beyond existing bridge beliefs and rumour events.

## Tests

`tests/structures_test.rs` — materials, partial vs severe damage, repair, downstream trade, determinism.  
Existing consequence, scenario, and M4 integrity tests cover merchant and replay behaviour.
