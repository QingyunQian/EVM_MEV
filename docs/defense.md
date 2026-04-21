# Defenses Against Sandwich MEV

The simulator makes two things quantitative that are often only discussed
qualitatively: what the *victim* pays, and how sensitive that payment is to
each defense. Each defense below is paired with the evidence from the
sweeps.

## 1. Tighter slippage tolerance

The clearest dial the user controls. In `data/sweep_slippage.csv`, attacker
profit and victim extra loss both scale roughly linearly with `s` in the
interior of the feasible interval. Once `s` drops below the round-trip fee
cost (twice the pool fee `f`), the attack flips infeasible and the victim
receives the honest quote.

Recommendation: wallets should default slippage to the smallest value at
which honest swaps succeed, and expose aggressive user-selectable presets.
Raising slippage to "make the swap go through" is precisely what the
attacker is waiting for.

## 2. Pool depth and fee tier selection

Deeper pools dilute the attacker's per-unit price impact, and higher fee
tiers widen the round-trip cost the attacker must overcome. The fee sweep
in `data/sweep_fee.csv` shows the critical-threshold behavior: at the
reference scenario (1% victim slippage, 0.3% fee), profit is ~7 X; raise
the fee to 1% and profit vanishes entirely.

Recommendation: for volatile or thinly-traded pairs, picking the 1% fee
tier instead of 0.05% is a directly observable sandwich deterrent, at the
obvious cost of paying more on normal trades.

## 3. Private transaction channels

Public mempools expose the victim's intent; private relays (MEV-Share,
Flashbots Protect, private RPC endpoints) remove the front-running
opportunity at the source.

The simulator does not model networking, but the implication is in the
assumptions: the strategy in `src/strategy.rs` assumes the victim's
`(v, slippage)` pair is observable before the front-run is submitted. Take
that assumption away and `profit(a)` degenerates into a blind bet that
would typically revert.

Recommendation: this is the strongest technical mitigation currently
available to end users. Downsides: reliance on centralized relays and
occasional inclusion latency.

## 4. Batch auctions and uniform pricing

CoW Swap, 1inch Fusion, and similar designs settle trades in periodic
batches at a single clearing price. Because the attacker cannot reorder
within the batch and cannot front-run the clearing price, sandwich
profit collapses structurally, not just quantitatively.

The simulator captures why: sandwich profit requires *three* ordered
trades touching the same state. Batch auctions collapse all trades in
a window into one state transition, removing the ordering degree of
freedom.

## 5. Intent-based execution

Intent systems (UniswapX, Across, etc.) let users specify the outcome they
want, and competing solvers bid to fill it. The user's order is not a live
AMM transaction, so there is no "victim swap" to sandwich in the mempool
sense. Bad solvers can still extract some value, but the extraction surface
is different and is bounded by solver competition.

## 6. AMM-level mitigations (research)

- **Dynamic fees** that rise with recent volatility (see Uniswap V3 hooks,
  or the dynamic-fee literature) can close the fee-threshold gap shown in
  the sweep, making sandwiches unprofitable exactly when they are most
  tempting.
- **Toxicity-aware LP curves** (CoW AMM, FM-AMM) price in adverse selection
  explicitly and capture some of the MEV for LPs rather than searchers.
- **Commit-reveal or threshold-encrypted mempools** aim to prevent the
  adversary from knowing `(v, slippage)` at all, directly nuking the
  optimization the Rust code performs.

## A practical ranking for the course report

For a typical DEX user today, ordered from cheapest to most structural:

1. Lower slippage tolerance.
2. Prefer deeper pools / higher fee tiers for volatile pairs.
3. Use a private-transaction route.
4. Prefer intent-based or batch-auction venues for large trades.

For protocol designers, the meaningful work is (4) onward: the user-level
mitigations are a floor, not a ceiling.
