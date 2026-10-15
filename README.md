# The Living World (prototype)

Simulation-first medieval sandbox RPG — **headless valley experiment** (M1 foundation).

Two settlements (**Hollowbrook** village, **Millford** market town) share one economy, one bridge, and one travelling merchant. The simulation advances one day at a time with typed world events and causal links.

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

## Roadmap

See design doc in project issue / GDD: next steps are autonomous NPC decisions (M2), full consequence chains (M3), then Bevy client (M4).
