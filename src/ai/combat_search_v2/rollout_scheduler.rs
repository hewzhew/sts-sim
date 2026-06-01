const TURN_BEAM_EXTENSION_BUDGET_DIVISOR: usize = 3;

pub(super) fn turn_beam_extension_budget(max_evaluations: usize, beam_width: usize) -> usize {
    if max_evaluations == 0 {
        return 0;
    }
    beam_width
        .max(1)
        .div_ceil(TURN_BEAM_EXTENSION_BUDGET_DIVISOR)
        .min(max_evaluations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_beam_extension_budget_is_beam_width_bounded() {
        assert_eq!(turn_beam_extension_budget(0, 3), 0);
        assert_eq!(turn_beam_extension_budget(1, 3), 1);
        assert_eq!(turn_beam_extension_budget(16, 3), 1);
        assert_eq!(turn_beam_extension_budget(384, 3), 1);
        assert_eq!(turn_beam_extension_budget(384, 8), 3);
    }
}
