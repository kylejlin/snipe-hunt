//! Tunable Snipe Hunt evaluation.
//!
//! Features are always expressed from the player-to-move's perspective.  The
//! rules adapter owns feature extraction; keeping the weights here allows
//! native tournaments to tune them without coupling search to board storage.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SnipeFeatures {
    /// Friendly animals not captured, minus enemy animals not captured.
    pub material: i32,
    /// Friendly reserve count minus enemy reserve count.
    pub reserve: i32,
    /// Friendly legal full turns minus enemy legal full turns.
    pub mobility: i32,
    /// Forward progress of friendly animals minus enemy progress.
    pub progress: i32,
    /// Friendly retreat-capable animals minus enemy retreat-capable animals.
    pub retreaters: i32,
    /// Friendly two-element near-triplets minus enemy near-triplets.
    pub near_triplets: i32,
    /// Enemy pieces capturable in one turn minus friendly pieces capturable.
    pub capture_pressure: i32,
    /// Enemy snipe threats minus threats to the friendly snipe.
    pub snipe_pressure: i32,
    /// Friendly snipe escape squares minus enemy snipe escape squares.
    pub snipe_liberties: i32,
    /// Enemy pinned/singleton rows minus friendly pinned/singleton rows.
    pub row_freedom: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnipeWeights {
    pub material: i32,
    pub reserve: i32,
    pub mobility: i32,
    pub progress: i32,
    pub retreaters: i32,
    pub near_triplets: i32,
    pub capture_pressure: i32,
    pub snipe_pressure: i32,
    pub snipe_liberties: i32,
    pub row_freedom: i32,
}

impl Default for SnipeWeights {
    fn default() -> Self {
        Self {
            // A snipe capture is handled as mate, never as material.
            material: 120,
            reserve: 18,
            mobility: 3,
            progress: 8,
            retreaters: 10,
            near_triplets: 34,
            capture_pressure: 55,
            snipe_pressure: 310,
            snipe_liberties: 42,
            row_freedom: 24,
        }
    }
}

impl SnipeWeights {
    #[inline]
    pub fn evaluate(self, f: SnipeFeatures) -> i32 {
        let score = self.material * f.material
            + self.reserve * f.reserve
            + self.mobility * f.mobility
            + self.progress * f.progress
            + self.retreaters * f.retreaters
            + self.near_triplets * f.near_triplets
            + self.capture_pressure * f.capture_pressure
            + self.snipe_pressure * f.snipe_pressure
            + self.snipe_liberties * f.snipe_liberties
            + self.row_freedom * f.row_freedom;
        // Preserve ample headroom below mate values.
        score.clamp(-500_000, 500_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_snipe_attack_dominates_a_small_material_gain() {
        let weights = SnipeWeights::default();
        let attack = weights.evaluate(SnipeFeatures {
            snipe_pressure: 1,
            ..SnipeFeatures::default()
        });
        let two_animals = weights.evaluate(SnipeFeatures {
            material: 2,
            ..SnipeFeatures::default()
        });
        assert!(attack > two_animals);
    }

    #[test]
    fn evaluation_is_antisymmetric_when_features_are_negated() {
        let weights = SnipeWeights::default();
        let f = SnipeFeatures {
            material: 2,
            reserve: -1,
            mobility: 7,
            progress: 3,
            retreaters: -2,
            near_triplets: 1,
            capture_pressure: -1,
            snipe_pressure: 0,
            snipe_liberties: 2,
            row_freedom: -3,
        };
        let n = SnipeFeatures {
            material: -f.material,
            reserve: -f.reserve,
            mobility: -f.mobility,
            progress: -f.progress,
            retreaters: -f.retreaters,
            near_triplets: -f.near_triplets,
            capture_pressure: -f.capture_pressure,
            snipe_pressure: -f.snipe_pressure,
            snipe_liberties: -f.snipe_liberties,
            row_freedom: -f.row_freedom,
        };
        assert_eq!(weights.evaluate(f), -weights.evaluate(n));
    }
}
