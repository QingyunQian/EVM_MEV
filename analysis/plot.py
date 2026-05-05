"""Render the sandwich-MEV experiment figures from the CSV sweeps.

Usage:
    python analysis/plot.py            # reads ./data, writes ./figures
    python analysis/plot.py --data DIR --figures DIR
"""

import argparse
import csv
from pathlib import Path

import matplotlib.pyplot as plt


Rows = list[dict[str, str]]


def read_rows(path: Path) -> Rows:
    with path.open(newline="") as f:
        return list(csv.DictReader(f))


def values(rows: Rows, key: str) -> list[float]:
    return [float(r[key]) for r in rows]


def labels(rows: Rows, key: str) -> list[str]:
    return [r[key] for r in rows]


def only(rows: Rows, key: str, want: str) -> Rows:
    return [r for r in rows if r[key] == want]


def plot_victim_size(df: Rows, out: Path) -> None:
    fig, ax1 = plt.subplots(figsize=(8, 5))
    ax1.plot(values(df, "victim_v"), values(df, "attacker_profit"), label="attacker profit (X)",
             color="tab:red")
    ax1.plot(values(df, "victim_v"), values(df, "victim_extra_loss"), label="victim extra loss (Y)",
             color="tab:blue", linestyle="--")
    ax1.set_xscale("log")
    ax1.set_xlabel("victim trade size v (X)")
    ax1.set_ylabel("tokens")
    ax1.grid(True, which="both", alpha=0.3)

    ax2 = ax1.twinx()
    ax2.plot(values(df, "victim_v"), [x * 100.0 for x in values(df, "attacker_roi")],
             label="attacker ROI (%)", color="tab:green", linestyle=":")
    ax2.set_ylabel("attacker ROI (%)")

    lines1, labels1 = ax1.get_legend_handles_labels()
    lines2, labels2 = ax2.get_legend_handles_labels()
    ax1.legend(lines1 + lines2, labels1 + labels2, loc="upper left")
    ax1.set_title("Sandwich profit vs victim trade size\n"
                  "(pool 100k/100k, fee 0.30%, slippage 1%)")
    fig.tight_layout()
    fig.savefig(out / "fig_victim_size.png", dpi=160)
    plt.close(fig)


def plot_slippage(df: Rows, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot([x * 100.0 for x in values(df, "slippage")], values(df, "attacker_profit"),
            label="attacker profit (X)", color="tab:red")
    ax.plot([x * 100.0 for x in values(df, "slippage")], values(df, "victim_extra_loss"),
            label="victim extra loss (Y)", color="tab:blue", linestyle="--")
    ax.set_xlabel("victim slippage tolerance (%)")
    ax.set_ylabel("tokens")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title("Effect of slippage tolerance on sandwich payoff\n"
                 "(pool 100k/100k, fee 0.30%, victim v=1000)")
    fig.tight_layout()
    fig.savefig(out / "fig_slippage.png", dpi=160)
    plt.close(fig)


def plot_pool_depth(df: Rows, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(values(df, "pool_x"), values(df, "attacker_profit"),
            label="attacker profit (X)", color="tab:red")
    ax.plot(values(df, "pool_x"), values(df, "victim_extra_loss"),
            label="victim extra loss (Y)", color="tab:blue", linestyle="--")
    ax.set_xscale("log")
    ax.set_yscale("symlog", linthresh=1e-3)
    ax.set_xlabel("pool depth (each side, log scale)")
    ax.set_ylabel("tokens (symlog)")
    ax.grid(True, which="both", alpha=0.3)
    ax.legend()
    ax.set_title("Deeper pools dilute sandwich profit\n"
                 "(fee 0.30%, victim v=1000, slippage 1%)")
    fig.tight_layout()
    fig.savefig(out / "fig_pool_depth.png", dpi=160)
    plt.close(fig)


def plot_fee(df: Rows, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    fee_pct = [x * 100.0 for x in values(df, "fee")]
    ax.plot(fee_pct, values(df, "attacker_profit"),
            label="attacker profit (X)", color="tab:red",
            marker="o", linewidth=2.2)
    ax.plot(fee_pct, values(df, "victim_extra_loss"),
            label="victim extra loss (Y)", color="tab:blue",
            marker="s", linestyle="--", linewidth=2.0)
    ax.axhline(0, color="black", linewidth=0.8, alpha=0.5)
    ax.set_xlabel("pool fee (%)")
    ax.set_ylabel("tokens")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title("Fee level kills sandwich profitability\n"
                 "(pool 100k/100k, victim v=1000, slippage 1%)")
    fig.tight_layout()
    fig.savefig(out / "fig_fee.png", dpi=160)
    plt.close(fig)


def plot_attacker_size(df: Rows, out: Path) -> None:
    fig, ax1 = plt.subplots(figsize=(8, 5))
    ok = only(df, "reverted", "0")
    reverted = only(df, "reverted", "1")

    ax1.plot(values(ok, "attacker_in"), values(ok, "attacker_profit"),
             label="attacker profit if victim executes", color="tab:red")
    if reverted:
        ax1.plot(values(reverted, "attacker_in"), values(reverted, "attacker_profit"),
                 label="attacker unwind PnL after victim reverts",
                 color="tab:gray", linestyle=":")
    best = max(ok, key=lambda r: float(r["attacker_profit"]))
    ax1.scatter([float(best["attacker_in"])], [float(best["attacker_profit"])],
                color="black", zorder=4, label="optimizer choice")
    ax1.axvline(float(best["attacker_in"]), color="black", alpha=0.25)
    ax1.axhline(0, color="black", linewidth=0.8, alpha=0.5)
    ax1.set_xlabel("attacker front-run size a (X)")
    ax1.set_ylabel("attacker PnL (X)")
    ax1.grid(True, alpha=0.3)

    ax2 = ax1.twinx()
    ax2.plot(values(df, "attacker_in"), values(df, "victim_actual_out"),
             label="victim output before slippage check (Y)", color="tab:blue",
             linestyle="--")
    ax2.axhline(float(df[0]["victim_min_out"]), color="tab:blue",
                alpha=0.35, linestyle="-.", label="victim min_out")
    ax2.set_ylabel("victim output (Y)")

    lines1, labels1 = ax1.get_legend_handles_labels()
    lines2, labels2 = ax2.get_legend_handles_labels()
    ax1.legend(lines1 + lines2, labels1 + labels2, loc="best")
    ax1.set_title("Attacker size frontier: profit peaks at the slippage edge\n"
                  "(pool 100k/100k, fee 0.30%, victim v=1000, slippage 1%)")
    fig.tight_layout()
    fig.savefig(out / "fig_attacker_size.png", dpi=160)
    plt.close(fig)


def plot_gas(df: Rows, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    x = values(df, "base_fee_gwei")
    ax.plot(x, values(df, "gross_profit"), label="gross attacker profit (X)",
            color="tab:red")
    ax.plot(x, values(df, "gas_cost"), label="gas cost (X)",
            color="tab:orange", linestyle="--")
    ax.plot(x, values(df, "net_profit_after_gas"), label="rational net profit after gas (X)",
            color="tab:green")
    ax.axhline(0, color="black", linewidth=0.8, alpha=0.5)
    ax.set_xlabel("base fee (gwei)")
    ax.set_ylabel("tokens X")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title("Gas cost turns gross MEV into non-executable MEV\n"
                 "(500k gas, 2 gwei priority fee, native token priced as 1 X)")
    fig.tight_layout()
    fig.savefig(out / "fig_gas.png", dpi=160)
    plt.close(fig)


def plot_defense(df: Rows, out: Path) -> None:
    xlabels = [s.replace("_", "\n") for s in labels(df, "defense")]
    x = list(range(len(df)))
    net_profit = values(df, "net_profit_after_gas")
    victim_loss = values(df, "victim_extra_loss")

    fig, ax = plt.subplots(figsize=(9, 5))
    width = 0.36
    net_bars = ax.bar([i - width / 2 for i in x], net_profit,
                      width=width, label="attacker net profit (X)",
                      color="tab:red", alpha=0.85)
    loss_bars = ax.bar([i + width / 2 for i in x], victim_loss,
                       width=width, label="victim extra loss (Y)",
                       color="tab:blue", alpha=0.75)
    ax.set_xticks(list(x))
    ax.set_xticklabels(xlabels)
    ax.set_ylabel("tokens")
    ax.grid(True, axis="y", alpha=0.3)
    ax.legend()
    ax.set_title("Defense comparison: each mitigation narrows or removes the sandwich window")

    top = max(net_profit + victim_loss + [1.0])
    ax.set_ylim(0, top * 1.18)
    for bars, vals in ((net_bars, net_profit), (loss_bars, victim_loss)):
        for bar, val in zip(bars, vals):
            x_pos = bar.get_x() + bar.get_width() / 2
            y_pos = val + top * 0.025 if val > 0 else top * 0.015
            label = f"{val:.1f}" if val >= 0.05 else "0"
            ax.text(x_pos, y_pos, label, ha="center", va="bottom", fontsize=8)

    fig.tight_layout()
    fig.savefig(out / "fig_defense.png", dpi=160)
    plt.close(fig)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", default="data", type=Path)
    parser.add_argument("--figures", default="figures", type=Path)
    args = parser.parse_args()

    args.figures.mkdir(parents=True, exist_ok=True)

    plot_victim_size(read_rows(args.data / "sweep_victim_size.csv"), args.figures)
    plot_slippage(read_rows(args.data / "sweep_slippage.csv"), args.figures)
    plot_pool_depth(read_rows(args.data / "sweep_pool_depth.csv"), args.figures)
    plot_fee(read_rows(args.data / "sweep_fee.csv"), args.figures)
    plot_attacker_size(read_rows(args.data / "sweep_attacker_size.csv"), args.figures)
    plot_gas(read_rows(args.data / "sweep_gas.csv"), args.figures)
    plot_defense(read_rows(args.data / "defense_comparison.csv"), args.figures)

    print(f"figures written to {args.figures}")


if __name__ == "__main__":
    main()
