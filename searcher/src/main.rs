//! CLI entry point for the sandwich MEV simulator.
//!
//! Two subcommands:
//!   * `simulate` – run a single sandwich scenario and print the outcome.
//!   * `sweep`    – run the parametric sweeps that the report / plots consume.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod amm;
mod experiments;
mod report;
mod strategy;

use crate::amm::Pool;
use crate::experiments::{sweep_fee, sweep_pool_depth, sweep_slippage, sweep_victim_size};
use crate::strategy::{optimal_sandwich, VictimSwap};

#[derive(Parser)]
#[command(name = "sandwich", about = "Educational sandwich MEV simulator for CPMMs")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a single scenario and print the result.
    Simulate {
        #[arg(long, default_value_t = 100_000.0)]
        x: f64,
        #[arg(long, default_value_t = 100_000.0)]
        y: f64,
        #[arg(long, default_value_t = 0.003)]
        fee: f64,
        #[arg(long, default_value_t = 1_000.0)]
        victim: f64,
        #[arg(long, default_value_t = 0.01)]
        slippage: f64,
    },
    /// Run all sweeps and write CSVs into `--out-dir` (default `../data`).
    Sweep {
        #[arg(long, default_value = "../data")]
        out_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Simulate { x, y, fee, victim, slippage } => {
            let pool = Pool::new(x, y, fee);
            let v = VictimSwap { v: victim, slippage };
            let o = optimal_sandwich(&pool, &v);
            println!("pool: x={x}  y={y}  fee={fee}");
            println!("victim: v={victim}  slippage={slippage}");
            println!();
            println!("attacker_in         = {:>14.6}", o.attacker_in);
            println!("attacker_front_out  = {:>14.6}", o.attacker_front_out);
            println!("attacker_back_out   = {:>14.6}", o.attacker_back_out);
            println!("attacker_profit (X) = {:>14.6}", o.attacker_profit);
            println!(
                "attacker_roi        = {:>14.4}%",
                if o.attacker_in > 0.0 { o.attacker_profit / o.attacker_in * 100.0 } else { 0.0 }
            );
            println!();
            println!("victim_honest_out   = {:>14.6}", o.victim_honest_out);
            println!("victim_actual_out   = {:>14.6}", o.victim_actual_out);
            println!("victim_extra_loss   = {:>14.6}", o.victim_extra_loss);
            println!("victim_min_out      = {:>14.6}", o.victim_min_out);
            println!("reverted            = {}", o.reverted);
            println!();
            println!("price_before        = {:>14.6}", o.price_before);
            println!("price_after_front   = {:>14.6}", o.price_after_front);
            println!("price_after_victim  = {:>14.6}", o.price_after_victim);
            println!("price_after_back    = {:>14.6}", o.price_after_back);
        }
        Cmd::Sweep { out_dir } => {
            std::fs::create_dir_all(&out_dir)?;

            let pool = Pool::new(100_000.0, 100_000.0, 0.003);

            let victim_rows = sweep_victim_size(pool, 0.01, 10.0, 20_000.0, 60);
            report::write_csv(&victim_rows, &out_dir.join("sweep_victim_size.csv"))?;

            let slip_rows = sweep_slippage(pool, 1_000.0, 0.0005, 0.05, 60);
            report::write_csv(&slip_rows, &out_dir.join("sweep_slippage.csv"))?;

            let depth_rows = sweep_pool_depth(0.003, 1_000.0, 0.01, 10_000.0, 10_000_000.0, 60);
            report::write_csv(&depth_rows, &out_dir.join("sweep_pool_depth.csv"))?;

            let fees = [0.0, 0.0005, 0.001, 0.003, 0.005, 0.01, 0.02, 0.03];
            let fee_rows = sweep_fee(100_000.0, 100_000.0, 1_000.0, 0.01, &fees);
            report::write_csv(&fee_rows, &out_dir.join("sweep_fee.csv"))?;

            println!("wrote CSVs to {}", out_dir.display());
        }
    }
    Ok(())
}
