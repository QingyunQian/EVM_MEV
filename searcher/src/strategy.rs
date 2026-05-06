//! Sandwich strategy: given a pool state and a known victim swap, find the
//! attacker's optimal front-run size.
//!
//! Setup:
//!  - Victim swaps `v` units of X for Y, with slippage tolerance `s`.
//!    Victim's `min_out` is computed from the honest expected output at the
//!    current pool state, i.e. what the victim's frontend would quote.
//!  - Attacker front-runs by swapping `a` units of X for Y.
//!  - Victim's transaction then executes against the disturbed pool; if the Y
//!    output falls below `min_out`, the victim tx reverts and the attacker
//!    also loses the opportunity.
//!  - Attacker back-runs by swapping the Y they just received back to X.
//!  - Profit (in X) is the X they recovered minus the X they put in.
//!
//! We search `a` on `[eps, a_max]` using a golden-section search, subject to
//! the victim-not-reverted constraint. This is not guaranteed to be globally
//! optimal under arbitrary fee schedules, but for a single CPMM pool the
//! profit function is unimodal on the feasible interval, so golden-section is
//! both simple and effective.

use crate::amm::Pool;

#[derive(Debug, Clone, Copy)]
pub struct VictimSwap {
    /// Amount of X the victim wants to swap in.
    pub v: f64,
    /// Slippage tolerance, e.g. 0.005 for 0.5%.
    pub slippage: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct SandwichOutcome {
    pub attacker_in: f64,
    pub attacker_front_out: f64,
    pub attacker_back_out: f64,
    pub attacker_profit: f64,
    pub victim_min_out: f64,
    pub victim_actual_out: f64,
    pub victim_honest_out: f64,
    pub victim_extra_loss: f64,
    pub reverted: bool,
    pub price_before: f64,
    pub price_after_front: f64,
    pub price_after_victim: f64,
    pub price_after_back: f64,
}

#[allow(dead_code)]
pub fn victim_min_out(pool: &Pool, victim: &VictimSwap) -> f64 {
    let honest = pool.preview_x_for_y(victim.v);
    honest * (1.0 - victim.slippage)
}

/// Simulate a sandwich with a specified attacker size `a`.
/// Returns the full outcome, including a `reverted` flag when the victim's
/// slippage check would fail.
pub fn simulate(pool: &Pool, victim: &VictimSwap, a: f64) -> SandwichOutcome {
    let honest_out = pool.preview_x_for_y(victim.v);
    let min_out = honest_out * (1.0 - victim.slippage);
    let price_before = pool.price();

    if a <= 0.0 {
        let mut p = *pool;
        let v_out = p.swap_x_for_y(victim.v);
        return SandwichOutcome {
            attacker_in: 0.0,
            attacker_front_out: 0.0,
            attacker_back_out: 0.0,
            attacker_profit: 0.0,
            victim_min_out: min_out,
            victim_actual_out: v_out,
            victim_honest_out: honest_out,
            victim_extra_loss: 0.0,
            reverted: false,
            price_before,
            price_after_front: price_before,
            price_after_victim: p.price(),
            price_after_back: p.price(),
        };
    }

    let mut p = *pool;
    let front_out = p.swap_x_for_y(a);
    let price_after_front = p.price();

    let victim_out_preview = p.preview_x_for_y(victim.v);
    if victim_out_preview < min_out {
        let back_out = p.swap_y_for_x(front_out);
        let price_after_back = p.price();
        return SandwichOutcome {
            attacker_in: a,
            attacker_front_out: front_out,
            attacker_back_out: back_out,
            attacker_profit: back_out - a,
            victim_min_out: min_out,
            victim_actual_out: victim_out_preview,
            victim_honest_out: honest_out,
            victim_extra_loss: honest_out - victim_out_preview,
            reverted: true,
            price_before,
            price_after_front,
            price_after_victim: price_after_front,
            price_after_back,
        };
    }

    let victim_out = p.swap_x_for_y(victim.v);
    let price_after_victim = p.price();

    let back_out = p.swap_y_for_x(front_out);
    let price_after_back = p.price();

    SandwichOutcome {
        attacker_in: a,
        attacker_front_out: front_out,
        attacker_back_out: back_out,
        attacker_profit: back_out - a,
        victim_min_out: min_out,
        victim_actual_out: victim_out,
        victim_honest_out: honest_out,
        victim_extra_loss: honest_out - victim_out,
        reverted: false,
        price_before,
        price_after_front,
        price_after_victim,
        price_after_back,
    }
}

/// Find the largest attacker size `a` such that the victim's slippage check
/// still passes. This is a monotonic bisection: larger `a` always hurts the
/// victim more, so feasibility is a half-line.
fn max_feasible_a(pool: &Pool, victim: &VictimSwap, hi: f64) -> f64 {
    if !simulate(pool, victim, hi).reverted {
        return hi;
    }
    let mut lo = 0.0_f64;
    let mut hi = hi;
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if simulate(pool, victim, mid).reverted {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    lo
}

/// Maximize attacker profit over `a` via golden-section search on the feasible
/// interval. Returns the best outcome seen.
pub fn optimal_sandwich(pool: &Pool, victim: &VictimSwap) -> SandwichOutcome {
    let a_hi_cap = pool.x * 10.0;
    let a_max = max_feasible_a(pool, victim, a_hi_cap);
    if a_max <= 0.0 {
        return simulate(pool, victim, 0.0);
    }

    let phi = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut lo = 0.0_f64;
    let mut hi = a_max;
    let mut c = hi - phi * (hi - lo);
    let mut d = lo + phi * (hi - lo);

    let profit = |a: f64| -> f64 {
        let o = simulate(pool, victim, a);
        if o.reverted {
            f64::NEG_INFINITY
        } else {
            o.attacker_profit
        }
    };

    for _ in 0..100 {
        if (hi - lo).abs() < 1e-10 {
            break;
        }
        if profit(c) > profit(d) {
            hi = d;
        } else {
            lo = c;
        }
        c = hi - phi * (hi - lo);
        d = lo + phi * (hi - lo);
    }

    let a_star = 0.5 * (lo + hi);
    let best = simulate(pool, victim, a_star);
    let null = simulate(pool, victim, 0.0);
    if best.attacker_profit > null.attacker_profit && !best.reverted {
        best
    } else {
        null
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn no_frontrun_is_zero_profit() {
        let pool = Pool::new(10_000.0, 10_000.0, 0.003);
        let v = VictimSwap {
            v: 100.0,
            slippage: 0.01,
        };
        let o = simulate(&pool, &v, 0.0);
        assert_relative_eq!(o.attacker_profit, 0.0);
        assert!(!o.reverted);
    }

    #[test]
    fn extreme_frontrun_reverts_victim() {
        let pool = Pool::new(10_000.0, 10_000.0, 0.003);
        let v = VictimSwap {
            v: 100.0,
            slippage: 0.005,
        };
        let o = simulate(&pool, &v, 5_000.0);
        assert!(o.reverted);
    }

    #[test]
    fn optimal_is_nonnegative_for_typical_case() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let v = VictimSwap {
            v: 1_000.0,
            slippage: 0.01,
        };
        let o = optimal_sandwich(&pool, &v);
        assert!(o.attacker_profit >= 0.0);
        assert!(!o.reverted);
    }

    #[test]
    fn reference_scenario_matches_known_values() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let v = VictimSwap {
            v: 1_000.0,
            slippage: 0.01,
        };
        let o = optimal_sandwich(&pool, &v);

        assert_relative_eq!(o.attacker_in, 507.044775, epsilon = 1e-5);
        assert_relative_eq!(o.victim_honest_out, 987.158034, epsilon = 1e-6);
        assert_relative_eq!(o.victim_actual_out, 977.286454, epsilon = 1e-6);
        assert_relative_eq!(o.victim_extra_loss, 9.871580, epsilon = 1e-6);
        assert_relative_eq!(o.attacker_profit, 7.016249, epsilon = 1e-6);
        assert_relative_eq!(o.victim_actual_out, o.victim_min_out, epsilon = 1e-6);
        assert!(!o.reverted);
    }

    #[test]
    fn price_path_uses_x_per_y_and_matches_sandwich_intuition() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let v = VictimSwap {
            v: 1_000.0,
            slippage: 0.01,
        };
        let o = optimal_sandwich(&pool, &v);

        assert_relative_eq!(o.price_before, 1.0, epsilon = 1e-12);
        assert!(o.price_after_front > o.price_before);
        assert!(o.price_after_victim > o.price_after_front);
        assert!(o.price_after_back < o.price_after_victim);
        assert_relative_eq!(o.price_after_front, 1.010151, epsilon = 1e-6);
        assert_relative_eq!(o.price_after_victim, 1.030322, epsilon = 1e-6);
        assert_relative_eq!(o.price_after_back, 1.019897, epsilon = 1e-6);
    }

    #[test]
    fn reverted_attack_records_preview_loss_and_unwind_loss() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let v = VictimSwap {
            v: 1_000.0,
            slippage: 0.01,
        };
        let o = simulate(&pool, &v, 2_000.0);

        assert!(o.reverted);
        assert!(o.victim_actual_out < o.victim_min_out);
        assert_relative_eq!(
            o.victim_extra_loss,
            o.victim_honest_out - o.victim_actual_out,
            epsilon = 1e-9
        );
        assert!(o.attacker_profit < 0.0);
    }

    #[test]
    fn tighter_slippage_shrinks_profit() {
        let pool = Pool::new(100_000.0, 100_000.0, 0.003);
        let loose = optimal_sandwich(
            &pool,
            &VictimSwap {
                v: 1_000.0,
                slippage: 0.02,
            },
        );
        let tight = optimal_sandwich(
            &pool,
            &VictimSwap {
                v: 1_000.0,
                slippage: 0.002,
            },
        );
        assert!(loose.attacker_profit >= tight.attacker_profit);
    }
}
