# Sandwich MEV Classroom Demo

This repository is a teaching demo for sandwich MEV on a Uniswap-V2-style
constant-product AMM. It shows the mechanism in three layers:

- a Rust simulator that computes and traces the optimal sandwich;
- a Python analysis script that turns parameter sweeps into figures;
- a Foundry project that reproduces the same attack on a local EVM.

It is **not** a production MEV searcher. It does not monitor a real mempool,
send Flashbots bundles, compete in priority-fee auctions, or execute against
mainnet liquidity. Its purpose is to make the mechanism visible enough for a
30-minute classroom presentation.

## One-Screen Intuition

A sandwich attack needs a visible victim order, a pool whose price moves when
someone trades, and enough victim slippage for the victim transaction to remain
valid after the attacker moves the price.

```mermaid
sequenceDiagram
    participant A as Attacker
    participant P as AMM Pool
    participant V as Victim

    A->>P: 1. front-run: swap X for Y
    Note over P: price moves against victim
    V->>P: 2. victim swap: X for Y
    Note over V,P: transaction still passes minOut
    A->>P: 3. back-run: swap Y back for X
    Note over A: attacker keeps the price-impact spread
```

The AMM uses the constant-product rule:

```text
x * y = k
```

For a swap from token X into token Y, the input increases `x`, the output
decreases `y`, and the pool price `y / x` moves. The victim protects their
trade with:

```text
minOut = honestQuote * (1 - slippageTolerance)
```

The attacker chooses a front-run size that pushes the victim close to `minOut`
without crossing it. If the attacker pushes too hard, the victim transaction
reverts and the sandwich fails.

## What Is Already Implemented

| Area | Implemented content | Demo value |
| ---- | ------------------- | ---------- |
| Rust simulator | CPMM math, victim slippage, fixed-size sandwich simulation, optimal attacker-size search, failure unwind, gas/priority-fee accounting, multi-hop route comparison, bundle/order comparison, CLI commands | Shows the mechanism, optimal trade size, executable profit after gas, and why routing/order placement changes MEV feasibility. |
| Rust trace | `trace` command prints the ordered pool states | Best live demo for explaining the three-transaction sequence. |
| Rust sweeps | Victim size, slippage, pool depth, fee, attacker size, gas cost, defense comparison | Produces the data behind the classroom figures. |
| Python plots | Seven PNG figures generated from sweep CSVs | Converts the attack into visual stories. |
| Solidity contracts | `MiniAMM` and `MockERC20` | Small EVM version of the same AMM. |
| Foundry tests | Honest swap test, profitable sandwich cross-check, oversized revert/unwind test | Confirms both the successful attack and the failed over-sized attack against local EVM execution. |
| Docs | Mechanism notes, defense discussion, classroom walkthrough, update log | Supporting material for a course report or presentation. |

Repository layout:

```text
EVM_MEV/
  searcher/     Rust simulator, optimizer, trace command, sweep runner
  contracts/    Foundry project with MiniAMM, mock tokens, tests, demo scripts
  analysis/     Python plotting script
  data/         Generated CSV sweep outputs
  figures/      Generated PNG figures
  dashboard/    Static browser dashboard for interactive visualization
  docs/         Mechanism notes, defense discussion, walkthrough, updates
```

## Reference Scenario

Reference pool and victim settings:

| Parameter | Value |
| --------- | ----- |
| Pool reserves | `100,000 X / 100,000 Y` |
| AMM fee | `0.30%` |
| Victim swap | `1,000 X -> Y` |
| Victim slippage | `1%` |

The optimized sandwich result is:

| Quantity | Value |
| -------- | ----- |
| Optimal attacker front-run `a` | `507.044775 X` |
| Attacker front-run output | `502.980953 Y` |
| Attacker back-run output | `514.061023 X` |
| Attacker profit | `7.016249 X` |
| Attacker ROI | `1.3838%` |
| Victim honest output | `987.158034 Y` |
| Victim actual output | `977.286454 Y` |
| Victim extra loss | `9.871580 Y` |
| Gas cost | `0 X` by default, configurable in CLI/dashboard |
| Net executable profit | `7.016249 X` before gas costs |

The victim's extra loss is almost exactly the 1% slippage budget. That is the
main lesson: loose slippage creates a feasible profit window, and the rational
attacker pushes to the edge of that window.

## Pool-State Visualization

The `trace` command prints the same sequence as a state table.

```bash
cd searcher
cargo run --release -- trace --victim 1000 --slippage 0.01
```

Expected reference states:

| Step | Actor | Action | Reserve X | Reserve Y | Price `Y/X` | Why it matters |
| ---- | ----- | ------ | --------- | --------- | ----------- | -------------- |
| 0 | - | Initial pool | `100000.000000` | `100000.000000` | `1.000000` | Victim frontend quotes the honest swap here. |
| 1 | Attacker | Front-run X -> Y | `100507.044775` | `99497.019047` | `0.989951` | The attacker moves price against the victim. |
| 2 | Victim | Swap X -> Y | `101507.044775` | `98519.732593` | `0.970570` | Victim receives only `977.286454 Y`, still just above `minOut`. |
| 3 | Attacker | Back-run Y -> X | `100992.983751` | `99022.713546` | `0.980491` | Attacker exits back to X and realizes profit. |

```mermaid
stateDiagram-v2
    [*] --> Initial: reserves 100k / 100k
    Initial --> FrontRun: attacker swaps 507.044775 X
    FrontRun --> VictimSwap: victim swaps 1000 X
    VictimSwap --> BackRun: attacker swaps received Y back
    BackRun --> [*]: attacker profit 7.016249 X
```

To show that "bigger attack" is not always better, force an oversized
front-run:

```bash
cd searcher
cargo run --release -- simulate --victim 1000 --slippage 0.01 --attacker 2000
```

This demonstrates the revert boundary: once the victim output falls below
`minOut`, the victim does not execute, and the attacker must unwind the failed
front-run.

To show why theoretical MEV is not the same as executable profit, add a gas
model. The output prints both gross profit and net profit:

```bash
cd searcher
cargo run --release -- simulate \
  --victim 1000 \
  --slippage 0.01 \
  --gas-units 500000 \
  --base-fee-gwei 25 \
  --priority-fee-gwei 2 \
  --native-price-x 1
```

Gas cost is modeled as:

```text
gas_cost_x = gas_units * (base_fee_gwei + priority_fee_gwei) * 1e-9 * native_price_x
net_profit_x = gross_profit_x - gas_cost_x
```

The model is intentionally simple: gas is treated as a fixed cost for the
bundle, and `native_price_x` converts the gas token into token X units. When
`--attacker` is omitted, the Rust CLI treats gas as a hurdle and returns
`attacker_in = 0` if the best gross sandwich would be net negative after gas.
When `--attacker` is supplied manually, the CLI prints that fixed attack's net
profit or loss.

## Figures To Show In Class

The figures are generated outputs, not hand-drawn slides. If the `figures/`
directory is missing, regenerate it with the commands in the next section.

| Figure | What to point at | Classroom takeaway |
| ------ | ---------------- | ------------------ |
| `figures/fig_attacker_size.png` | Profit curve peaks just before victim output crosses `minOut` | The attacker is constrained by slippage; the optimum is near the revert boundary. |
| `figures/fig_slippage.png` | Profit and victim extra loss rise as slippage rises | Slippage is not only a UX setting; it is the attacker's feasible window. |
| `figures/fig_pool_depth.png` | Profit shrinks in deeper pools | Larger reserves dilute price impact. |
| `figures/fig_fee.png` | Profit collapses when fee becomes high enough | The attacker pays fees twice, on front-run and back-run. |
| `figures/fig_victim_size.png` | Bigger victim trades create larger opportunities | Large visible swaps are more attractive targets. |
| `figures/fig_gas.png` | Gross profit can stay positive while net profit disappears | Gas and priority fees decide whether theoretical MEV is executable. |
| `figures/fig_defense.png` | Mitigations compared side by side | Lower slippage, deeper pools, higher fees, gas costs, and private routing shrink or remove the attack window. |

The two figures below are the best ones to put directly on screen during the
main explanation.

![Attacker size frontier](figures/fig_attacker_size.png)

![Slippage effect](figures/fig_slippage.png)

## Interactive Dashboard

Open the static dashboard in a browser:

```text
dashboard/index.html
```

If typing `dashboard/index.html` into the browser address bar opens a blank or
missing page, open it from the repository root instead:

```bash
python3 -m http.server 8000
```

Then visit:

```text
http://127.0.0.1:8000/dashboard/
```

The dashboard is an interactive explanation tool, not a separate backend app.
It recomputes the same CPMM sandwich model in JavaScript and shows:

- saved scenario presets for reference, high gas, deep pool, and oversized attack;
- optimal attacker size or a manually selected attacker size;
- gross profit, gas cost, net profit, ROI, victim output, `minOut`, and revert status;
- the attacker-size frontier with the victim `minOut` line;
- the three-step pool state after front-run, victim swap, and back-run;
- the pool price path and step-candle chart with MEV buy/sell markers for explaining price movement.

Use it after the Rust trace when the audience understands the basic sequence:
move one parameter at a time, then connect the curve movement back to the
slippage and price-impact story.

Recommended live-demo setup:

1. Keep this README open for the diagrams and speaking script.
2. Keep a terminal open in `searcher/` for `trace`, `simulate`, and `sweep`.
3. Keep `dashboard/index.html` open in a browser for parameter Q&A.
4. Keep a second terminal open in `contracts/` for `forge test -vv --offline`.

During Q&A, use the dashboard in this order:

1. Click "Reference" and connect the dashboard values to the Rust trace.
2. Click "High gas" and show gross profit versus net profit.
3. Click "Deep pool" and point out that the same victim trade moves price less.
4. Click "Oversized" and show victim revert plus attacker unwind loss.
5. Manually increase slippage and point out that the feasible attacker window expands.
6. Increase the fee and point out that the attacker pays fees on both legs.
7. Disable "Use optimal attacker size", drag attacker size too far, and show the
   victim revert status.

## Reproduce The Demo

Rust tests and reference trace:

```bash
cd searcher
cargo test --release
cargo run --release -- simulate --victim 1000 --slippage 0.01
cargo run --release -- trace --victim 1000 --slippage 0.01
cargo run --release -- simulate --victim 1000 --slippage 0.01 --gas-units 500000 --base-fee-gwei 25 --priority-fee-gwei 2 --native-price-x 1
```

Generate CSV sweeps:

```bash
cd searcher
cargo run --release -- sweep --out-dir ../data
cargo run --release -- defense --out-dir ../data
```

Run the route and bundle/order demos:

```bash
cd searcher
cargo run --release -- route
cargo run --release -- bundle
```

`route` compares the reference direct `X -> Y` pool with a two-hop
`X -> M -> Y` route where the attacker sandwiches the first hop and the victim
checks `minOut` on final `Y` output. `bundle` compares honest execution,
profitable sandwich ordering, oversized front-run with unwind, and victim-first
ordering.

Render figures:

```bash
cd analysis
pip install -r requirements.txt
python plot.py --data ../data --figures ../figures
```

Run the EVM cross-check:

```bash
cd contracts
# First time only, if contracts/lib/forge-std is missing:
# forge install foundry-rs/forge-std
forge test -vv --offline
```

The `--offline` flag avoids Foundry's optional online signature lookup. In this
environment, plain `forge test -vv` can compile successfully and then fail in
Foundry's network/proxy path; the offline command is the stable classroom
version.

## Repository Architecture

The repo is organized as a small teaching pipeline: the Rust simulator owns the
reference sandwich model, generated CSVs and figures turn that model into
presentation material, and the Foundry project cross-checks the same mechanism
on a local EVM.

```text
searcher/  ->  data/  ->  analysis/plot.py  ->  figures/
    |
    +-> dashboard/index.html
    +-> contracts/ Foundry EVM validation
```

| Layer | Main files | Responsibility |
| ----- | ---------- | -------------- |
| Core AMM model | `searcher/src/amm.rs` | Implements Uniswap-V2-style constant-product swap math, fees, quotes, and reserve updates. |
| Sandwich logic | `searcher/src/strategy.rs` | Computes victim `minOut`, simulates front-run/victim/back-run order, detects reverts, and searches for the best attacker size. |
| CLI and experiments | `searcher/src/main.rs`, `searcher/src/experiments.rs`, `searcher/src/report.rs` | Exposes `simulate`, `trace`, `sweep`, `defense`, `route`, and `bundle`; writes CSV outputs for analysis. |
| Analysis outputs | `data/`, `analysis/plot.py`, `figures/` | Stores sweep results and renders the classroom PNG charts. |
| Interactive demo | `dashboard/index.html` | Static browser dashboard that recomputes the same CPMM model for live parameter changes. |
| EVM validation | `contracts/src/`, `contracts/test/`, `contracts/script/` | Minimal Solidity AMM/token contracts plus Foundry tests and scripts that reproduce the sandwich scenarios locally. |
| Teaching material | `docs/`, `README.md`, `Final_Lab_Sandwich_MEV_EN.ipynb` | Mechanism notes, defenses, walkthrough material, and notebook flow for the final lab. |
