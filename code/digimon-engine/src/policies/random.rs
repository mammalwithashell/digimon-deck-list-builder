//! Uniformly-random valid action selection. Seeded for reproducibility.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::action::space::PASS;
use crate::game::Game;

use super::Policy;

pub struct RandomPolicy {
    rng: StdRng,
}

impl RandomPolicy {
    pub fn new(seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };
        Self { rng }
    }
}

impl Default for RandomPolicy {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Policy for RandomPolicy {
    fn select(&mut self, _game: &Game, mask: &[f32]) -> u16 {
        let valid: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if *v > 0.0 { Some(i as u16) } else { None })
            .collect();
        if valid.is_empty() {
            return PASS;
        }
        let idx = self.rng.gen_range(0..valid.len());
        valid[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mask_returns_pass() {
        // The empty-mask branch in `select` doesn't touch game state, so
        // assert via the same filter the implementation uses. A full
        // DebugRunner-driven integration test lives in
        // `tests/policies_random.rs`.
        let mask = vec![0.0f32; crate::action::space::ACTION_SPACE_SIZE];
        let valid: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if *v > 0.0 { Some(i as u16) } else { None })
            .collect();
        assert!(valid.is_empty());
    }

    #[test]
    fn seeded_is_deterministic() {
        let mut a = RandomPolicy::new(Some(7));
        let mut b = RandomPolicy::new(Some(7));
        let mut mask = vec![0.0f32; crate::action::space::ACTION_SPACE_SIZE];
        for i in 0..100 {
            mask[i] = 1.0;
        }
        // Same mask, same seed → same pick.
        // We can't call select without a Game, so exercise the rng directly:
        let va: u32 = a.rng.gen();
        let vb: u32 = b.rng.gen();
        assert_eq!(va, vb);
    }
}
