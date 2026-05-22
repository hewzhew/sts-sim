use serde::{Deserialize, Serialize};

use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::events::{
    EventCardKind, EventEffect, EventOption, EventOptionTransition, EventRelicKind,
    EventSelectionKind,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CandidateResolution {
    pub status: ResolutionStatus,
    pub known_effects: Vec<KnownEffect>,
    pub unresolved_effects: Vec<UnresolvedEffect>,
    pub followup: Option<FollowupBoundary>,
    pub evidence: ResolutionEvidence,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ResolutionStatus {
    Known,
    Partial,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ResolutionEvidence {
    PublicScreenSemantics,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum KnownEffect {
    GainGold(i32),
    LoseGold(i32),
    LoseHp(i32),
    LoseMaxHp(i32),
    Heal(i32),
    GainMaxHp(i32),
    ObtainSpecificRelic { count: usize, relic: RelicId },
    ObtainSpecificCard { count: usize, card: CardId },
    ObtainSpecificColorlessCard { count: usize, card: CardId },
    ObtainSpecificCurse { count: usize, card: CardId },
    RemoveCard { count: usize },
    UpgradeCard { count: usize },
    TransformCard { count: usize },
    DuplicateCard { count: usize },
    LoseSpecificRelic { relic: RelicId, starter_only: bool },
    LoseRelic { starter_only: bool },
    StartCombat,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum UnresolvedEffect {
    RandomRelic {
        count: usize,
        pool: RelicPoolBoundary,
        visibility: HiddenInfoVisibility,
    },
    RandomPotion {
        count: usize,
        visibility: HiddenInfoVisibility,
    },
    RandomCard {
        count: usize,
        pool: CardPoolBoundary,
        visibility: HiddenInfoVisibility,
    },
    RandomCurse {
        count: usize,
        pool: CardPoolBoundary,
        visibility: HiddenInfoVisibility,
    },
    PlayerSelection {
        kind: SelectionBoundary,
    },
    TransformOutput {
        count: usize,
        visibility: HiddenInfoVisibility,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum HiddenInfoVisibility {
    DistributionKnownResultHiddenUntilResolved,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum RelicPoolBoundary {
    AnyRelic,
    Book,
    Face,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum CardPoolBoundary {
    ClassCard,
    Colorless,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum FollowupBoundary {
    EventScreenAdvance,
    EventComplete,
    RewardScreen,
    Selection(SelectionBoundary),
    CombatStart,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum SelectionBoundary {
    RemoveCard,
    UpgradeCard,
    TransformCard,
    DuplicateCard,
    OfferCard,
    Unknown,
}

impl CandidateResolution {
    pub fn from_event_option(option: &EventOption) -> Option<Self> {
        if option.ui.disabled {
            return None;
        }

        let mut known_effects = Vec::new();
        let mut unresolved_effects = Vec::new();
        for effect in &option.semantics.effects {
            push_effect_resolution(effect, &mut known_effects, &mut unresolved_effects);
        }

        let followup = followup_boundary(option.semantics.transition);
        if known_effects.is_empty() && unresolved_effects.is_empty() && followup.is_none() {
            return None;
        }

        let status = if !unresolved_effects.is_empty()
            || matches!(
                followup,
                Some(FollowupBoundary::RewardScreen | FollowupBoundary::Selection(_))
            ) {
            ResolutionStatus::Partial
        } else {
            ResolutionStatus::Known
        };

        Some(Self {
            status,
            known_effects,
            unresolved_effects,
            followup,
            evidence: ResolutionEvidence::PublicScreenSemantics,
        })
    }

    pub fn main_note(&self) -> Option<String> {
        let mut parts = self
            .known_effects
            .iter()
            .map(KnownEffect::brief)
            .chain(self.unresolved_effects.iter().map(UnresolvedEffect::brief))
            .collect::<Vec<_>>();
        if let Some(followup) = self.followup.and_then(FollowupBoundary::main_note) {
            if !parts.iter().any(|part| part == &followup) {
                parts.push(followup);
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }

    pub fn detail_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("resolution: {:?}", self.status)];
        if !self.known_effects.is_empty() {
            lines.push("known_effects:".to_string());
            lines.extend(
                self.known_effects
                    .iter()
                    .map(|effect| format!("  - {}", effect.brief())),
            );
        }
        if !self.unresolved_effects.is_empty() {
            lines.push("unresolved_effects:".to_string());
            lines.extend(
                self.unresolved_effects
                    .iter()
                    .map(|effect| format!("  - {}", effect.detail())),
            );
        }
        if let Some(followup) = self.followup {
            lines.push(format!("followup: {}", followup.detail()));
        }
        lines.push(
            "evidence: public screen semantics; hidden random results are not sampled".to_string(),
        );
        lines
    }
}

impl KnownEffect {
    fn brief(&self) -> String {
        match self {
            KnownEffect::GainGold(amount) => format!("gain {amount} gold"),
            KnownEffect::LoseGold(amount) => format!("lose {amount} gold"),
            KnownEffect::LoseHp(amount) => format!("lose {amount} hp"),
            KnownEffect::LoseMaxHp(amount) => format!("lose {amount} max hp"),
            KnownEffect::Heal(amount) => format!("heal {amount} hp"),
            KnownEffect::GainMaxHp(amount) => format!("gain {amount} max hp"),
            KnownEffect::ObtainSpecificRelic { count, relic } => {
                format!("obtain {count} specific relic {relic:?}")
            }
            KnownEffect::ObtainSpecificCard { count, card } => {
                format!("obtain {count} specific card {card:?}")
            }
            KnownEffect::ObtainSpecificColorlessCard { count, card } => {
                format!("obtain {count} specific colorless card {card:?}")
            }
            KnownEffect::ObtainSpecificCurse { count, card } => {
                format!("obtain {count} specific curse {card:?}")
            }
            KnownEffect::RemoveCard { count } => format!("remove {count} card"),
            KnownEffect::UpgradeCard { count } => format!("upgrade {count} card"),
            KnownEffect::TransformCard { count } => format!("transform {count} card"),
            KnownEffect::DuplicateCard { count } => format!("duplicate {count} card"),
            KnownEffect::LoseSpecificRelic {
                relic,
                starter_only,
            } => {
                if *starter_only {
                    format!("lose starter relic {relic:?}")
                } else {
                    format!("lose relic {relic:?}")
                }
            }
            KnownEffect::LoseRelic { starter_only } => {
                if *starter_only {
                    "lose starter relic".to_string()
                } else {
                    "lose relic".to_string()
                }
            }
            KnownEffect::StartCombat => "starts combat".to_string(),
        }
    }
}

impl UnresolvedEffect {
    fn brief(&self) -> String {
        match self {
            UnresolvedEffect::RandomRelic { pool, .. } => {
                format!("{} outcome", pool.brief())
            }
            UnresolvedEffect::RandomPotion { .. } => "random potion outcome".to_string(),
            UnresolvedEffect::RandomCard { pool, .. } => {
                format!("{} outcome", pool.brief())
            }
            UnresolvedEffect::RandomCurse { pool, .. } => {
                format!("{} outcome", pool.brief())
            }
            UnresolvedEffect::PlayerSelection { kind } => {
                format!("{} target selected later", kind.brief())
            }
            UnresolvedEffect::TransformOutput { .. } => {
                "transform result hidden until resolved".to_string()
            }
        }
    }

    fn detail(&self) -> String {
        match self {
            UnresolvedEffect::RandomRelic { count, pool, .. } => {
                format!(
                    "{count} {}; distribution known, result hidden",
                    pool.brief()
                )
            }
            UnresolvedEffect::RandomPotion { count, .. } => {
                format!("{count} random potion; distribution known, result hidden")
            }
            UnresolvedEffect::RandomCard { count, pool, .. } => {
                format!(
                    "{count} {}; distribution known, result hidden",
                    pool.brief()
                )
            }
            UnresolvedEffect::RandomCurse { count, pool, .. } => {
                format!(
                    "{count} {}; distribution known, result hidden",
                    pool.brief()
                )
            }
            UnresolvedEffect::PlayerSelection { kind } => {
                format!("player selection required: {}", kind.brief())
            }
            UnresolvedEffect::TransformOutput { count, .. } => {
                format!("{count} transform result; distribution known, result hidden")
            }
        }
    }
}

impl RelicPoolBoundary {
    fn brief(self) -> &'static str {
        match self {
            RelicPoolBoundary::AnyRelic => "random relic",
            RelicPoolBoundary::Book => "random book relic",
            RelicPoolBoundary::Face => "random face relic",
            RelicPoolBoundary::Unknown => "unknown relic",
        }
    }
}

impl CardPoolBoundary {
    fn brief(self) -> &'static str {
        match self {
            CardPoolBoundary::ClassCard => "random class card",
            CardPoolBoundary::Colorless => "random colorless card",
            CardPoolBoundary::Unknown => "unknown card",
        }
    }
}

impl FollowupBoundary {
    fn main_note(self) -> Option<String> {
        match self {
            FollowupBoundary::RewardScreen => Some("opens follow-up reward".to_string()),
            FollowupBoundary::Selection(kind) => Some(format!("opens {} selection", kind.brief())),
            FollowupBoundary::CombatStart => Some("starts combat".to_string()),
            FollowupBoundary::EventScreenAdvance | FollowupBoundary::EventComplete => None,
        }
    }

    fn detail(self) -> String {
        match self {
            FollowupBoundary::EventScreenAdvance => "event screen advances".to_string(),
            FollowupBoundary::EventComplete => "event completes".to_string(),
            FollowupBoundary::RewardScreen => {
                "reward screen; contents resolved by engine".to_string()
            }
            FollowupBoundary::Selection(kind) => format!("selection: {}", kind.brief()),
            FollowupBoundary::CombatStart => "combat start boundary".to_string(),
        }
    }
}

impl SelectionBoundary {
    fn brief(self) -> &'static str {
        match self {
            SelectionBoundary::RemoveCard => "remove card",
            SelectionBoundary::UpgradeCard => "upgrade card",
            SelectionBoundary::TransformCard => "transform card",
            SelectionBoundary::DuplicateCard => "duplicate card",
            SelectionBoundary::OfferCard => "offer card",
            SelectionBoundary::Unknown => "unknown",
        }
    }
}

fn push_effect_resolution(
    effect: &EventEffect,
    known_effects: &mut Vec<KnownEffect>,
    unresolved_effects: &mut Vec<UnresolvedEffect>,
) {
    match effect {
        EventEffect::GainGold(amount) => known_effects.push(KnownEffect::GainGold(*amount)),
        EventEffect::LoseGold(amount) => known_effects.push(KnownEffect::LoseGold(*amount)),
        EventEffect::LoseHp(amount) => known_effects.push(KnownEffect::LoseHp(*amount)),
        EventEffect::LoseMaxHp(amount) => known_effects.push(KnownEffect::LoseMaxHp(*amount)),
        EventEffect::Heal(amount) => known_effects.push(KnownEffect::Heal(*amount)),
        EventEffect::GainMaxHp(amount) => known_effects.push(KnownEffect::GainMaxHp(*amount)),
        EventEffect::ObtainRelic { count, kind } => match kind {
            EventRelicKind::Specific(relic) => {
                known_effects.push(KnownEffect::ObtainSpecificRelic {
                    count: *count,
                    relic: *relic,
                });
            }
            EventRelicKind::RandomRelic => {
                push_random_relic(unresolved_effects, *count, RelicPoolBoundary::AnyRelic)
            }
            EventRelicKind::RandomBook => {
                push_random_relic(unresolved_effects, *count, RelicPoolBoundary::Book)
            }
            EventRelicKind::RandomFace => {
                push_random_relic(unresolved_effects, *count, RelicPoolBoundary::Face)
            }
            EventRelicKind::Unknown => {
                push_random_relic(unresolved_effects, *count, RelicPoolBoundary::Unknown)
            }
        },
        EventEffect::ObtainPotion { count } => {
            unresolved_effects.push(UnresolvedEffect::RandomPotion {
                count: *count,
                visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
            })
        }
        EventEffect::ObtainCard { count, kind } => push_card_effect(
            known_effects,
            unresolved_effects,
            *count,
            *kind,
            CardFactKind::Class,
        ),
        EventEffect::ObtainColorlessCard { count, kind } => push_card_effect(
            known_effects,
            unresolved_effects,
            *count,
            *kind,
            CardFactKind::Colorless,
        ),
        EventEffect::ObtainCurse { count, kind } => match kind {
            EventCardKind::Specific(card) => {
                known_effects.push(KnownEffect::ObtainSpecificCurse {
                    count: *count,
                    card: *card,
                });
            }
            EventCardKind::RandomClassCard => {
                unresolved_effects.push(UnresolvedEffect::RandomCurse {
                    count: *count,
                    pool: CardPoolBoundary::ClassCard,
                    visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
                })
            }
            EventCardKind::RandomColorless => {
                unresolved_effects.push(UnresolvedEffect::RandomCurse {
                    count: *count,
                    pool: CardPoolBoundary::Colorless,
                    visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
                })
            }
            EventCardKind::Unknown => unresolved_effects.push(UnresolvedEffect::RandomCurse {
                count: *count,
                pool: CardPoolBoundary::Unknown,
                visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
            }),
        },
        EventEffect::RemoveCard { count, .. } => {
            known_effects.push(KnownEffect::RemoveCard { count: *count });
        }
        EventEffect::UpgradeCard { count } => {
            known_effects.push(KnownEffect::UpgradeCard { count: *count });
        }
        EventEffect::TransformCard { count } => {
            known_effects.push(KnownEffect::TransformCard { count: *count });
            unresolved_effects.push(UnresolvedEffect::TransformOutput {
                count: *count,
                visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
            });
        }
        EventEffect::DuplicateCard { count } => {
            known_effects.push(KnownEffect::DuplicateCard { count: *count });
        }
        EventEffect::LoseRelic {
            specific,
            starter_only,
        } => {
            if let Some(relic) = specific {
                known_effects.push(KnownEffect::LoseSpecificRelic {
                    relic: *relic,
                    starter_only: *starter_only,
                });
            } else {
                known_effects.push(KnownEffect::LoseRelic {
                    starter_only: *starter_only,
                });
            }
        }
        EventEffect::LoseStarterRelic { specific } => {
            if let Some(relic) = specific {
                known_effects.push(KnownEffect::LoseSpecificRelic {
                    relic: *relic,
                    starter_only: true,
                });
            } else {
                known_effects.push(KnownEffect::LoseRelic { starter_only: true });
            }
        }
        EventEffect::StartCombat => known_effects.push(KnownEffect::StartCombat),
    }
}

#[derive(Clone, Copy)]
enum CardFactKind {
    Class,
    Colorless,
}

fn push_card_effect(
    known_effects: &mut Vec<KnownEffect>,
    unresolved_effects: &mut Vec<UnresolvedEffect>,
    count: usize,
    kind: EventCardKind,
    fact_kind: CardFactKind,
) {
    match kind {
        EventCardKind::Specific(card) => match fact_kind {
            CardFactKind::Class => {
                known_effects.push(KnownEffect::ObtainSpecificCard { count, card })
            }
            CardFactKind::Colorless => {
                known_effects.push(KnownEffect::ObtainSpecificColorlessCard { count, card })
            }
        },
        EventCardKind::RandomClassCard => unresolved_effects.push(UnresolvedEffect::RandomCard {
            count,
            pool: CardPoolBoundary::ClassCard,
            visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
        }),
        EventCardKind::RandomColorless => unresolved_effects.push(UnresolvedEffect::RandomCard {
            count,
            pool: CardPoolBoundary::Colorless,
            visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
        }),
        EventCardKind::Unknown => unresolved_effects.push(UnresolvedEffect::RandomCard {
            count,
            pool: match fact_kind {
                CardFactKind::Class => CardPoolBoundary::Unknown,
                CardFactKind::Colorless => CardPoolBoundary::Colorless,
            },
            visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
        }),
    }
}

fn push_random_relic(
    unresolved_effects: &mut Vec<UnresolvedEffect>,
    count: usize,
    pool: RelicPoolBoundary,
) {
    unresolved_effects.push(UnresolvedEffect::RandomRelic {
        count,
        pool,
        visibility: HiddenInfoVisibility::DistributionKnownResultHiddenUntilResolved,
    });
}

fn followup_boundary(transition: EventOptionTransition) -> Option<FollowupBoundary> {
    match transition {
        EventOptionTransition::None => None,
        EventOptionTransition::AdvanceScreen => Some(FollowupBoundary::EventScreenAdvance),
        EventOptionTransition::Complete => Some(FollowupBoundary::EventComplete),
        EventOptionTransition::OpenSelection(kind) => {
            Some(FollowupBoundary::Selection(selection_boundary(kind)))
        }
        EventOptionTransition::OpenReward => Some(FollowupBoundary::RewardScreen),
        EventOptionTransition::StartCombat => Some(FollowupBoundary::CombatStart),
    }
}

fn selection_boundary(kind: EventSelectionKind) -> SelectionBoundary {
    match kind {
        EventSelectionKind::None => SelectionBoundary::Unknown,
        EventSelectionKind::RemoveCard => SelectionBoundary::RemoveCard,
        EventSelectionKind::UpgradeCard => SelectionBoundary::UpgradeCard,
        EventSelectionKind::TransformCard => SelectionBoundary::TransformCard,
        EventSelectionKind::DuplicateCard => SelectionBoundary::DuplicateCard,
        EventSelectionKind::OfferCard => SelectionBoundary::OfferCard,
    }
}
