use serde::Serialize;

use super::super::focus::CombatReviewFocus;
use super::super::key_card_lifecycle::KeyCardLifecycleReport;
use super::super::search_types::SearchReview;

#[derive(Serialize)]
pub(crate) struct KeyCardCounterfactualProbe {
    pub(crate) schema: &'static str,
    pub(crate) contract: &'static str,
    pub(crate) skipped_reason: Option<&'static str>,
    pub(crate) variants: Vec<KeyCardCounterfactualVariant>,
}

#[derive(Serialize)]
pub(crate) struct KeyCardCounterfactualVariant {
    pub(crate) card: String,
    pub(crate) uuid: u32,
    pub(crate) reason: &'static str,
    pub(crate) placement: &'static str,
    pub(crate) skipped_reason: Option<&'static str>,
    pub(crate) search: Option<SearchReview>,
    pub(crate) focus: Option<CombatReviewFocus>,
    pub(crate) key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

#[derive(Clone, Copy)]
pub(crate) enum KeyCardCounterfactualPlacement {
    OpeningHand,
    DrawTop,
}

impl KeyCardCounterfactualPlacement {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OpeningHand => "opening_hand",
            Self::DrawTop => "draw_top",
        }
    }

    pub(super) fn search_label(self) -> &'static str {
        match self {
            Self::OpeningHand => "key_card_counterfactual_opening_hand",
            Self::DrawTop => "key_card_counterfactual_draw_top",
        }
    }
}
