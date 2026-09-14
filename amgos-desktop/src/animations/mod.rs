//! AMGOS Animation Engine — Cubic-Bezier Timing System
//!
//! Evaluates timing curves across all system surfaces, window transitions,
//! desktop switches, and UI layer animations.
//! Implements Newton-Raphson numerical inversion with bisection fallback.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    pub p1x: f64,
    pub p1y: f64,
    pub p2x: f64,
    pub p2y: f64,
}

impl CubicBezier {
    pub const fn new(p1x: f64, p1y: f64, p2x: f64, p2y: f64) -> Self {
        Self { p1x, p1y, p2x, p2y }
    }

    /// Solves for y given an animation progress x in [0.0, 1.0]
    pub fn solve(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }

        let t = self.find_t_for_x(x);
        self.eval_y(t)
    }

    #[inline]
    fn eval_x(&self, t: f64) -> f64 {
        // B(t) = 3*(1-t)^2 * t * p1x + 3*(1-t)*t^2 * p2x + t^3
        let one_minus_t = 1.0 - t;
        3.0 * one_minus_t * one_minus_t * t * self.p1x
            + 3.0 * one_minus_t * t * t * self.p2x
            + t * t * t
    }

    #[inline]
    fn eval_dx(&self, t: f64) -> f64 {
        // Derivative of eval_x with respect to t
        let one_minus_t = 1.0 - t;
        3.0 * one_minus_t * one_minus_t * self.p1x
            + 6.0 * one_minus_t * t * (self.p2x - self.p1x)
            + 3.0 * t * t * (1.0 - self.p2x)
    }

    #[inline]
    fn eval_y(&self, t: f64) -> f64 {
        let one_minus_t = 1.0 - t;
        3.0 * one_minus_t * one_minus_t * t * self.p1y
            + 3.0 * one_minus_t * t * t * self.p2y
            + t * t * t
    }

    fn find_t_for_x(&self, x: f64) -> f64 {
        // 1. Initial guess using Newton-Raphson
        let mut t = x;
        for _ in 0..8 {
            let current_x = self.eval_x(t) - x;
            if current_x.abs() < 1e-6 {
                return t;
            }
            let dx = self.eval_dx(t);
            if dx.abs() < 1e-6 {
                break;
            }
            t -= current_x / dx;
        }

        // 2. Fallback to binary bisection search
        let mut t0 = 0.0;
        let mut t1 = 1.0;
        let mut t = x;

        while t0 < t1 {
            let current_x = self.eval_x(t);
            if (current_x - x).abs() < 1e-6 {
                return t;
            }
            if x > current_x {
                t0 = t;
            } else {
                t1 = t;
            }
            t = (t1 + t0) * 0.5;
        }

        t
    }
}

/// Standardized AMGOS Timing Curves
pub mod curves {
    use super::CubicBezier;

    /// Standard AMGOS transition curve (natural, responsive)
    pub const STANDARD: CubicBezier = CubicBezier::new(0.2, 0.0, 0.0, 1.0);

    /// Decelerating curve for incoming windows, toolbars, and menus
    pub const DECELERATE: CubicBezier = CubicBezier::new(0.0, 0.0, 0.2, 1.0);

    /// Accelerating curve for dismissals, app close, and screen lock
    pub const ACCELERATE: CubicBezier = CubicBezier::new(0.4, 0.0, 1.0, 1.0);

    /// System Spring curve for dock bounce and gesture feedback
    pub const SYSTEM_SPRING: CubicBezier = CubicBezier::new(0.15, 1.0, 0.3, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_bezier_endpoints() {
        let curve = curves::STANDARD;
        assert_eq!(curve.solve(0.0), 0.0);
        assert_eq!(curve.solve(1.0), 1.0);
    }

    #[test]
    fn test_cubic_bezier_monotonicity() {
        let curve = curves::STANDARD;
        let mut prev = 0.0;
        for i in 1..=100 {
            let progress = i as f64 / 100.0;
            let val = curve.solve(progress);
            assert!(val >= prev, "Progress should be monotonic");
            prev = val;
        }
    }
}
