"""Render the sandwich-MEV experiment figures from the CSV sweeps.

Usage:
    python analysis/plot.py            # reads ./data, writes ./figures
    python analysis/plot.py --data DIR --figures DIR
"""

import argparse
from pathlib import Path

import matplotlib.pyplot as plt
import pandas as pd


def plot_victim_size(df: pd.DataFrame, out: Path) -> None:
    fig, ax1 = plt.subplots(figsize=(8, 5))
    ax1.plot(df["victim_v"], df["attacker_profit"], label="attacker profit (X)",
             color="tab:red")
    ax1.plot(df["victim_v"], df["victim_extra_loss"], label="victim extra loss (Y)",
             color="tab:blue", linestyle="--")
    ax1.set_xscale("log")
    ax1.set_xlabel("victim trade size v (X)")
    ax1.set_ylabel("tokens")
    ax1.grid(True, which="both", alpha=0.3)

    ax2 = ax1.twinx()
    ax2.plot(df["victim_v"], df["attacker_roi"] * 100.0,
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


def plot_slippage(df: pd.DataFrame, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(df["slippage"] * 100.0, df["attacker_profit"],
            label="attacker profit (X)", color="tab:red")
    ax.plot(df["slippage"] * 100.0, df["victim_extra_loss"],
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


def plot_pool_depth(df: pd.DataFrame, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(df["pool_x"], df["attacker_profit"],
            label="attacker profit (X)", color="tab:red")
    ax.plot(df["pool_x"], df["victim_extra_loss"],
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


def plot_fee(df: pd.DataFrame, out: Path) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.bar(df["fee"] * 100.0, df["attacker_profit"], width=0.12,
           label="attacker profit (X)", color="tab:red", alpha=0.8)
    ax.plot(df["fee"] * 100.0, df["victim_extra_loss"],
            label="victim extra loss (Y)", color="tab:blue",
            marker="o", linestyle="--")
    ax.set_xlabel("pool fee (%)")
    ax.set_ylabel("tokens")
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title("Fee level kills sandwich profitability\n"
                 "(pool 100k/100k, victim v=1000, slippage 1%)")
    fig.tight_layout()
    fig.savefig(out / "fig_fee.png", dpi=160)
    plt.close(fig)


def plot_attacker_size(df: pd.DataFrame, out: Path) -> None:
    fig, ax1 = plt.subplots(figsize=(8, 5))
    ok = df[df["reverted"] == 0]
    reverted = df[df["reverted"] == 1]

    ax1.plot(ok["attacker_in"], ok["attacker_profit"],
             label="attacker profit if victim executes", color="tab:red")
    if not reverted.empty:
        ax1.plot(reverted["attacker_in"], reverted["attacker_profit"],
                 label="attacker unwind PnL after victim reverts",
                 color="tab:gray", linestyle=":")
    best = ok.loc[ok["attacker_profit"].idxmax()]
    ax1.scatter([best["attacker_in"]], [best["attacker_profit"]],
                color="black", zorder=4, label="optimizer choice")
    ax1.axvline(best["attacker_in"], color="black", alpha=0.25)
    ax1.axhline(0, color="black", linewidth=0.8, alpha=0.5)
    ax1.set_xlabel("attacker front-run size a (X)")
    ax1.set_ylabel("attacker PnL (X)")
    ax1.grid(True, alpha=0.3)

    ax2 = ax1.twinx()
    ax2.plot(df["attacker_in"], df["victim_actual_out"],
             label="victim output before slippage check (Y)", color="tab:blue",
             linestyle="--")
    ax2.axhline(df["victim_min_out"].iloc[0], color="tab:blue",
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", default="data", type=Path)
    parser.add_argument("--figures", default="figures", type=Path)
    args = parser.parse_args()

    args.figures.mkdir(parents=True, exist_ok=True)

    plot_victim_size(pd.read_csv(args.data / "sweep_victim_size.csv"), args.figures)
    plot_slippage(pd.read_csv(args.data / "sweep_slippage.csv"), args.figures)
    plot_pool_depth(pd.read_csv(args.data / "sweep_pool_depth.csv"), args.figures)
    plot_fee(pd.read_csv(args.data / "sweep_fee.csv"), args.figures)
    plot_attacker_size(pd.read_csv(args.data / "sweep_attacker_size.csv"), args.figures)

    print(f"figures written to {args.figures}")


if __name__ == "__main__":
    main()
