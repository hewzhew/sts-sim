// No dependencies for golden_eye logic yet.

/// Golden Eye: Whenever you Scry, Scry 2 additional cards.
/// This mechanic is statically embedded into `engine::resolve_action` -> `Action::Scry`.
/// We check `has_relic(GoldenEye)` there and increase the amount by 2 before resolving `ScrySelect`.

pub fn is_golden_eye() -> bool {
    true
}
