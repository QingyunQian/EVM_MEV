# Sandwich MEV Lab Walkthrough

This walkthrough is designed for a classroom demo. The goal is to make the
attack visible as an ordered state transition, not just as a profit number.

## 1. Start with an honest AMM swap

Use the reference pool:

- Pool reserves: `100,000 X / 100,000 Y`
- Fee: `0.30%`
- Victim swap: `1,000 X -> Y`
- Victim slippage tolerance: `1%`

The honest quote is about `987.158 Y`, so the victim's transaction allows a
minimum output near `977.286 Y`.

Classroom point: slippage tolerance is not just a UI setting. It is the range
inside which an adversary can move the execution price without reverting the
victim's transaction.

## 2. Show the ordered sandwich trace

Run:

```bash
cd searcher
cargo run --release -- trace --victim 1000 --slippage 0.01
```

The trace prints a table with four states:

| Step | Meaning |
| ---- | ------- |
| `0` | Initial pool before the attacker acts |
| `1` | Attacker front-runs by swapping X for Y |
| `2` | Victim swaps X for Y against the worse price |
| `3` | Attacker back-runs by swapping Y back to X |

The important visual pattern is:

```text
price before < price after front-run < price after victim
```

The attacker buys before the victim pushes price further, then sells back after
the victim has moved the pool.

## 3. Compare success and failure

Successful sandwich near the reference setting:

```bash
cargo run --release -- simulate --victim 1000 --slippage 0.01
```

Over-sized front-run that makes the victim revert:

```bash
cargo run --release -- simulate --victim 1000 --slippage 0.01 --attacker 2000
```

Classroom point: the attacker is constrained. A larger front-run is not always
better, because pushing too far makes the victim transaction fail. The optimal
attack sits near the victim's `min_out` boundary.

## 4. Generate the plots

Run:

```bash
cargo run --release -- sweep --out-dir ../data
cd ../analysis
python plot.py --data ../data --figures ../figures
```

Use the figures in this order:

1. `fig_attacker_size.png`: explains why the optimizer picks a specific
   front-run size.
2. `fig_slippage.png`: shows why looser slippage creates more extractable
   value.
3. `fig_pool_depth.png`: shows why deeper pools reduce price impact.
4. `fig_fee.png`: shows why higher fees can eliminate the attacker's edge.
5. `fig_victim_size.png`: shows how larger victim trades increase the attack
   surface.

## 5. Verify with local EVM execution

Run:

```bash
cd contracts
forge test -vv
```

The Solidity test executes the same sequence on a local EVM:

```text
attacker front-run -> victim swap -> attacker back-run
```

The printed values show the honest quote, victim `minOut`, front-run output,
victim actual output, back-run output, attacker profit, and victim extra loss.

## 6. End with defenses

Connect every defense to the assumption it breaks:

| Defense | Broken assumption |
| ------- | ----------------- |
| Lower slippage | The attacker has enough room to move price |
| Deeper liquidity | The attacker's trade has large price impact |
| Higher fee tier | Round-trip attack cost is small |
| Private transaction route | The victim swap is visible before execution |
| Batch auction / intent-based venue | The attacker can force a three-transaction ordering |

The final takeaway is that sandwich MEV is an ordering attack plus a slippage
constraint. Good defenses either hide the order, reduce the price impact, or
remove the attacker's feasible profit window.
