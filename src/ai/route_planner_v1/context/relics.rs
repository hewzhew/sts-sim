use crate::content::relics::RelicId;
use crate::state::RunState;

use super::super::types::RouteRelicSummaryV1;

pub(super) fn build_relic_summary(run_state: &RunState) -> RouteRelicSummaryV1 {
    let relics = run_state
        .relics
        .iter()
        .map(|relic| relic.id)
        .collect::<Vec<_>>();
    let has_juzu_bracelet = has_relic(&relics, RelicId::JuzuBracelet);
    let has_tiny_chest = has_relic(&relics, RelicId::TinyChest);
    let has_preserved_insect = has_relic(&relics, RelicId::PreservedInsect);
    let has_peace_pipe = has_relic(&relics, RelicId::PeacePipe);
    let has_shovel = has_relic(&relics, RelicId::Shovel);
    let has_girya = has_relic(&relics, RelicId::Girya);
    let has_smiling_mask = has_relic(&relics, RelicId::SmilingMask);
    let has_membership_card = has_relic(&relics, RelicId::MembershipCard);
    let has_courier = has_relic(&relics, RelicId::Courier);
    let has_cursed_key = has_relic(&relics, RelicId::CursedKey);
    let wing_boots_charges = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::WingBoots)
        .map(|relic| relic.counter.max(0) as u8)
        .unwrap_or(0);
    RouteRelicSummaryV1 {
        relic_count: run_state.relics.len(),
        relics,
        wing_boots_charges,
        has_juzu_bracelet,
        has_tiny_chest,
        has_preserved_insect,
        has_peace_pipe,
        has_shovel,
        has_girya,
        has_smiling_mask,
        has_membership_card,
        has_courier,
        has_cursed_key,
    }
}

fn has_relic(relics: &[RelicId], id: RelicId) -> bool {
    relics.iter().any(|&relic| relic == id)
}
