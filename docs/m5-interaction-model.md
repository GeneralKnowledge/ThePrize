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

Typed resolution lives in `interactions::resolve_bridge` with explicit `StructureInteractionKind` and `BridgeInteractionInput` (impact target, load units, repair supplies).

| Function | Role |
|----------|------|
| `materials::impact_damage` | Material-specific loss from impact force |
| `materials::load_stress` | Per-component daily traffic stress (role-weighted; active at load 12) |
| `materials::weather_decay` | Weekly environmental wear (material-specific rates) |
| `materials::repair_restoration` | Labour + supplies → integrity gain |
| `structures::apply_impact` | Spread impact (e.g. scheduled collapse) |
| `structures::apply_targeted_impact` | Primary component + spillover to connected parts |
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

## Repair economy

Bridge repair consumes **village treasury coin**, **construction_timber**, and **construction_stone** each repair day (see `tick_bridge_repair`). Stocks are settlement inventories, not abstract resolver-only numbers.

## Snapshots and persistence

`CanonicalSnapshot.bridge_components` lists every component (id, material, role, integrity, stress) in stable order. SQLite runs include a `bridge_components` table and construction stocks on settlements.

## Not in this milestone

- Full combat / tool edge modelling beyond the typed interaction scaffold.
- Per-NPC structural inspection beyond existing bridge beliefs and rumour events.

## Tests

`tests/structures_test.rs` — materials, partial vs severe damage, repair, downstream trade, determinism.  
Existing consequence, scenario, and M4 integrity tests cover merchant and replay behaviour.
