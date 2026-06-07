use crate::content::relics::RelicId;
use crate::state::events::{EventCardKind, EventEffect, EventOption, EventRelicKind};

use super::types::{
    NeowCandidateTraceV1, NeowChoiceClassV1, NeowChoiceInputV1, NeowDecisionInputV1,
    NeowDecisionTraceV1, NeowGuidanceConfigV1, NeowMapFeaturesV1, NeowScoreTermsV1,
};

pub fn choices_from_event_options_v1(options: &[EventOption]) -> Vec<NeowChoiceInputV1> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            NeowChoiceInputV1::from_effects(
                index,
                option.ui.text.clone(),
                option.semantics.effects.clone(),
            )
        })
        .collect()
}

pub fn rank_neow_choices_v1(input: NeowDecisionInputV1) -> NeowDecisionTraceV1 {
    let mut candidates = input
        .choices
        .iter()
        .map(|choice| {
            let terms = score_neow_choice(choice, &input.player_class, &input.map, &input.config);
            NeowCandidateTraceV1 {
                index: choice.index,
                label: choice.label.clone(),
                class: choice.class,
                terms,
                total: terms.total(),
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .total
            .partial_cmp(&left.total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.index.cmp(&right.index))
    });
    NeowDecisionTraceV1 {
        label_role: "behavior_policy_not_teacher",
        map: input.map,
        selected_index: (!candidates.is_empty()).then_some(0),
        candidates,
    }
}

pub(super) fn classify_neow_choice(effects: &[EventEffect]) -> NeowChoiceClassV1 {
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::Specific(RelicId::NeowsLament),
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::Lament;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::LoseStarterRelic { .. }))
        && effects.iter().any(|effect| {
            matches!(
                effect,
                EventEffect::ObtainRelic {
                    kind: EventRelicKind::RandomBossRelic,
                    ..
                }
            )
        })
    {
        return NeowChoiceClassV1::BossSwap;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::RandomCommonRelic,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::CommonRelic;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::RandomRareRelic,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::RareRelic;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::GainGold(amount) if *amount > 0))
    {
        return NeowChoiceClassV1::Gold;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::ObtainPotion { .. }))
    {
        return NeowChoiceClassV1::Potions;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainCard {
                kind: EventCardKind::RandomClassRare,
                ..
            } | EventEffect::OfferCards {
                kind: EventCardKind::RandomClassRare,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::RareCardReward;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::OfferCards {
                kind: EventCardKind::RandomColorless
                    | EventCardKind::RandomColorlessUncommon
                    | EventCardKind::RandomColorlessRare,
                ..
            } | EventEffect::ObtainColorlessCard { .. }
        )
    }) {
        return NeowChoiceClassV1::ColorlessCardReward;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainCard { .. } | EventEffect::OfferCards { .. }
        )
    }) {
        return NeowChoiceClassV1::CardReward;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::RemoveCard { .. }))
    {
        return NeowChoiceClassV1::Remove;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::UpgradeCard { .. }))
    {
        return NeowChoiceClassV1::Upgrade;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::TransformCard { .. }))
    {
        return NeowChoiceClassV1::Transform;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::GainMaxHp(amount) if *amount > 0))
    {
        return NeowChoiceClassV1::MaxHp;
    }
    NeowChoiceClassV1::Unknown
}

fn score_neow_choice(
    choice: &NeowChoiceInputV1,
    player_class: &str,
    map: &NeowMapFeaturesV1,
    config: &NeowGuidanceConfigV1,
) -> NeowScoreTermsV1 {
    let mut terms = NeowScoreTermsV1 {
        immediate_power: base_immediate_power(choice.class),
        downside_cost: downside_cost(&choice.effects),
        variance_penalty: variance_penalty(choice.class),
        ..NeowScoreTermsV1::default()
    };

    match choice.class {
        NeowChoiceClassV1::Gold => {
            if map.early_shop_available {
                terms.shop_convertibility += config.early_shop_gold_bonus;
            }
            if map.shop_before_first_elite {
                terms.shop_convertibility += config.shop_before_elite_gold_bonus;
            }
        }
        NeowChoiceClassV1::Lament => {
            if map.lament_elite_snipe_candidate {
                terms.first_elite_security += config.lament_elite_snipe_bonus;
            } else if map.early_elite_available {
                terms.first_elite_security += 1.0;
            }
        }
        NeowChoiceClassV1::Potions => {
            if map.early_elite_available {
                terms.first_elite_security += config.early_elite_potion_bonus;
            }
        }
        NeowChoiceClassV1::CardReward
        | NeowChoiceClassV1::RareCardReward
        | NeowChoiceClassV1::ColorlessCardReward
        | NeowChoiceClassV1::CommonRelic
        | NeowChoiceClassV1::RareRelic
        | NeowChoiceClassV1::Transform => {
            if map.early_elite_available {
                terms.first_elite_security += config.early_elite_immediate_bonus;
            }
        }
        NeowChoiceClassV1::BossSwap => {
            terms.path_flexibility += map.path_flexibility;
            if map.path_flexibility < 0.4 {
                terms.downside_cost -= config.low_flex_boss_swap_penalty;
            }
            terms.variance_penalty -= config.boss_swap_variance_penalty;
        }
        _ => {}
    }

    terms.character_synergy += character_synergy(choice.class, player_class, config);
    terms
}

fn base_immediate_power(class: NeowChoiceClassV1) -> f32 {
    match class {
        NeowChoiceClassV1::Lament => 1.0,
        NeowChoiceClassV1::MaxHp => 1.2,
        NeowChoiceClassV1::Gold => 1.4,
        NeowChoiceClassV1::Potions => 2.4,
        NeowChoiceClassV1::CardReward => 2.6,
        NeowChoiceClassV1::RareCardReward => 2.9,
        NeowChoiceClassV1::ColorlessCardReward => 2.2,
        NeowChoiceClassV1::CommonRelic => 2.8,
        NeowChoiceClassV1::RareRelic => 2.7,
        NeowChoiceClassV1::BossSwap => 2.2,
        NeowChoiceClassV1::Remove => 1.2,
        NeowChoiceClassV1::Upgrade => 1.1,
        NeowChoiceClassV1::Transform => 2.3,
        NeowChoiceClassV1::Unknown => 0.0,
    }
}

fn variance_penalty(class: NeowChoiceClassV1) -> f32 {
    match class {
        NeowChoiceClassV1::BossSwap => -0.4,
        NeowChoiceClassV1::RareRelic | NeowChoiceClassV1::Transform => -0.3,
        NeowChoiceClassV1::ColorlessCardReward => -0.2,
        _ => 0.0,
    }
}

fn character_synergy(
    class: NeowChoiceClassV1,
    player_class: &str,
    config: &NeowGuidanceConfigV1,
) -> f32 {
    match player_class.to_ascii_lowercase().as_str() {
        "ironclad" | "red" => match class {
            NeowChoiceClassV1::BossSwap => -config.ironclad_boss_swap_penalty,
            NeowChoiceClassV1::RareCardReward => 0.4,
            NeowChoiceClassV1::CommonRelic => 0.1,
            _ => 0.0,
        },
        "silent" | "green" => match class {
            NeowChoiceClassV1::Potions
            | NeowChoiceClassV1::CardReward
            | NeowChoiceClassV1::Transform
            | NeowChoiceClassV1::Gold => 0.4,
            NeowChoiceClassV1::Remove => -0.2,
            _ => 0.0,
        },
        "defect" | "blue" => match class {
            NeowChoiceClassV1::BossSwap => 0.2,
            NeowChoiceClassV1::CommonRelic | NeowChoiceClassV1::CardReward => 0.2,
            _ => 0.0,
        },
        "watcher" | "purple" => match class {
            NeowChoiceClassV1::Remove | NeowChoiceClassV1::Transform => 0.8,
            NeowChoiceClassV1::Upgrade => 0.5,
            NeowChoiceClassV1::BossSwap => -0.4,
            _ => 0.0,
        },
        _ => 0.0,
    }
}

fn downside_cost(effects: &[EventEffect]) -> f32 {
    effects.iter().map(effect_downside_cost).sum()
}

fn effect_downside_cost(effect: &EventEffect) -> f32 {
    match effect {
        EventEffect::LoseHp(amount) => -((*amount as f32) / 10.0).min(2.5),
        EventEffect::LoseMaxHp(amount) => -((*amount as f32) / 5.0).min(2.2),
        EventEffect::LoseGold(amount) => -((*amount as f32) / 100.0).min(2.5),
        EventEffect::ObtainCurse { .. } => -1.8,
        EventEffect::LoseStarterRelic { .. } => -0.8,
        _ => 0.0,
    }
}
