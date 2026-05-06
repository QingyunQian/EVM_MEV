//! Parametric sweeps used to study how the sandwich payoff depends on
//! victim size, slippage tolerance, pool depth, and fee.

use serde::Serialize;

use crate::amm::Pool;
use crate::strategy::{optimal_sandwich, simulate, VictimSwap};

#[derive(Debug, Clone, Serialize)]
pub struct SweepRow {
    pub scenario: String,
    pub pool_x: f64,
    pub pool_y: f64,
    pub fee: f64,
    pub victim_v: f64,
    pub slippage: f64,
    pub attacker_in: f64,
    pub attacker_profit: f64,
    pub victim_honest_out: f64,
    pub victim_actual_out: f64,
    pub victim_extra_loss: f64,
    pub attacker_roi: f64,
    pub reverted: u8,
    pub price_before: f64,
    pub price_after_front: f64,
    pub price_after_victim: f64,
    pub price_after_back: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttackCurveRow {
    pub scenario: String,
    pub pool_x: f64,
    pub pool_y: f64,
    pub fee: f64,
    pub victim_v: f64,
    pub slippage: f64,
    pub attacker_in: f64,
    pub attacker_profit: f64,
    pub victim_actual_out: f64,
    pub victim_min_out: f64,
    pub victim_extra_loss: f64,
    pub reverted: u8,
    pub price_after_front: f64,
    pub is_optimal: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct GasModel {
    pub gas_units: f64,
    pub base_fee_gwei: f64,
    pub priority_fee_gwei: f64,
    pub native_price_x: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct GasSweepConfig {
    pub gas_units: f64,
    pub priority_fee_gwei: f64,
    pub native_price_x: f64,
    pub base_fee_min: f64,
    pub base_fee_max: f64,
    pub n: usize,
}

impl GasModel {
    pub fn cost_x(self) -> f64 {
        self.gas_units * (self.base_fee_gwei + self.priority_fee_gwei) * 1e-9 * self.native_price_x
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GasSweepRow {
    pub scenario: String,
    pub pool_x: f64,
    pub pool_y: f64,
    pub fee: f64,
    pub victim_v: f64,
    pub slippage: f64,
    pub gas_units: f64,
    pub base_fee_gwei: f64,
    pub priority_fee_gwei: f64,
    pub native_price_x: f64,
    pub attacker_in_gross_optimal: f64,
    pub gross_profit: f64,
    pub gas_cost: f64,
    pub net_profit_after_gas: f64,
    pub gas_aware_attacker_in: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefenseRow {
    pub defense: String,
    pub setting: String,
    pub pool_x: f64,
    pub pool_y: f64,
    pub fee: f64,
    pub victim_v: f64,
    pub slippage: f64,
    pub gas_units: f64,
    pub base_fee_gwei: f64,
    pub priority_fee_gwei: f64,
    pub native_price_x: f64,
    pub attacker_in: f64,
    pub gross_profit: f64,
    pub gas_cost: f64,
    pub net_profit_after_gas: f64,
    pub victim_extra_loss: f64,
}

fn row(scenario: &str, pool: &Pool, v: &VictimSwap) -> SweepRow {
    let o = optimal_sandwich(pool, v);
    let roi = if o.attacker_in > 0.0 {
        o.attacker_profit / o.attacker_in
    } else {
        0.0
    };
    SweepRow {
        scenario: scenario.to_string(),
        pool_x: pool.x,
        pool_y: pool.y,
        fee: pool.fee,
        victim_v: v.v,
        slippage: v.slippage,
        attacker_in: o.attacker_in,
        attacker_profit: o.attacker_profit,
        victim_honest_out: o.victim_honest_out,
        victim_actual_out: o.victim_actual_out,
        victim_extra_loss: o.victim_extra_loss,
        attacker_roi: roi,
        reverted: if o.reverted { 1 } else { 0 },
        price_before: o.price_before,
        price_after_front: o.price_after_front,
        price_after_victim: o.price_after_victim,
        price_after_back: o.price_after_back,
    }
}

fn gas_aware_row(scenario: &str, pool: Pool, victim: VictimSwap, gas: GasModel) -> GasSweepRow {
    let gross = optimal_sandwich(&pool, &victim);
    let gas_cost = if gross.attacker_in > 0.0 {
        gas.cost_x()
    } else {
        0.0
    };
    let net = gross.attacker_profit - gas_cost;
    GasSweepRow {
        scenario: scenario.to_string(),
        pool_x: pool.x,
        pool_y: pool.y,
        fee: pool.fee,
        victim_v: victim.v,
        slippage: victim.slippage,
        gas_units: gas.gas_units,
        base_fee_gwei: gas.base_fee_gwei,
        priority_fee_gwei: gas.priority_fee_gwei,
        native_price_x: gas.native_price_x,
        attacker_in_gross_optimal: gross.attacker_in,
        gross_profit: gross.attacker_profit,
        gas_cost,
        net_profit_after_gas: net.max(0.0),
        gas_aware_attacker_in: if net > 0.0 { gross.attacker_in } else { 0.0 },
    }
}

fn defense_row(
    defense: &str,
    setting: &str,
    pool: Pool,
    victim: VictimSwap,
    gas: GasModel,
    force_no_attack: bool,
) -> DefenseRow {
    let gross = if force_no_attack {
        simulate(&pool, &victim, 0.0)
    } else {
        optimal_sandwich(&pool, &victim)
    };
    let gas_cost = if gross.attacker_in > 0.0 {
        gas.cost_x()
    } else {
        0.0
    };
    let net = gross.attacker_profit - gas_cost;
    DefenseRow {
        defense: defense.to_string(),
        setting: setting.to_string(),
        pool_x: pool.x,
        pool_y: pool.y,
        fee: pool.fee,
        victim_v: victim.v,
        slippage: victim.slippage,
        gas_units: gas.gas_units,
        base_fee_gwei: gas.base_fee_gwei,
        priority_fee_gwei: gas.priority_fee_gwei,
        native_price_x: gas.native_price_x,
        attacker_in: if net > 0.0 { gross.attacker_in } else { 0.0 },
        gross_profit: gross.attacker_profit,
        gas_cost,
        net_profit_after_gas: net.max(0.0),
        victim_extra_loss: if net > 0.0 {
            gross.victim_extra_loss
        } else {
            0.0
        },
    }
}

/// Sweep victim trade size from `v_min` to `v_max` with `n` log-spaced points.
pub fn sweep_victim_size(
    pool: Pool,
    slippage: f64,
    v_min: f64,
    v_max: f64,
    n: usize,
) -> Vec<SweepRow> {
    logspace(v_min, v_max, n)
        .into_iter()
        .map(|v| row("victim_size", &pool, &VictimSwap { v, slippage }))
        .collect()
}

/// Sweep slippage tolerance from `s_min` to `s_max` linearly.
pub fn sweep_slippage(pool: Pool, v: f64, s_min: f64, s_max: f64, n: usize) -> Vec<SweepRow> {
    linspace(s_min, s_max, n)
        .into_iter()
        .map(|s| row("slippage", &pool, &VictimSwap { v, slippage: s }))
        .collect()
}

/// Sweep pool depth, keeping price at 1 X = 1 Y and all other params fixed.
pub fn sweep_pool_depth(
    fee: f64,
    victim_v: f64,
    slippage: f64,
    d_min: f64,
    d_max: f64,
    n: usize,
) -> Vec<SweepRow> {
    logspace(d_min, d_max, n)
        .into_iter()
        .map(|d| {
            let p = Pool::new(d, d, fee);
            row(
                "pool_depth",
                &p,
                &VictimSwap {
                    v: victim_v,
                    slippage,
                },
            )
        })
        .collect()
}

/// Sweep fee over a list of values.
pub fn sweep_fee(x: f64, y: f64, victim_v: f64, slippage: f64, fees: &[f64]) -> Vec<SweepRow> {
    fees.iter()
        .map(|&f| {
            let p = Pool::new(x, y, f);
            row(
                "fee",
                &p,
                &VictimSwap {
                    v: victim_v,
                    slippage,
                },
            )
        })
        .collect()
}

/// Sweep fixed attacker sizes around the reference scenario. This is meant
/// for classroom visualization: it shows where profit peaks and where the
/// victim starts reverting.
pub fn sweep_attacker_size(pool: Pool, victim: VictimSwap, n: usize) -> Vec<AttackCurveRow> {
    let optimal = optimal_sandwich(&pool, &victim);
    let upper = if optimal.attacker_in > 0.0 {
        optimal.attacker_in * 2.4
    } else {
        victim.v.max(pool.x * 0.02)
    };
    linspace(0.0, upper, n)
        .into_iter()
        .map(|a| {
            let o = simulate(&pool, &victim, a);
            AttackCurveRow {
                scenario: "attacker_size".to_string(),
                pool_x: pool.x,
                pool_y: pool.y,
                fee: pool.fee,
                victim_v: victim.v,
                slippage: victim.slippage,
                attacker_in: o.attacker_in,
                attacker_profit: o.attacker_profit,
                victim_actual_out: o.victim_actual_out,
                victim_min_out: o.victim_min_out,
                victim_extra_loss: o.victim_extra_loss,
                reverted: if o.reverted { 1 } else { 0 },
                price_after_front: o.price_after_front,
                is_optimal: if (a - optimal.attacker_in).abs() <= upper / (n.max(1) as f64) {
                    1
                } else {
                    0
                },
            }
        })
        .collect()
}

/// Sweep base fee to show where gross MEV remains positive but rational net
/// profit disappears after fixed bundle gas cost.
pub fn sweep_gas_cost(pool: Pool, victim: VictimSwap, config: GasSweepConfig) -> Vec<GasSweepRow> {
    linspace(config.base_fee_min, config.base_fee_max, config.n)
        .into_iter()
        .map(|base_fee_gwei| {
            gas_aware_row(
                "gas_cost",
                pool,
                victim,
                GasModel {
                    gas_units: config.gas_units,
                    base_fee_gwei,
                    priority_fee_gwei: config.priority_fee_gwei,
                    native_price_x: config.native_price_x,
                },
            )
        })
        .collect()
}

/// Small defense comparison table used for live presentation. The private
/// route row is modeled as no public pre-trade visibility, so no attack is
/// attempted.
pub fn defense_comparison() -> Vec<DefenseRow> {
    let base_pool = Pool::new(100_000.0, 100_000.0, 0.003);
    let base_victim = VictimSwap {
        v: 1_000.0,
        slippage: 0.01,
    };
    let base_gas = GasModel {
        gas_units: 500_000.0,
        base_fee_gwei: 25.0,
        priority_fee_gwei: 2.0,
        native_price_x: 1.0,
    };

    vec![
        defense_row(
            "reference",
            "1% slippage, 30 bps fee, 100k pool",
            base_pool,
            base_victim,
            base_gas,
            false,
        ),
        defense_row(
            "lower_slippage",
            "0.2% slippage",
            base_pool,
            VictimSwap {
                v: 1_000.0,
                slippage: 0.002,
            },
            base_gas,
            false,
        ),
        defense_row(
            "deeper_pool",
            "500k / 500k pool",
            Pool::new(500_000.0, 500_000.0, 0.003),
            base_victim,
            base_gas,
            false,
        ),
        defense_row(
            "higher_fee",
            "1% pool fee",
            Pool::new(100_000.0, 100_000.0, 0.01),
            base_victim,
            base_gas,
            false,
        ),
        defense_row(
            "high_gas",
            "500k gas, 20000 gwei base fee",
            base_pool,
            base_victim,
            GasModel {
                gas_units: 500_000.0,
                base_fee_gwei: 20_000.0,
                priority_fee_gwei: 0.0,
                native_price_x: 1.0,
            },
            false,
        ),
        defense_row(
            "private_route",
            "victim order not visible before execution",
            base_pool,
            base_victim,
            base_gas,
            true,
        ),
    ]
}

fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n <= 1 {
        return vec![a];
    }
    let step = (b - a) / (n as f64 - 1.0);
    (0..n).map(|i| a + step * i as f64).collect()
}

fn logspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n <= 1 {
        return vec![a];
    }
    let la = a.ln();
    let lb = b.ln();
    let step = (lb - la) / (n as f64 - 1.0);
    (0..n).map(|i| (la + step * i as f64).exp()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn gas_model_cost_uses_total_fee_and_native_price() {
        let gas = GasModel {
            gas_units: 500_000.0,
            base_fee_gwei: 25.0,
            priority_fee_gwei: 2.0,
            native_price_x: 1.0,
        };

        assert_relative_eq!(gas.cost_x(), 0.0135, epsilon = 1e-12);
    }

    #[test]
    fn gas_sweep_clamps_net_profit_after_gas() {
        let rows = sweep_gas_cost(
            Pool::new(100_000.0, 100_000.0, 0.003),
            VictimSwap {
                v: 1_000.0,
                slippage: 0.01,
            },
            GasSweepConfig {
                gas_units: 500_000.0,
                priority_fee_gwei: 0.0,
                native_price_x: 1.0,
                base_fee_min: 0.0,
                base_fee_max: 20_000.0,
                n: 2,
            },
        );

        assert_eq!(rows.len(), 2);
        assert!(rows[0].net_profit_after_gas > 0.0);
        assert_eq!(rows[1].net_profit_after_gas, 0.0);
        assert_eq!(rows[1].gas_aware_attacker_in, 0.0);
        assert!(rows[1].gross_profit > 0.0);
        assert!(rows[1].gas_cost > rows[1].gross_profit);
    }

    #[test]
    fn attacker_size_sweep_contains_executable_optimum_and_revert_region() {
        let rows = sweep_attacker_size(
            Pool::new(100_000.0, 100_000.0, 0.003),
            VictimSwap {
                v: 1_000.0,
                slippage: 0.01,
            },
            120,
        );

        assert_eq!(rows.len(), 120);
        assert!(rows.iter().any(|r| r.is_optimal == 1 && r.reverted == 0));
        assert!(rows.iter().any(|r| r.reverted == 1));
        assert!(rows
            .iter()
            .filter(|r| r.reverted == 0)
            .all(|r| r.victim_actual_out >= r.victim_min_out));
        assert!(rows
            .iter()
            .filter(|r| r.reverted == 1)
            .all(|r| r.victim_actual_out < r.victim_min_out));
    }

    #[test]
    fn defense_comparison_contains_reference_and_private_route() {
        let rows = defense_comparison();
        let reference = rows
            .iter()
            .find(|r| r.defense == "reference")
            .expect("reference defense row");
        let private_route = rows
            .iter()
            .find(|r| r.defense == "private_route")
            .expect("private route defense row");

        assert!(reference.net_profit_after_gas > 0.0);
        assert!(reference.victim_extra_loss > 0.0);
        assert_eq!(private_route.attacker_in, 0.0);
        assert_eq!(private_route.net_profit_after_gas, 0.0);
        assert_eq!(private_route.victim_extra_loss, 0.0);
    }

    #[test]
    fn defense_high_gas_keeps_gross_mev_but_skips_execution() {
        let rows = defense_comparison();
        let high_gas = rows
            .iter()
            .find(|r| r.defense == "high_gas")
            .expect("high gas defense row");

        assert!(high_gas.gross_profit > 0.0);
        assert!(high_gas.gas_cost > high_gas.gross_profit);
        assert_eq!(high_gas.attacker_in, 0.0);
        assert_eq!(high_gas.net_profit_after_gas, 0.0);
        assert_eq!(high_gas.victim_extra_loss, 0.0);
    }

    #[test]
    fn one_point_sweeps_return_requested_boundary_value() {
        let slip = sweep_slippage(
            Pool::new(100_000.0, 100_000.0, 0.003),
            1_000.0,
            0.005,
            0.05,
            1,
        );
        let depth = sweep_pool_depth(0.003, 1_000.0, 0.01, 10_000.0, 1_000_000.0, 1);

        assert_eq!(slip.len(), 1);
        assert_relative_eq!(slip[0].slippage, 0.005, epsilon = 1e-12);
        assert_eq!(depth.len(), 1);
        assert_relative_eq!(depth[0].pool_x, 10_000.0, epsilon = 1e-12);
        assert_relative_eq!(depth[0].pool_y, 10_000.0, epsilon = 1e-12);
    }
}
