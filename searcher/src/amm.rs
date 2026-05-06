//! Constant-product AMM math (Uniswap V2-style), in `f64`.
//!
//! The invariant is `x * y = k` with a linear fee applied to the *input* side,
//! identical to Uniswap V2: the fraction `1 - fee` of the input amount is
//! actually swapped against the reserves.

#[derive(Debug, Clone, Copy)]
pub struct Pool {
    pub x: f64,
    pub y: f64,
    pub fee: f64,
}

impl Pool {
    pub fn new(x: f64, y: f64, fee: f64) -> Self {
        assert!(x > 0.0 && y > 0.0, "reserves must be positive");
        assert!((0.0..1.0).contains(&fee), "fee must be in [0, 1)");
        Self { x, y, fee }
    }

    #[cfg(test)]
    pub fn k(&self) -> f64 {
        self.x * self.y
    }

    /// Mid-price of Y in units of X: `x / y`.
    pub fn price(&self) -> f64 {
        self.x / self.y
    }

    /// Swap `dx` units of X into the pool, receiving Y out.
    /// Returns the Y amount out.
    pub fn swap_x_for_y(&mut self, dx: f64) -> f64 {
        let dx_eff = dx * (1.0 - self.fee);
        let dy = self.y * dx_eff / (self.x + dx_eff);
        self.x += dx;
        self.y -= dy;
        dy
    }

    /// Swap `dy` units of Y into the pool, receiving X out.
    pub fn swap_y_for_x(&mut self, dy: f64) -> f64 {
        let dy_eff = dy * (1.0 - self.fee);
        let dx = self.x * dy_eff / (self.y + dy_eff);
        self.y += dy;
        self.x -= dx;
        dx
    }

    /// Pure preview of `swap_x_for_y` without mutating the pool.
    pub fn preview_x_for_y(&self, dx: f64) -> f64 {
        let dx_eff = dx * (1.0 - self.fee);
        self.y * dx_eff / (self.x + dx_eff)
    }

    /// Pure preview of `swap_y_for_x` without mutating the pool.
    #[allow(dead_code)]
    pub fn preview_y_for_x(&self, dy: f64) -> f64 {
        let dy_eff = dy * (1.0 - self.fee);
        self.x * dy_eff / (self.y + dy_eff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn zero_fee_preserves_k() {
        let mut p = Pool::new(1_000.0, 1_000.0, 0.0);
        let k0 = p.k();
        let _ = p.swap_x_for_y(10.0);
        assert_relative_eq!(p.k(), k0, epsilon = 1e-9);
    }

    #[test]
    fn fee_grows_k() {
        let mut p = Pool::new(1_000.0, 1_000.0, 0.003);
        let k0 = p.k();
        let _ = p.swap_x_for_y(10.0);
        assert!(p.k() > k0);
    }

    #[test]
    fn price_is_x_per_y() {
        let p = Pool::new(120.0, 80.0, 0.003);

        assert_relative_eq!(p.price(), 1.5, epsilon = 1e-12);
    }

    #[test]
    fn preview_matches_swap() {
        let p = Pool::new(5_000.0, 5_000.0, 0.003);
        let dy_preview = p.preview_x_for_y(100.0);
        let mut p2 = p;
        let dy = p2.swap_x_for_y(100.0);
        assert_relative_eq!(dy_preview, dy, epsilon = 1e-12);
    }

    #[test]
    fn preview_y_for_x_matches_swap() {
        let p = Pool::new(5_000.0, 7_000.0, 0.003);
        let dx_preview = p.preview_y_for_x(120.0);
        let mut p2 = p;
        let dx = p2.swap_y_for_x(120.0);

        assert_relative_eq!(dx_preview, dx, epsilon = 1e-12);
    }

    #[test]
    fn swaps_move_x_per_y_price_in_expected_direction() {
        let mut p = Pool::new(10_000.0, 10_000.0, 0.003);
        let initial_price = p.price();

        let y_out = p.swap_x_for_y(100.0);
        let after_buy_y = p.price();
        assert!(after_buy_y > initial_price);

        let _ = p.swap_y_for_x(y_out);
        let after_sell_y = p.price();
        assert!(after_sell_y < after_buy_y);
    }
}
