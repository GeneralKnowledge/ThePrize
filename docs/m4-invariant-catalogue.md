# M4 invariant catalogue

Validation entry point: `validate_world(world, log) -> ValidationReport` (read-only).

## Errors (fatal)

| Code | Meaning |
|------|---------|
| `duplicate_npc_id` | Two NPCs share a `PersonId` |
| `merchant_npc_id_collision` | Merchant id collides with an NPC |
| `duplicate_settlement_id` | Duplicate settlement id |
| `population_mismatch` | Settlement `population` ≠ alive NPCs with that `home` |
| `merchant_over_capacity` | In-transit cargo exceeds `grain_capacity` |
| `merchant_zero_travel_days` | Travelling with `days_remaining == 0` |
| `merchant_invalid_trip` | `from == to` while travelling |
| `route_repair_passable` | Route passable while `repair_in_progress` |
| `route_repair_overflow` | Repair progress beyond required days while still blocked |
| `npc_hunger_range` | Alive NPC hunger outside [0, 1] |
| `orphan_parent_cause` | Event references unknown `parent_cause` |
| `self_parent_cause` | Event parents to its own cause id |
| `double_consume` | Aggregate `GoodsConsumed` events in log (legacy path) |

## Warnings (suspicious)

| Code | Meaning |
|------|---------|
| `grain_suspiciously_high` | Settlement grain > 10M |
| `npc_coin_high` / `merchant_coin_high` | Coin > 1M |
| `merchant_transfer_sale_mismatch` | Count of transfers ≠ sales |
| `sale_missing_trip_cause` | Sale without parent trip cause |

## Accounting rules (reported, not all validated each tick)

- **Food:** only `NpcAte` counts as consumption; `NpcBoughtFood` moves market stock to personal inventory.
- **Grain conservation (valley-wide):** `opening + produced ≈ closing + eaten` (see `reconcile_grain_conservation`).
- **Money:** wages debit settlement treasury (`WagePaidFromTreasury`); daily `population × 2` treasury inflow (abstract tax); merchant pays village on load, receives payment once on sale.
