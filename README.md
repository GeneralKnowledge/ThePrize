# The Living World (prototype)

Simulation-first medieval sandbox RPG — **headless valley experiment** (M1–M3).

Two settlements (**Hollowbrook** village, **Millford** market town) share one economy, one bridge, and one travelling merchant (**Corvin the Carter**). **40 NPCs** (24 + 16) have jobs, hunger, personal stores, goals, beliefs, and **explainable daily decisions** (eat, buy food, work, travel). The simulation advances one day at a time with typed `WorldEvent` records, `CauseId` links, and SQLite run archives.

## Run scenarios

```bash
cargo run --release
```

Runs four one-year simulations (seed 42 by default):

| Scenario | Description |
|----------|-------------|
| **Baseline** | Normal operations |
| **Disruption** | Bridge blocked mid-year (day 182) |
| **InterventionRepair** | Block + player-funded bridge repair |
| **InterventionHaul** | Block + weekly alternative grain hauls |

SQLite archives land in `runs/` (world snapshot + full event log).

```bash
cargo run --release -- --seed 7 --scenario disruption
```

## Architecture

- `living_world` library: world state, economy, NPC decisions, merchant travel lifecycle, event log, headless simulator (no Bevy)
- **Consumption:** NPC personal eating and market purchases only — no parallel aggregate settlement consumption in the daily loop
- **Wages:** Paid from each settlement’s `treasury_coin`, replenished daily by `population × 2` (abstract tax/tithing — not coin minted on workers)
- **Merchant:** Grain leaves the source on `MerchantDeparted`; destination inventory updates on `MerchantArrived` with a single `MerchantSaleCompleted` payment
- **Causality:** `history::ancestors_of`, `descendants_of`, `trace_from_cause`, `canonical_history_digest`
- **Invariants:** `invariants::check_world_invariants` (debug builds assert each day)

## Tests

```bash
cargo test
```

Covers scenario acceptance, NPC behaviour, consequence actions, merchant integrity, treasury wages, and deterministic history digests.

## M3 consequences

Player/world actions go through `apply_world_action` (`WorldAction`: bridge/property damage, theft, kill). Properties can change hands on death; guards and witnesses get memories. Bridge block emits rumour events linked to the block cause for traceability.

## Roadmap

M4: Bevy playable slice. M5: rumour discovery and objective history UI.
