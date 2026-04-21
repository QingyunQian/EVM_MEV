# Sandwich Mechanism in a Constant-Product AMM

This note derives the attacker's payoff in closed form and states the
optimization problem the Rust simulator solves numerically.

## Pool primitives

The pool holds reserves `(x, y)` of tokens X and Y. Each swap applies a
linear fee `f` to the *input* side. Given input `dx` of X, the amount of Y
received is

```
dy(dx; x, y) = y * (dx * (1 - f)) / (x + dx * (1 - f))
```

and the new reserves are `(x + dx, y - dy)`. The symmetric formula applies
to swapping Y for X.

## Honest victim quote

The victim wants to swap `v` units of X for Y and configures a slippage
tolerance `s`. Their frontend quotes the honest output

```
honest_out = dy(v; x0, y0)
```

and sets `min_out = honest_out * (1 - s)` as the revert threshold.

## Sandwich sequence

An attacker picks a front-run size `a` and performs three swaps:

1. Front-run: swap `a` X for Y against `(x0, y0)`.
   Receives `fa = dy(a; x0, y0)` and leaves the pool at `(x1, y1) =
   (x0 + a, y0 - fa)`.
2. Victim: swap `v` X for Y against `(x1, y1)`. Receives `va = dy(v; x1, y1)`.
   Transaction reverts if `va < min_out`, in which case the attacker keeps
   their Y but no sandwich happened (loss = trading fees on the front-run).
3. Back-run: swap the entire `fa` Y for X against the post-victim pool
   `(x2, y2) = (x1 + v, y1 - va)`. Receives `ba = dx(fa; x2, y2)` using the
   Y-for-X version of the swap formula.

## Attacker payoff

Net profit in X is

```
profit(a) = ba - a
         = dx(fa; x0 + a + v, y0 - fa - va) - a
```

subject to the feasibility constraint `va >= min_out` (otherwise the victim
reverts and the attacker is left holding non-back-runnable Y).

On a single CPMM pool, `profit(a)` is unimodal over the feasible interval:
larger `a` keeps pushing the price until the victim would revert, and past
that point feasibility is gone. The Rust code handles this as:

1. Binary-search the largest feasible `a_max` such that the victim survives.
2. Golden-section search `[0, a_max]` to maximize `profit`.

## Structural observations

Three facts fall out of the algebra and are visible in the sweeps:

- **Linear scaling with victim size.** For fixed pool depth and slippage,
  both the optimal `a` and the profit grow roughly linearly with `v` in the
  small-`v` regime, then bend as `v` becomes non-negligible versus `x0`.

- **Slippage tolerance is a linear dial on profit.** A tighter `s`
  directly shrinks the feasible interval, hence the profit. At `s = 0` the
  attack is infeasible.

- **Fees are a threshold.** The fee is paid on both legs of the attack;
  once `f` exceeds roughly the victim's slippage tolerance, the fee cost
  exceeds the price-impact arbitrage and the optimal `a` collapses to zero.
  This is exactly what the fee sweep shows, with profit dropping to zero at
  `f = 1%` while victim slippage is 1%.

- **Depth dilutes everything.** The attack is local-impact arbitrage, so
  fixing `v` and growing pool depth makes the spread the attacker can
  capture shrink as `O(1 / sqrt(depth))` asymptotically.

## What this does not cover

- Gas cost for the attacker. Including gas moves the profitability
  frontier inward but does not change the qualitative picture.
- Priority-fee auctions / block-space competition.
- Multi-pool routing, where the attacker can split across paths.
- Stateful adversaries that adapt to the victim's slippage distribution.

These are worthwhile extensions but out of scope for the course project.
