# The Living World (prototype)

Simulation-first medieval sandbox RPG — **headless valley experiment** (M1 foundation).

Two settlements (**Hollowbrook** village, **Millford** market town) share one economy, one bridge, and one travelling merchant. **40 NPCs** (24 + 16) have jobs, hunger, personal stores, goals, beliefs, and **explainable daily decisions** (eat, buy food, work, travel). The simulation advances one day at a time with typed world events and causal links.

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

- `living_world` library: world state, economy, event log, headless simulator (no Bevy yet)
- Typed `WorldEvent` records; systems react on scheduled ticks (daily production, consumption, prices, travel)
- Persistence at run boundaries via SQLite (simulation stays in memory)

## Tests

```bash
cargo test
```

Acceptance checks verify that disruption and interventions produce distinguishable, explainable outcomes versus baseline.

## M3 consequences

Actions go through `apply_world_action` (`WorldAction`: bridge/property damage, theft, kill). Properties can change hands on death; guards and witnesses get memories. Use `history::trace_from_cause` to inspect chains.

## Roadmap

M4: Bevy playable slice. M5: rumour discovery and objective history UI.
