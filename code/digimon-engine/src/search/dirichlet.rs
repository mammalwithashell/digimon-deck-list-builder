//! Seeded Dirichlet sampling for root exploration noise.
//!
//! `rand_distr` is not a workspace dependency, so the Gamma sampler is
//! implemented directly: Marsaglia & Tsang's squeeze method for
//! `alpha >= 1`, plus the `alpha < 1` boost trick
//! (`Gamma(a) = Gamma(a + 1) * U^(1/a)`). A Dirichlet(alpha) draw over `n`
//! components is `n` iid `Gamma(alpha, 1)` draws normalized to sum 1. All
//! randomness comes from the caller's RNG so the search stays deterministic
//! per seed.

use rand::Rng;

/// Standard normal via Box–Muller (avoids a `rand_distr` dependency).
fn sample_standard_normal<R: Rng + ?Sized>(rng: &mut R) -> f64 {
    loop {
        let u1: f64 = rng.gen();
        if u1 <= f64::MIN_POSITIVE {
            continue; // ln(0) guard
        }
        let u2: f64 = rng.gen();
        return (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    }
}

/// One `Gamma(alpha, 1)` draw (Marsaglia–Tsang). `alpha` must be positive.
fn sample_gamma<R: Rng + ?Sized>(rng: &mut R, alpha: f64) -> f64 {
    debug_assert!(alpha > 0.0, "gamma shape must be positive, got {alpha}");
    if alpha < 1.0 {
        // Boost: Gamma(a) = Gamma(a + 1) * U^(1/a).
        let u: f64 = rng.gen::<f64>().max(f64::MIN_POSITIVE);
        return sample_gamma(rng, alpha + 1.0) * u.powf(1.0 / alpha);
    }
    let d = alpha - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = sample_standard_normal(rng);
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u: f64 = rng.gen();
        // Cheap squeeze first, exact acceptance second.
        if u < 1.0 - 0.0331 * x.powi(4) {
            return d * v;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// One Dirichlet(alpha, ..., alpha) draw over `n` components.
///
/// Returns a non-negative vector summing to ~1.0. Individual components can
/// underflow to exactly 0.0 in `f32` for very small `alpha` (sparse draws) —
/// harmless as exploration noise since the mix keeps `(1-eps)` of the prior.
/// Degenerate cases: `n == 0` returns empty; a (theoretically impossible)
/// all-zero gamma draw falls back to uniform.
pub(crate) fn sample_dirichlet<R: Rng + ?Sized>(rng: &mut R, alpha: f64, n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }
    let draws: Vec<f64> = (0..n).map(|_| sample_gamma(rng, alpha)).collect();
    let sum: f64 = draws.iter().sum();
    if sum <= 0.0 {
        return vec![1.0 / n as f32; n];
    }
    draws.iter().map(|&g| (g / sum) as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn dirichlet_is_nonnegative_and_normalized() {
        let mut rng = StdRng::seed_from_u64(1234);
        for &alpha in &[0.03f64, 0.3, 1.0, 5.0] {
            for &n in &[1usize, 2, 7, 40] {
                let d = sample_dirichlet(&mut rng, alpha, n);
                assert_eq!(d.len(), n);
                // Tiny-alpha draws are sparse: components may underflow to
                // exactly 0.0 in f32, but never go negative or non-finite.
                assert!(
                    d.iter().all(|&x| x >= 0.0 && x.is_finite()),
                    "alpha={alpha} n={n}: {d:?}"
                );
                let sum: f32 = d.iter().sum();
                assert!(
                    (sum - 1.0).abs() < 1e-4,
                    "alpha={alpha} n={n}: sum {sum} not ~1"
                );
            }
        }
    }

    #[test]
    fn dirichlet_is_deterministic_per_seed() {
        let a = sample_dirichlet(&mut StdRng::seed_from_u64(99), 0.3, 8);
        let b = sample_dirichlet(&mut StdRng::seed_from_u64(99), 0.3, 8);
        assert_eq!(a, b);
        let c = sample_dirichlet(&mut StdRng::seed_from_u64(100), 0.3, 8);
        assert_ne!(a, c);
    }

    #[test]
    fn gamma_mean_is_roughly_alpha() {
        // Loose sanity: mean of Gamma(alpha, 1) is alpha.
        let mut rng = StdRng::seed_from_u64(7);
        for &alpha in &[0.5f64, 2.0] {
            let n = 4000;
            let mean: f64 = (0..n).map(|_| sample_gamma(&mut rng, alpha)).sum::<f64>() / n as f64;
            assert!(
                (mean - alpha).abs() < 0.15 * alpha.max(1.0),
                "alpha={alpha}: sample mean {mean}"
            );
        }
    }

    #[test]
    fn dirichlet_empty_is_empty() {
        let mut rng = StdRng::seed_from_u64(0);
        assert!(sample_dirichlet(&mut rng, 0.3, 0).is_empty());
    }
}
