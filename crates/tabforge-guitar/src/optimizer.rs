use crate::error::{GuitarError, Result};
use crate::fretboard::{FretPosition, Fretboard};
use tabforge_core::Pitch;

/// Cost weights configuration for fingering optimization
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostWeights {
    pub fret_distance_weight: f32,
    pub string_change_weight: f32,
    pub hand_position_shift_weight: f32,
    pub open_string_bonus: f32,
}

impl Default for CostWeights {
    fn default() -> Self {
        Self {
            fret_distance_weight: 1.0,
            string_change_weight: 0.5,
            hand_position_shift_weight: 1.5,
            open_string_bonus: -0.2,
        }
    }
}

/// Dynamic Programming / Viterbi-based fingering optimizer
pub struct FingeringOptimizer {
    pub fretboard: Fretboard,
    pub weights: CostWeights,
}

impl FingeringOptimizer {
    pub fn new(fretboard: Fretboard) -> Self {
        Self {
            fretboard,
            weights: CostWeights::default(),
        }
    }

    pub fn with_weights(mut self, weights: CostWeights) -> Self {
        self.weights = weights;
        self
    }

    /// Calculate transition cost between two successive fret positions
    pub fn transition_cost(&self, prev: &FretPosition, curr: &FretPosition) -> f32 {
        let fret_dist = (prev.fret as i32 - curr.fret as i32).abs() as f32;
        let string_dist = (prev.string as i32 - curr.string as i32).abs() as f32;

        let mut cost = fret_dist * self.weights.fret_distance_weight
            + string_dist * self.weights.string_change_weight;

        // Open string ergonomic bonus
        if curr.fret == 0 {
            cost += self.weights.open_string_bonus;
        }

        cost
    }

    /// Optimize fingering positions for a sequential melody of pitches using Dynamic Programming
    pub fn optimize_melody(&self, pitches: &[Pitch]) -> Result<Vec<FretPosition>> {
        if pitches.is_empty() {
            return Ok(Vec::new());
        }

        // Generate candidate positions for each step
        let mut candidates_per_step: Vec<Vec<FretPosition>> = Vec::with_capacity(pitches.len());
        for pitch in pitches {
            let positions = self.fretboard.find_positions(pitch);
            if positions.is_empty() {
                return Err(GuitarError::OutOfRange(pitch.to_string()));
            }
            candidates_per_step.push(positions);
        }

        // DP table: (accumulated_cost, previous_candidate_index)
        let num_steps = candidates_per_step.len();
        let mut dp: Vec<Vec<(f32, usize)>> = Vec::with_capacity(num_steps);

        // Initialize step 0
        let step0: Vec<(f32, usize)> = candidates_per_step[0]
            .iter()
            .map(|pos| {
                // Bias slightly towards lower positions near nut (fret 0..5)
                let initial_cost = if pos.fret == 0 { 0.0 } else { (pos.fret as f32) * 0.1 };
                (initial_cost, 0)
            })
            .collect();
        dp.push(step0);

        // Forward Viterbi pass
        for t in 1..num_steps {
            let prev_candidates = &candidates_per_step[t - 1];
            let curr_candidates = &candidates_per_step[t];
            let prev_dp = &dp[t - 1];

            let mut curr_dp = Vec::with_capacity(curr_candidates.len());
            for curr_pos in curr_candidates {
                let mut best_cost = f32::MAX;
                let mut best_prev_idx = 0;

                for (p_idx, prev_pos) in prev_candidates.iter().enumerate() {
                    let cost = prev_dp[p_idx].0 + self.transition_cost(prev_pos, curr_pos);
                    if cost < best_cost {
                        best_cost = cost;
                        best_prev_idx = p_idx;
                    }
                }
                curr_dp.push((best_cost, best_prev_idx));
            }
            dp.push(curr_dp);
        }

        // Backward trace to reconstruct path
        let mut path = Vec::with_capacity(num_steps);
        let mut best_last_idx = 0;
        let mut min_final_cost = f32::MAX;
        for (idx, &(cost, _)) in dp[num_steps - 1].iter().enumerate() {
            if cost < min_final_cost {
                min_final_cost = cost;
                best_last_idx = idx;
            }
        }

        let mut curr_idx = best_last_idx;
        for t in (0..num_steps).rev() {
            path.push(candidates_per_step[t][curr_idx]);
            curr_idx = dp[t][curr_idx].1;
        }

        path.reverse();
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_viterbi_fingering_optimization() {
        let fretboard = Fretboard::default();
        let optimizer = FingeringOptimizer::new(fretboard);

        // Sequence of notes in C Major scale
        let notes = vec![
            Pitch::from_str("C4").unwrap(),
            Pitch::from_str("D4").unwrap(),
            Pitch::from_str("E4").unwrap(),
            Pitch::from_str("F4").unwrap(),
            Pitch::from_str("G4").unwrap(),
        ];

        let result = optimizer.optimize_melody(&notes);
        assert!(result.is_ok());
        let fingering = result.unwrap();
        assert_eq!(fingering.len(), 5);
    }
}
