# M3 audit (Phase 0)

## Confirmed defects

1. **Merchant instant transfer** — `tick_merchant` deducts village grain and credits town on the same day without travel; payment happens before arrival.
2. **Duplicate delivery path** — travelling arrival adds grain; load branch also adds to town (double-count risk).
3. **Wage minting** — `NpcWorked` increases NPC coin without settlement treasury debit (implicit money creation).
4. **Aggregate consumption** — `run_consumption` remains in codebase but is unused; metrics must not double-count with NPC eat/buy.
5. **Causal graph** — only upward trace helper; no descendant query or canonical history digest for regression.
6. **Route block mid-trip** — no adjustment when bridge closes while merchant is in transit.

## Architectural risks

- Pending NPC actions flush before production; order is acceptable but theft can affect same-day production.
- Repair completion mints a new cause without linking to repair-start cause.

## Smallest safe changes

- Extract merchant lifecycle; fix load/travel/deliver/pay-once.
- Settlement `treasury_coin` for wages.
- `invariants` module + tests.
- `ledger` grain conservation from events.
- Extend `history` (ancestors, descendants, digest).
- Richer scenario reports; NPC market/route adaptation scores.
