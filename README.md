# Rust Sandwich MEV Simulator

A teaching/research prototype for FTEC 5320 (Decentralized Finance). The
project studies sandwich MEV in a Uniswap-V2-style constant-product AMM: a
Rust simulator finds the attacker's optimal front-run size, sweeps the
outcome over several variables, and a Foundry test cross-checks the numbers
against a real EVM execution of the same pool.

## Layout

```
final_project/
  searcher/     Rust simulator + optimizer + experiment runner
  contracts/    Foundry project with MiniAMM and on-chain sandwich test
  analysis/     Python plotting script (consumes the Rust CSVs)
  data/         CSV output from the Rust sweeps
  figures/      PNG plots generated from the CSVs
  docs/         Mechanism notes and defense discussion
```

## Reproducing everything

```bash
# 1. Rust core: unit tests and a single-scenario demo
cd searcher
cargo test --release
cargo run --release -- simulate --victim 1000 --slippage 0.01
cargo run --release -- trace --victim 1000 --slippage 0.01

# 2. Parametric sweeps -> CSV
cargo run --release -- sweep --out-dir ../data

# 3. Plots (matplotlib + pandas)
cd ../analysis
pip install -r requirements.txt
python plot.py --data ../data --figures ../figures

# 4. On-chain cross-check (Foundry)
cd ../contracts
forge install foundry-rs/forge-std   # first time only
forge test -vv
```

## Headline result

For a 100k / 100k pool with a 0.30% fee and a victim swapping 1,000 X with
1% slippage tolerance:

| Quantity                   | Value       |
| -------------------------- | ----------- |
| Optimal attacker size `a`  | 507.045 X   |
| Attacker profit            | 7.016 X     |
| Attacker ROI               | ~1.38%      |
| Victim honest output       | 987.158 Y   |
| Victim actual output       | 977.286 Y   |
| Victim extra loss          | 9.872 Y     |

The extra loss is essentially exactly at the victim's 1% slippage cap,
confirming that a rational attacker pushes all the way to the constraint.
The on-chain Foundry test reproduces the same profit to within 1%.

## Classroom demo path

For a live lab, use [`docs/lab_walkthrough.md`](docs/lab_walkthrough.md) as
the teaching script. The most useful demo command is:

```bash
cd searcher
cargo run --release -- trace --victim 1000 --slippage 0.01
```

It prints the ordered sandwich sequence as pool states:

1. Initial AMM reserves and honest quote.
2. Attacker front-run that moves the price.
3. Victim swap that still passes `min_out`.
4. Attacker back-run that realizes the profit.

To show a failed over-sized attack, fix the attacker amount manually:

```bash
cargo run --release -- simulate --victim 1000 --slippage 0.01 --attacker 2000
```

The sweep also generates `sweep_attacker_size.csv`, and the plotting script
renders `fig_attacker_size.png`, which is the clearest figure for explaining
why the optimal attacker size sits near the victim's slippage boundary.

## Defense discussion for the lab

The attack works because the victim's swap is visible before execution, and
because the victim gives the transaction enough slippage tolerance to remain
valid after the attacker moves the price. The final part of the lab should
connect each defense back to one of those two assumptions.

| Defense | What it changes | Lab takeaway |
| ------- | --------------- | ------------ |
| Lower slippage tolerance | Shrinks the feasible range for the attacker's front-run size | The attacker cannot push the victim as far before the transaction reverts. |
| Deeper liquidity pool | Reduces price impact for the same victim trade | The same sandwich produces less price movement and lower profit. |
| Higher fee tier | Makes the attacker pay more on the front-run and back-run | Once round-trip fees exceed the available slippage, the sandwich becomes unprofitable. |
| Private transaction route | Hides the victim trade from the public mempool | If the attacker cannot see the trade in advance, they cannot reliably place the front-run. |
| Batch auction or intent-based venue | Removes strict per-transaction ordering around a public AMM swap | The attacker no longer gets the ordered three-step sequence needed for a sandwich. |

A useful classroom prompt is to ask students which variable they would change
first if they were the victim. In this simulator, lowering slippage is the
most direct user-controlled defense: it narrows the attacker's feasible region
and visibly reduces both attacker profit and victim extra loss. For larger
trades, the more structural defenses are deeper liquidity, private routing,
and auction-style execution.

## Scope notes

This is explicitly **not** a production searcher. We do not touch mempool
monitoring, Flashbots bundles, multi-hop routing, low-latency networking,
or any real-network execution. The value of the project is the clean
mechanism analysis and the defense discussion in
[`docs/defense.md`](docs/defense.md).
