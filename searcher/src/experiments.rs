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
