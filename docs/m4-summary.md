# M4 deliverables summary

## Audit (Phase 0)

See `docs/m4-audit-plan.md`. Baseline before edits: **19 tests passed** on `main`.

## Implementation highlights

| Area | Files |
|------|--------|
| Validation report | `src/invariants.rs` |
| Grain/money accounting | `src/ledger.rs` |
| Canonical snapshots | `src/snapshot.rs` |
| Merchant causality + mid-trip block | `src/merchant.rs`, `src/world.rs`, `src/events.rs` |
| Cause graph helper | `src/history.rs` (`validate_cause_graph`) |
| Scenario reporting | `src/scenarios.rs`, `src/main.rs` |
| Tests | `tests/m4_*.rs`, invariant unit tests in `invariants.rs` |

## Tests (executed)

```bash
cargo test
```

**31 passed** (including 2 lib unit, 4 consequences, 6 integrity, 2 determinism, 3 long-run, 3 merchant, 4 npc, 5 scenarios).

Long-run: 400 days × 5 seeds × baseline + disruption; integrity check every 50 days; grain imbalance tolerance ≤ 5.

## Example scenario

```bash
cargo run --release -- --seed 42 --scenario disruption
```

## Known limitations

- No grain spoilage; abstract treasury tax inflow; player haul is instant transfer; SQLite omits merchant in-transit state; RNG seeded but unused (deterministic NPC ordering).
- `inform_bridge_blocked` still sets beliefs directly (global inform) — limited rumour events added at block time.

## Recommended next milestone (M5)

- Replace global bridge inform with rumour propagation and NPC-local knowledge only.
- Persist merchant journey + pending delayed actions in SQLite.
- Player character as `Actor` in daily tick without new renderer requirements.
