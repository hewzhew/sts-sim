use sts_simulator::ai::combat_search_v2::CombatLineLabReport;
use sts_simulator::eval::combat_case::CombatCase;

use super::search_types::SearchReview;

#[path = "boss_pressure_lens/collector.rs"]
mod collector;
#[path = "boss_pressure_lens/line_reviews.rs"]
mod line_reviews;
#[path = "boss_pressure_lens/types.rs"]
mod types;

pub(super) use types::BossPressureLensReport;

use collector::collector_pressure_lens;

pub(super) fn boss_pressure_lens(
    case: &CombatCase,
    ladder: &[SearchReview],
    line_lab: Option<&CombatLineLabReport>,
) -> Option<BossPressureLensReport> {
    collector_pressure_lens(case, ladder, line_lab)
}
