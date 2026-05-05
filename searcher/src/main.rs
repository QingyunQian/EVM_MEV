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
    defense_comparison, sweep_attacker_size, sweep_fee, sweep_gas_cost, sweep_pool_depth,
    sweep_slippage, sweep_victim_size, GasSweepConfig,
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
    /// Write the defense comparison CSV into `--out-dir`.
    Defense {
        #[arg(long, default_value = "../data")]
        out_dir: PathBuf,
    },
    /// Compare direct one-pool execution with a two-hop route.
    Route {
        #[arg(long, default_value_t = 100_000.0)]
        x_m_x: f64,
        #[arg(long, default_value_t = 100_000.0)]
        x_m_m: f64,
        #[arg(long, default_value_t = 100_000.0)]
        m_y_m: f64,
        #[arg(long, default_value_t = 100_000.0)]
        m_y_y: f64,
        #[arg(long, default_value_t = 0.003)]
        fee: f64,
        #[arg(long, default_value_t = 1_000.0)]
        victim: f64,
        #[arg(long, default_value_t = 0.01)]
        slippage: f64,
    },
    /// Compare transaction orderings around the reference victim swap.
    Bundle {
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
        #[arg(long, default_value_t = 2_000.0)]
        oversized_attacker: f64,
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

            let gas_rows = sweep_gas_cost(
                pool,
                reference_victim,
                GasSweepConfig {
                    gas_units: 500_000.0,
                    priority_fee_gwei: 2.0,
                    native_price_x: 1.0,
                    base_fee_min: 0.0,
                    base_fee_max: 30_000.0,
                    n: 80,
                },
            );
            report::write_csv(&gas_rows, &out_dir.join("sweep_gas.csv"))?;

            let defense_rows = defense_comparison();
            report::write_csv(&defense_rows, &out_dir.join("defense_comparison.csv"))?;

            println!("wrote CSVs to {}", out_dir.display());
        }
        Cmd::Defense { out_dir } => {
            std::fs::create_dir_all(&out_dir)?;
            let defense_rows = defense_comparison();
            report::write_csv(&defense_rows, &out_dir.join("defense_comparison.csv"))?;
            println!("wrote defense comparison to {}", out_dir.display());
        }
        Cmd::Route {
            x_m_x,
            x_m_m,
            m_y_m,
            m_y_y,
            fee,
            victim,
            slippage,
        } => {
            print_route_demo(x_m_x, x_m_m, m_y_m, m_y_y, fee, victim, slippage);
        }
        Cmd::Bundle {
            x,
            y,
            fee,
            victim,
            slippage,
            oversized_attacker,
        } => {
            print_bundle_demo(x, y, fee, victim, slippage, oversized_attacker);
        }
    }
    Ok(())
}

fn print_route_demo(
    x_m_x: f64,
    x_m_m: f64,
    m_y_m: f64,
    m_y_y: f64,
    fee: f64,
    victim_in: f64,
    slippage: f64,
) {
    let direct_pool = Pool::new(100_000.0, 100_000.0, fee);
    let direct_victim = VictimSwap {
        v: victim_in,
        slippage,
    };
    let direct = optimal_sandwich(&direct_pool, &direct_victim);

    let route = simulate_two_hop_sandwich(
        Pool::new(x_m_x, x_m_m, fee),
        Pool::new(m_y_m, m_y_y, fee),
        victim_in,
        slippage,
    );

    println!("route comparison: direct X->Y vs two-hop X->M->Y");
    println!("victim: v={victim_in}  slippage={slippage}  fee={fee}");
    println!();
    println!("| route | attacker_in | attacker_profit_X | victim_honest_out_Y | victim_actual_out_Y | victim_extra_loss_Y | reverted |");
    println!("| ----- | ----------- | ----------------- | ------------------- | ------------------- | ------------------- | -------- |");
    println!(
        "| direct X->Y | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} | {} |",
        direct.attacker_in,
        direct.attacker_profit,
        direct.victim_honest_out,
        direct.victim_actual_out,
        direct.victim_extra_loss,
        direct.reverted
    );
    println!(
        "| two-hop X->M->Y | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} | {} |",
        route.attacker_in,
        route.attacker_profit,
        route.victim_honest_out,
        route.victim_actual_out,
        route.victim_extra_loss,
        route.reverted
    );
    println!();
    println!("two-hop note: the attacker sandwiches the first hop X->M, while the victim's minOut is checked on final Y output.");
}

#[derive(Debug, Clone, Copy)]
struct RouteOutcome {
    attacker_in: f64,
    attacker_profit: f64,
    victim_honest_out: f64,
    victim_actual_out: f64,
    victim_extra_loss: f64,
    reverted: bool,
}

fn simulate_two_hop_sandwich(
    first_pool: Pool,
    second_pool: Pool,
    victim_in: f64,
    slippage: f64,
) -> RouteOutcome {
    let honest_mid = first_pool.preview_x_for_y(victim_in);
    let honest_out = second_pool.preview_x_for_y(honest_mid);
    let min_out = honest_out * (1.0 - slippage);
    let hi = first_pool.x * 0.05;

    let mut best = route_outcome(first_pool, second_pool, victim_in, min_out, honest_out, 0.0);
    for i in 1..=160 {
        let attacker_in = hi * i as f64 / 160.0;
        let o = route_outcome(
            first_pool,
            second_pool,
            victim_in,
            min_out,
            honest_out,
            attacker_in,
        );
        if !o.reverted && o.attacker_profit > best.attacker_profit {
            best = o;
        }
    }
    best
}

fn route_outcome(
    first_pool: Pool,
    second_pool: Pool,
    victim_in: f64,
    min_out: f64,
    honest_out: f64,
    attacker_in: f64,
) -> RouteOutcome {
    let mut first = first_pool;
    let mut second = second_pool;

    if attacker_in <= 0.0 {
        let mid = first.swap_x_for_y(victim_in);
        let out = second.swap_x_for_y(mid);
        return RouteOutcome {
            attacker_in: 0.0,
            attacker_profit: 0.0,
            victim_honest_out: honest_out,
            victim_actual_out: out,
            victim_extra_loss: 0.0,
            reverted: false,
        };
    }

    let attacker_mid = first.swap_x_for_y(attacker_in);
    let victim_mid_preview = first.preview_x_for_y(victim_in);
    let victim_out_preview = second.preview_x_for_y(victim_mid_preview);
    if victim_out_preview < min_out {
        let back_out = first.swap_y_for_x(attacker_mid);
        return RouteOutcome {
            attacker_in,
            attacker_profit: back_out - attacker_in,
            victim_honest_out: honest_out,
            victim_actual_out: victim_out_preview,
            victim_extra_loss: honest_out - victim_out_preview,
            reverted: true,
        };
    }

    let victim_mid = first.swap_x_for_y(victim_in);
    let victim_out = second.swap_x_for_y(victim_mid);
    let back_out = first.swap_y_for_x(attacker_mid);
    RouteOutcome {
        attacker_in,
        attacker_profit: back_out - attacker_in,
        victim_honest_out: honest_out,
        victim_actual_out: victim_out,
        victim_extra_loss: honest_out - victim_out,
        reverted: false,
    }
}

fn print_bundle_demo(
    x: f64,
    y: f64,
    fee: f64,
    victim_in: f64,
    slippage: f64,
    oversized_attacker: f64,
) {
    let pool = Pool::new(x, y, fee);
    let victim = VictimSwap {
        v: victim_in,
        slippage,
    };
    let honest = simulate(&pool, &victim, 0.0);
    let sandwich = optimal_sandwich(&pool, &victim);
    let oversized = simulate(&pool, &victim, oversized_attacker);

    println!("bundle/order comparison");
    println!("victim: v={victim_in}  slippage={slippage}  fee={fee}");
    println!();
    println!("| ordering | attacker_in | attacker_profit_X | victim_output_Y | victim_extra_loss_Y | outcome |");
    println!("| -------- | ----------- | ----------------- | --------------- | ------------------- | ------- |");
    println!(
        "| honest victim only | {:.6} | {:.6} | {:.6} | {:.6} | no attack |",
        0.0, 0.0, honest.victim_actual_out, 0.0
    );
    println!(
        "| attacker -> victim -> attacker | {:.6} | {:.6} | {:.6} | {:.6} | sandwich executes |",
        sandwich.attacker_in,
        sandwich.attacker_profit,
        sandwich.victim_actual_out,
        sandwich.victim_extra_loss
    );
    println!(
        "| oversized attacker -> victim -> unwind | {:.6} | {:.6} | {:.6} | {:.6} | victim reverts={} |",
        oversized.attacker_in,
        oversized.attacker_profit,
        oversized.victim_actual_out,
        oversized.victim_extra_loss,
        oversized.reverted
    );
    println!(
        "| victim -> attacker | {:.6} | {:.6} | {:.6} | {:.6} | no front-run opportunity |",
        0.0, 0.0, honest.victim_actual_out, 0.0
    );
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

#[cfg(test)]
mod cli_demo_tests {
    use super::*;

    #[test]
    fn two_hop_route_finds_executable_positive_sandwich() {
        let outcome = simulate_two_hop_sandwich(
            Pool::new(100_000.0, 100_000.0, 0.003),
            Pool::new(100_000.0, 100_000.0, 0.003),
            1_000.0,
            0.01,
        );

        assert!(!outcome.reverted);
        assert!(outcome.attacker_in > 0.0);
        assert!(outcome.attacker_profit > 0.0);
        assert!(outcome.victim_extra_loss > 0.0);
        assert!(outcome.victim_actual_out >= outcome.victim_honest_out * 0.99);
    }

    #[test]
    fn oversized_bundle_case_reverts_and_unwinds_at_loss() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let victim = VictimSwap {
            v: 1_000.0,
            slippage: 0.01,
        };
        let oversized = simulate(&pool, &victim, 2_000.0);

        assert!(oversized.reverted);
        assert!(oversized.attacker_profit < 0.0);
    }
}
