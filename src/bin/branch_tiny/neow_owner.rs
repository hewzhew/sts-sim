use std::cmp::Reverse;

use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::state::events::{
    EventCardKind, EventEffect, EventId, EventOptionSemantics, EventOptionTransition,
    EventRelicKind, EventSelectionKind,
};

use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion, OwnerDecision};
use super::owners::executable_choices;

pub(super) fn neow_owner_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> OwnerDecision {
    let Some(event) = session.run_state.event_state.as_ref() else {
        return OwnerDecision::Gap("Neow owner requires event_state".to_string());
    };
    if event.id != EventId::Neow {
        return OwnerDecision::Gap(format!("Neow owner received {:?}", event.id));
    }
    if event.current_screen == 0 {
        return OwnerDecision::Candidates(executable_choices(surface));
    }

    let options = sts_simulator::engine::event_handler::get_event_options(&session.run_state);
    let mut ranked = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let Some(DecisionCandidateKey::EventOption {
                event_id,
                screen,
                option_index,
                ..
            }) = candidate.key
            else {
                return None;
            };
            if event_id != EventId::Neow || screen != event.current_screen {
                return None;
            }
            let option = options.get(option_index)?;
            if option.ui.disabled {
                return None;
            }
            let action = candidate.action.executable_command()?;
            let rank = rank_neow_option(&option.semantics);
            Some((
                rank,
                option_index,
                OwnerChoice {
                    key: candidate.key.clone(),
                    action,
                    label: format!("{} [neow:{}]", candidate.label, rank.reason),
                    annotation: ChoiceAnnotation::None,
                    expansion: rank.expansion(),
                },
            ))
        })
        .collect::<Vec<_>>();

    ranked.sort_by_key(|(rank, option_index, _)| (rank.tier, Reverse(rank.score), *option_index));
    let choices = ranked
        .into_iter()
        .map(|(_, _, choice)| choice)
        .collect::<Vec<_>>();
    if choices.is_empty() {
        OwnerDecision::Gap("Neow owner found no visible option".to_string())
    } else {
        OwnerDecision::Candidates(choices)
    }
}

#[derive(Clone, Copy)]
struct NeowRank {
    tier: u8,
    score: i32,
    reason: &'static str,
    inspect: Option<&'static str>,
}

impl NeowRank {
    fn expansion(self) -> OwnerChoiceExpansion {
        self.inspect
            .map(OwnerChoiceExpansion::InspectOnly)
            .unwrap_or(OwnerChoiceExpansion::AutoAllowed)
    }
}

fn rank_neow_option(semantics: &EventOptionSemantics) -> NeowRank {
    let mut score = 0;
    let mut best_gain = 0;
    let mut reason = "typed_effect";
    let mut has_curse = false;
    let mut boss_swap = false;
    let mut unsupported_followup = unsupported_transition(semantics.transition);

    for effect in &semantics.effects {
        match *effect {
            EventEffect::GainGold(gold) => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                80 + gold.max(0),
                "gain_gold",
            ),
            EventEffect::GainMaxHp(hp) => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                hp.max(0) * 20,
                "gain_max_hp",
            ),
            EventEffect::ObtainPotion { count } => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                40 * count as i32,
                "gain_potions",
            ),
            EventEffect::OfferCards { kind, .. } => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                offer_cards_value(kind),
                "offer_cards",
            ),
            EventEffect::ObtainCard { kind, .. }
            | EventEffect::ObtainColorlessCard { kind, .. } => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                obtain_card_value(kind),
                "obtain_card",
            ),
            EventEffect::ObtainRelic { kind, .. } => add_gain(
                &mut score,
                &mut best_gain,
                &mut reason,
                relic_value(kind),
                "obtain_relic",
            ),
            EventEffect::RemoveCard { count, .. } => {
                if count != 1 {
                    unsupported_followup = true;
                }
                add_gain(
                    &mut score,
                    &mut best_gain,
                    &mut reason,
                    if count == 1 { 260 } else { 360 },
                    "remove_card",
                );
            }
            EventEffect::UpgradeCard { count } => {
                unsupported_followup = true;
                add_gain(
                    &mut score,
                    &mut best_gain,
                    &mut reason,
                    160 * count as i32,
                    "upgrade_card",
                );
            }
            EventEffect::TransformCard { count } => {
                unsupported_followup = true;
                add_gain(
                    &mut score,
                    &mut best_gain,
                    &mut reason,
                    120 * count as i32,
                    "transform_card",
                );
            }
            EventEffect::LoseGold(gold) => score -= gold.min(180).max(0),
            EventEffect::LoseHp(hp) => score -= hp.max(0) * 8,
            EventEffect::LoseMaxHp(hp) => score -= hp.max(0) * 25,
            EventEffect::ObtainCurse { .. } => {
                has_curse = true;
                score -= 420;
            }
            EventEffect::LoseStarterRelic { .. } => {
                boss_swap = true;
                score -= 220;
            }
            EventEffect::RandomOutcome { .. }
            | EventEffect::StartCombat
            | EventEffect::DuplicateCard { .. }
            | EventEffect::UpgradeAllCards
            | EventEffect::LoseRelic { .. }
            | EventEffect::GainGoldRange { .. }
            | EventEffect::Heal(_) => {}
        }
    }

    if unsupported_followup {
        return rank(2, score, reason, Some("neow follow-up owner unsupported"));
    }
    if boss_swap {
        return rank(2, score, "boss_swap", Some("neow boss swap is probe-only"));
    }
    if has_curse {
        return rank(2, score, reason, Some("neow curse downside is probe-only"));
    }
    rank(0, score, reason, None)
}

fn add_gain(
    score: &mut i32,
    best_gain: &mut i32,
    reason: &mut &'static str,
    amount: i32,
    label: &'static str,
) {
    *score += amount;
    if amount > *best_gain {
        *best_gain = amount;
        *reason = label;
    }
}

fn unsupported_transition(transition: EventOptionTransition) -> bool {
    matches!(
        transition,
        EventOptionTransition::OpenSelection(
            EventSelectionKind::UpgradeCard
                | EventSelectionKind::TransformCard
                | EventSelectionKind::DuplicateCard
                | EventSelectionKind::OfferCard
        ) | EventOptionTransition::StartCombat
    )
}

fn offer_cards_value(kind: EventCardKind) -> i32 {
    match kind {
        EventCardKind::RandomClassRare => 360,
        EventCardKind::RandomClassCommonOrUncommon => 320,
        EventCardKind::RandomColorlessRare => 260,
        EventCardKind::RandomColorlessUncommon | EventCardKind::RandomColorless => 180,
        EventCardKind::Specific(_) | EventCardKind::RandomClassCard => 260,
        EventCardKind::Unknown => 160,
    }
}

fn obtain_card_value(kind: EventCardKind) -> i32 {
    match kind {
        EventCardKind::RandomClassRare => 330,
        EventCardKind::Specific(_) => 260,
        EventCardKind::RandomClassCommonOrUncommon | EventCardKind::RandomClassCard => 240,
        EventCardKind::RandomColorlessRare => 220,
        EventCardKind::RandomColorlessUncommon | EventCardKind::RandomColorless => 160,
        EventCardKind::Unknown => 120,
    }
}

fn relic_value(kind: EventRelicKind) -> i32 {
    match kind {
        EventRelicKind::RandomRareRelic => 380,
        EventRelicKind::RandomUncommonRelic | EventRelicKind::RandomRelic => 330,
        EventRelicKind::RandomCommonRelic => 300,
        EventRelicKind::Specific(_) => 260,
        EventRelicKind::RandomBossRelic => 240,
        EventRelicKind::RandomShopRelic
        | EventRelicKind::RandomBook
        | EventRelicKind::RandomFace => 220,
        EventRelicKind::Unknown => 180,
    }
}

fn rank(tier: u8, score: i32, reason: &'static str, inspect: Option<&'static str>) -> NeowRank {
    NeowRank {
        tier,
        score,
        reason,
        inspect,
    }
}
