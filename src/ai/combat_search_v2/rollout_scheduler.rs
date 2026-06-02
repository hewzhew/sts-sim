const TURN_BEAM_EVALUATIONS_PER_EXTENSION: usize = 32;
const TURN_BEAM_MAX_EXTENSION_MULTIPLIER: usize = 4;

pub(super) fn turn_beam_extension_budget(max_evaluations: usize, beam_width: usize) -> usize {
    if max_evaluations == 0 {
        return 0;
    }
    let evaluation_budget = max_evaluations.div_ceil(TURN_BEAM_EVALUATIONS_PER_EXTENSION);
    let beam_budget = beam_width
        .max(1)
        .saturating_mul(TURN_BEAM_MAX_EXTENSION_MULTIPLIER);
    evaluation_budget.min(beam_budget).min(max_evaluations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_beam_extension_budget_scales_with_rollout_budget_and_beam_width() {
        assert_eq!(turn_beam_extension_budget(0, 3), 0);
        assert_eq!(turn_beam_extension_budget(1, 3), 1);
        assert_eq!(turn_beam_extension_budget(16, 3), 1);
        assert_eq!(turn_beam_extension_budget(384, 3), 12);
        assert_eq!(turn_beam_extension_budget(384, 8), 12);
        assert_eq!(turn_beam_extension_budget(2048, 3), 12);
        assert_eq!(turn_beam_extension_budget(2048, 8), 32);
    }
}
