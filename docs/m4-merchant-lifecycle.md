# Merchant journey lifecycle (M4)

## States

`MerchantLocation`:

- **At(settlement)** — idle / evaluating next trip at current settlement.
- **Travelling { from, to, days_remaining, carrying_grain }** — multi-tick movement; cargo in transit.

`active_trip_cause` links `MerchantDeparted` to arrival, transfer, and sale events via `parent_cause`.

## Transitions

1. **Evaluate** (while At village): town stock low, village export surplus, bridge passable, load > 0, merchant coin ≥ purchase cost.
2. **Depart:** deduct village grain and merchant coin; set travelling; emit `MerchantDeparted { grain, coin_spent }`; store trip cause.
3. **Travel:** decrement `days_remaining` each day until 1, then arrive.
4. **Arrive:** credit destination grain; single `MerchantSaleCompleted`; emit `GoodsTransferred` with parent trip cause; clear trip cause.
5. **Return:** from town, empty return trip (1 day) when imports still needed.
6. **Cancel:** bridge closed before depart, or insufficient coin → `MerchantTripCancelled`.
7. **Disruption mid-trip:** `on_route_blocked` extends journey to ford days when bridge closes on a village↔town leg.

## Not modelled

- Multi-merchant convoys, cargo reservations separate from in-transit grain, persistence of in-progress trips in SQLite (world snapshot omits merchant travel detail).
