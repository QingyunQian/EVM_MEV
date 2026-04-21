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

## Scope notes

This is explicitly **not** a production searcher. We do not touch mempool
monitoring, Flashbots bundles, multi-hop routing, low-latency networking,
or any real-network execution. The value of the project is the clean
mechanism analysis and the defense discussion in
[`docs/defense.md`](docs/defense.md).
