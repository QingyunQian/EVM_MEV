//! CLI entry point for the sandwich MEV simulator.
//!
//! Three subcommands:
//!   * `simulate` - run a single sandwich scenario and print the outcome.
//!   * `trace`    - print the ordered pool-state trace for classroom demos.
//!   * `sweep`    - run the parametric sweeps that the report / plots consume.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod amm;
mod experiments;
mod report;
mod strategy;

use crate::amm::Pool;
use crate::experiments::{
    sweep_attacker_size, sweep_fee, sweep_pool_depth, sweep_slippage, sweep_victim_size,
};
use crate::strategy::{optimal_sandwich, simulate, VictimSwap};

#[derive(Debug, Clone, Copy)]
struct GasCost {
    gas_units: f64,
    base_fee_gwei: f64,
    priority_fee_gwei: f64,
    native_price_x: f64,
}

impl GasCost {
    fn cost_x(self) -> f64 {
        self.gas_units * (self.base_fee_gwei + self.priority_fee_gwei) * 1e-9 * self.native_price_x
    }
}

#[derive(Parser)]
#[command(
    name = "sandwich",
    about = "Educational sandwich MEV simulator for CPMMs"
)]
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
        #[arg(long)]
        attacker: Option<f64>,
        /// Total gas units consumed by the attack bundle. Use 0 to ignore gas.
        #[arg(long, default_value_t = 0.0)]
        gas_units: f64,
        /// Base fee in gwei for gas-cost accounting.
        #[arg(long, default_value_t = 0.0)]
        base_fee_gwei: f64,
        /// Priority fee in gwei for gas-cost accounting.
        #[arg(long, default_value_t = 0.0)]
        priority_fee_gwei: f64,
        /// Native gas token price denominated in token X.
        #[arg(long, default_value_t = 1.0)]
        native_price_x: f64,
    },
    /// Print the ordered pool-state trace for a sandwich.
    Trace {
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
        /// Attacker front-run size. Defaults to the optimizer's best value.
        #[arg(long)]
        attacker: Option<f64>,
        /// Total gas units consumed by the attack bundle. Use 0 to ignore gas.
        #[arg(long, default_value_t = 0.0)]
        gas_units: f64,
        /// Base fee in gwei for gas-cost accounting.
        #[arg(long, default_value_t = 0.0)]
        base_fee_gwei: f64,
        /// Priority fee in gwei for gas-cost accounting.
        #[arg(long, default_value_t = 0.0)]
        priority_fee_gwei: f64,
        /// Native gas token price denominated in token X.
        #[arg(long, default_value_t = 1.0)]
        native_price_x: f64,
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
        Cmd::Simulate {
            x,
            y,
            fee,
            victim,
            slippage,
            attacker,
            gas_units,
            base_fee_gwei,
            priority_fee_gwei,
            native_price_x,
        } => {
            let pool = Pool::new(x, y, fee);
            let v = VictimSwap {
                v: victim,
                slippage,
            };
            let gas = GasCost {
                gas_units,
                base_fee_gwei,
                priority_fee_gwei,
                native_price_x,
            };
            let o = match attacker {
                Some(a) => simulate(&pool, &v, a),
                None => {
                    let candidate = optimal_sandwich(&pool, &v);
                    if candidate.attacker_profit > gas.cost_x() {
                        candidate
                    } else {
                        simulate(&pool, &v, 0.0)
                    }
                }
            };
            let gas_cost_x = if o.attacker_in > 0.0 {
                gas.cost_x()
            } else {
                0.0
            };
            let net_profit_x = o.attacker_profit - gas_cost_x;
            println!("pool: x={x}  y={y}  fee={fee}");
            println!("victim: v={victim}  slippage={slippage}");
            if let Some(a) = attacker {
                println!("attacker: fixed front-run size={a}");
            } else {
                println!("attacker: optimized front-run size");
            }
            println!();
            println!("attacker_in         = {:>14.6}", o.attacker_in);
            println!("attacker_front_out  = {:>14.6}", o.attacker_front_out);
            println!("attacker_back_out   = {:>14.6}", o.attacker_back_out);
            println!("attacker_profit (X) = {:>14.6}", o.attacker_profit);
            println!("gas_cost (X)        = {:>14.6}", gas_cost_x);
            println!("net_profit (X)      = {:>14.6}", net_profit_x);
            println!(
                "attacker_roi        = {:>14.4}%",
                if o.attacker_in > 0.0 {
                    o.attacker_profit / o.attacker_in * 100.0
                } else {
                    0.0
                }
            );
            println!(
                "net_roi             = {:>14.4}%",
                if o.attacker_in > 0.0 {
                    net_profit_x / o.attacker_in * 100.0
                } else {
                    0.0
                }
            );
            if gas_units > 0.0 {
                println!(
                    "gas_model           = gas_units={}  base_fee_gwei={}  priority_fee_gwei={}  native_price_x={}",
                    gas_units, base_fee_gwei, priority_fee_gwei, native_price_x
                );
            }
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
        Cmd::Trace {
            x,
            y,
            fee,
            victim,
            slippage,
            attacker,
            gas_units,
            base_fee_gwei,
            priority_fee_gwei,
            native_price_x,
        } => {
            let pool = Pool::new(x, y, fee);
            let v = VictimSwap {
                v: victim,
                slippage,
            };
            let gas = GasCost {
                gas_units,
                base_fee_gwei,
                priority_fee_gwei,
                native_price_x,
            };
            let a = attacker.unwrap_or_else(|| {
                let candidate = optimal_sandwich(&pool, &v);
                if candidate.attacker_profit > gas.cost_x() {
                    candidate.attacker_in
                } else {
                    0.0
                }
            });
            print_trace(pool, v, a, gas);
        }
        Cmd::Sweep { out_dir } => {
            std::fs::create_dir_all(&out_dir)?;

            let pool = Pool::new(100_000.0, 100_000.0, 0.003);
            let reference_victim = VictimSwap {
                v: 1_000.0,
                slippage: 0.01,
            };

            let victim_rows = sweep_victim_size(pool, 0.01, 10.0, 20_000.0, 60);
            report::write_csv(&victim_rows, &out_dir.join("sweep_victim_size.csv"))?;

            let slip_rows = sweep_slippage(pool, 1_000.0, 0.0005, 0.05, 60);
            report::write_csv(&slip_rows, &out_dir.join("sweep_slippage.csv"))?;

            let depth_rows = sweep_pool_depth(0.003, 1_000.0, 0.01, 10_000.0, 10_000_000.0, 60);
            report::write_csv(&depth_rows, &out_dir.join("sweep_pool_depth.csv"))?;

            let fees = [0.0, 0.0005, 0.001, 0.003, 0.005, 0.01, 0.02, 0.03];
            let fee_rows = sweep_fee(100_000.0, 100_000.0, 1_000.0, 0.01, &fees);
            report::write_csv(&fee_rows, &out_dir.join("sweep_fee.csv"))?;

            let attacker_rows = sweep_attacker_size(pool, reference_victim, 120);
            report::write_csv(&attacker_rows, &out_dir.join("sweep_attacker_size.csv"))?;

            println!("wrote CSVs to {}", out_dir.display());
        }
    }
    Ok(())
}

fn print_trace(pool: Pool, victim: VictimSwap, attacker_in: f64, gas: GasCost) {
    let honest_out = pool.preview_x_for_y(victim.v);
    let min_out = honest_out * (1.0 - victim.slippage);
    let outcome = simulate(&pool, &victim, attacker_in);

    println!("pool: x={}  y={}  fee={}", pool.x, pool.y, pool.fee);
    println!(
        "victim: v={}  slippage={}  honest_out={:.6}  min_out={:.6}",
        victim.v, victim.slippage, honest_out, min_out
    );
    println!("attacker_in={:.6}", attacker_in);
    println!();
    println!("| step | actor | action | amount_in | amount_out | reserve_x | reserve_y | price_y_per_x | note |");
    println!("| ---- | ----- | ------ | --------- | ---------- | --------- | --------- | ------------- | ---- |");
    println!(
        "| 0 | - | initial pool | - | - | {:.6} | {:.6} | {:.6} | quote before attack |",
        pool.x,
        pool.y,
        pool.price()
    );

    let mut p = pool;
    let front_out = p.swap_x_for_y(attacker_in);
    println!("| 1 | attacker | front-run X->Y | {:.6} X | {:.6} Y | {:.6} | {:.6} | {:.6} | pushes price against victim |",
        attacker_in, front_out, p.x, p.y, p.price());

    let victim_preview = p.preview_x_for_y(victim.v);
    if victim_preview < min_out {
        println!("| 2 | victim | X->Y | {:.6} X | {:.6} Y | {:.6} | {:.6} | {:.6} | reverts: below min_out |",
            victim.v, victim_preview, p.x, p.y, p.price());
        let back_out = p.swap_y_for_x(front_out);
        println!("| 3 | attacker | unwind Y->X | {:.6} Y | {:.6} X | {:.6} | {:.6} | {:.6} | closes position after failed sandwich |",
            front_out, back_out, p.x, p.y, p.price());
    } else {
        let victim_out = p.swap_x_for_y(victim.v);
        println!("| 2 | victim | X->Y | {:.6} X | {:.6} Y | {:.6} | {:.6} | {:.6} | executes near slippage bound |",
            victim.v, victim_out, p.x, p.y, p.price());
        let back_out = p.swap_y_for_x(front_out);
        println!("| 3 | attacker | back-run Y->X | {:.6} Y | {:.6} X | {:.6} | {:.6} | {:.6} | realizes price-impact profit |",
            front_out, back_out, p.x, p.y, p.price());
    }

    println!();
    println!("attacker_profit (X) = {:.6}", outcome.attacker_profit);
    let gas_cost_x = if attacker_in > 0.0 { gas.cost_x() } else { 0.0 };
    println!("gas_cost (X)        = {:.6}", gas_cost_x);
    println!(
        "net_profit (X)      = {:.6}",
        outcome.attacker_profit - gas_cost_x
    );
    println!("victim_extra_loss   = {:.6}", outcome.victim_extra_loss);
    println!("reverted            = {}", outcome.reverted);
}
