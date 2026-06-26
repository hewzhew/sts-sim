use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::strategic::{AcquisitionVerdict, CandidateAction};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceDecisionSignalV1, BranchExperimentRewardOptionPortfolioEntryV1,
    BranchExperimentRewardOptionPortfolioV1,
    BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1,
};
use crate::eval::branch_experiment_trajectory::summarize_branch_trajectory_v1;
use crate::eval::run_control::RunControlSession;
use crate::runtime::action::CardDestination;
use crate::state::core::{EngineState, PendingChoice};
use crate::state::rewards::{RewardCard, RewardItem};

#[derive(Clone, Debug)]
pub(crate) struct CardRewardBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) upgrades: Option<u8>,
    pub(crate) source: CardRewardBranchOptionSource,
    pub(crate) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CardRewardBranchOptionSource {
    PermanentReward,
    CombatGeneratedToHand,
    SkipCardReward,
    SingingBowl,
}

#[derive(Clone, Debug)]
pub(crate) struct CardRewardBranchOptionSelection {
    pub(crate) options: Vec<CardRewardBranchOption>,
    pub(crate) portfolio: Option<BranchExperimentRewardOptionPortfolioV1>,
}

#[derive(Clone, Debug)]
pub(crate) struct CardRewardPortfolioContext {
    pub(crate) depth: usize,
    pub(crate) frontier_key: String,
    pub(crate) boundary_title: String,
}

pub(crate) fn card_reward_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CardRewardBranchOption>> {
    let source = card_reward_option_source(session)?;
    let cards = active_or_visible_reward_cards(session)?;
    let options = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| CardRewardBranchOption {
            label: format_reward_card_label(card),
            command: match source {
                CardRewardBranchOptionSource::PermanentReward => format!("rp {idx}"),
                CardRewardBranchOptionSource::CombatGeneratedToHand => format!("choose {idx}"),
                CardRewardBranchOptionSource::SkipCardReward
                | CardRewardBranchOptionSource::SingingBowl => {
                    unreachable!("card reward card options cannot use non-card reward sources")
                }
            },
            card: Some(card.id),
            upgrades: Some(card.upgrades),
            source,
            decision_signal: None,
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    Some(options)
}

pub(crate) fn card_reward_decline_branch_options(
    session: &RunControlSession,
    include_event_reward_skip: bool,
) -> Vec<CardRewardBranchOption> {
    let mut options = Vec::new();
    if has_singing_bowl(session) && card_reward_bowl_available(session) {
        options.push(CardRewardBranchOption {
            label: "Singing Bowl | gain 2 max HP".to_string(),
            command: "bowl".to_string(),
            card: None,
            upgrades: None,
            source: CardRewardBranchOptionSource::SingingBowl,
            decision_signal: None,
        });
        return options;
    }
    if include_event_reward_skip || !completed_event_reward_skip(session) {
        if let Some(command) = card_reward_skip_command(session) {
            options.push(CardRewardBranchOption {
                label: "Skip card reward".to_string(),
                command,
                card: None,
                upgrades: None,
                source: CardRewardBranchOptionSource::SkipCardReward,
                decision_signal: None,
            });
        }
    }
    options
}

pub(crate) fn select_card_reward_branch_options_for_session(
    session: &RunControlSession,
    mut options: Vec<CardRewardBranchOption>,
    max_reward_options_per_branch: Option<usize>,
    portfolio_context: Option<CardRewardPortfolioContext>,
) -> CardRewardBranchOptionSelection {
    attach_card_reward_decision_signals(session, &mut options);

    if options
        .iter()
        .all(|option| option.source == CardRewardBranchOptionSource::CombatGeneratedToHand)
    {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    }
    let Some(limit) = max_reward_options_per_branch else {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    };
    select_card_reward_branch_options_with_limit_and_strategy(
        options,
        limit,
        portfolio_context,
        Some(session),
    )
}

fn select_card_reward_branch_options_with_limit_and_strategy(
    options: Vec<CardRewardBranchOption>,
    limit: usize,
    portfolio_context: Option<CardRewardPortfolioContext>,
    session: Option<&RunControlSession>,
) -> CardRewardBranchOptionSelection {
    let capped_limit = limit.min(options.len());
    if options.len() <= capped_limit {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    }

    let strategic_retention_keys = reward_option_strategic_retention_keys(session, &options);
    let mut annotated = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let class_key = reward_option_semantic_class_for_option(option);
            let retention_key = strategic_retention_keys
                .get(&index)
                .cloned()
                .unwrap_or_else(RewardOptionStrategicRetentionKey::missing);
            RewardOptionAnnotated {
                index,
                verdict_retention_order: retention_key.verdict_retention_order,
                strategic_score_sort_key: retention_key.strategic_score_sort_key,
                class_key,
                verdict_label: retention_key.verdict_label,
                is_decline: matches!(
                    option.source,
                    CardRewardBranchOptionSource::SkipCardReward
                        | CardRewardBranchOptionSource::SingingBowl
                ),
            }
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| {
        left.verdict_retention_order
            .cmp(&right.verdict_retention_order)
            .then_with(|| {
                left.strategic_score_sort_key
                    .cmp(&right.strategic_score_sort_key)
            })
            .then_with(|| left.index.cmp(&right.index))
    });

    let reject_order = AcquisitionVerdict::Reject.retention_order();
    let mut selected = select_reward_option_indices(&annotated, limit, reject_order);
    selected = preserve_decline_option_indices(&annotated, selected, limit, reject_order);

    selected.sort_unstable();
    let selected_indices = selected.iter().copied().collect::<BTreeSet<_>>();
    let portfolio = portfolio_context.map(|context| {
        reward_option_portfolio_report(
            context.depth,
            context.frontier_key,
            context.boundary_title,
            limit,
            &options,
            &annotated,
            &selected_indices,
        )
    });
    let options = options
        .into_iter()
        .enumerate()
        .filter_map(|(index, option)| selected_indices.contains(&index).then_some(option))
        .collect();
    CardRewardBranchOptionSelection { options, portfolio }
}

#[derive(Clone, Debug)]
struct RewardOptionAnnotated {
    index: usize,
    verdict_retention_order: usize,
    strategic_score_sort_key: i32,
    class_key: String,
    verdict_label: String,
    is_decline: bool,
}

fn select_reward_option_indices(
    annotated: &[RewardOptionAnnotated],
    limit: usize,
    reject_order: usize,
) -> Vec<usize> {
    let mut selected = Vec::new();
    let tiers = annotated
        .iter()
        .filter(|entry| entry.verdict_retention_order < reject_order)
        .map(|entry| entry.verdict_retention_order)
        .collect::<BTreeSet<_>>();

    for tier in tiers {
        if selected.len() >= limit {
            break;
        }
        let tier_entries = annotated
            .iter()
            .filter(|entry| entry.verdict_retention_order == tier)
            .collect::<Vec<_>>();
        let mut selected_classes = BTreeSet::new();
        for entry in &tier_entries {
            if selected.len() >= limit {
                break;
            }
            if selected_classes.insert(entry.class_key.clone()) {
                selected.push(entry.index);
            }
        }
        for entry in &tier_entries {
            if selected.len() >= limit {
                break;
            }
            if !selected.contains(&entry.index) {
                selected.push(entry.index);
            }
        }
    }

    if selected.is_empty() {
        for entry in annotated {
            if selected.len() >= limit {
                break;
            }
            if !selected.contains(&entry.index) {
                selected.push(entry.index);
            }
        }
    }

    selected
}

fn preserve_decline_option_indices(
    annotated: &[RewardOptionAnnotated],
    mut selected: Vec<usize>,
    limit: usize,
    reject_order: usize,
) -> Vec<usize> {
    if limit < 2
        || selected.iter().any(|index| {
            annotated_entry_by_option_index(annotated, *index).is_some_and(|entry| entry.is_decline)
        })
        || selected.len() < limit.min(annotated.len())
    {
        return selected;
    }

    let Some(decline) = annotated
        .iter()
        .filter(|entry| entry.is_decline)
        .filter(|entry| entry.verdict_retention_order < reject_order)
        .min_by(|left, right| {
            left.verdict_retention_order
                .cmp(&right.verdict_retention_order)
                .then_with(|| {
                    left.strategic_score_sort_key
                        .cmp(&right.strategic_score_sort_key)
                })
                .then_with(|| left.index.cmp(&right.index))
        })
    else {
        return selected;
    };

    let strong_take_order = AcquisitionVerdict::StrongTake.retention_order();
    let Some(remove_position) = selected
        .iter()
        .enumerate()
        .filter_map(|(position, index)| {
            annotated_entry_by_option_index(annotated, *index).map(|entry| (position, entry))
        })
        .filter(|(_, entry)| !entry.is_decline)
        .filter(|(_, entry)| entry.verdict_retention_order > strong_take_order)
        .max_by(|(_, left), (_, right)| {
            left.verdict_retention_order
                .cmp(&right.verdict_retention_order)
                .then_with(|| {
                    left.strategic_score_sort_key
                        .cmp(&right.strategic_score_sort_key)
                })
                .then_with(|| right.index.cmp(&left.index))
        })
        .map(|(position, _)| position)
    else {
        return selected;
    };

    selected.remove(remove_position);
    selected.push(decline.index);
    selected
}

fn annotated_entry_by_option_index(
    annotated: &[RewardOptionAnnotated],
    index: usize,
) -> Option<&RewardOptionAnnotated> {
    annotated.iter().find(|entry| entry.index == index)
}

fn reward_option_portfolio_report(
    depth: usize,
    frontier_key: String,
    boundary_title: String,
    max_reward_options_per_branch: usize,
    options: &[CardRewardBranchOption],
    annotated: &[RewardOptionAnnotated],
    selected_indices: &BTreeSet<usize>,
) -> BranchExperimentRewardOptionPortfolioV1 {
    let class_by_index = annotated
        .iter()
        .map(|entry| {
            (
                entry.index,
                format!(
                    "strategic_retention=verdict_order:{}:verdict:{}:class:{}",
                    entry.verdict_retention_order, entry.verdict_label, entry.class_key
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut selected_options = Vec::new();
    let mut pruned_options = Vec::new();

    for (index, option) in options.iter().enumerate() {
        let entry = BranchExperimentRewardOptionPortfolioEntryV1 {
            command: option.command.clone(),
            label: option.label.clone(),
            semantic_class: class_by_index
                .get(&index)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        };
        if selected_indices.contains(&index) {
            selected_options.push(entry);
        } else {
            pruned_options.push(entry);
        }
    }

    BranchExperimentRewardOptionPortfolioV1 {
        branch_id: String::new(),
        branch_choices: Vec::new(),
        branch_commands: Vec::new(),
        depth,
        frontier_key,
        boundary_title,
        max_reward_options_per_branch,
        original_count: options.len(),
        selected_count: selected_options.len(),
        selected_options,
        pruned_options,
    }
}

#[derive(Clone, Debug)]
struct RewardOptionStrategicRetentionKey {
    verdict_retention_order: usize,
    strategic_score_sort_key: i32,
    verdict_label: String,
    score_milli: Option<i32>,
    preferred: bool,
}

fn attach_card_reward_decision_signals(
    session: &RunControlSession,
    options: &mut [CardRewardBranchOption],
) {
    if options
        .iter()
        .all(|option| option.source == CardRewardBranchOptionSource::CombatGeneratedToHand)
    {
        return;
    }

    let strategic_retention_keys = reward_option_strategic_retention_keys(Some(session), options);
    for (index, option) in options.iter_mut().enumerate() {
        option.decision_signal = strategic_retention_keys
            .get(&index)
            .and_then(RewardOptionStrategicRetentionKey::decision_signal);
    }
}

fn reward_option_strategic_retention_keys(
    session: Option<&RunControlSession>,
    options: &[CardRewardBranchOption],
) -> BTreeMap<usize, RewardOptionStrategicRetentionKey> {
    let Some(session) = session else {
        return options
            .iter()
            .enumerate()
            .map(|(index, _)| (index, RewardOptionStrategicRetentionKey::unavailable()))
            .collect();
    };
    let mut option_card_indices = BTreeMap::new();
    let cards = options
        .iter()
        .enumerate()
        .filter_map(|(option_index, option)| {
            let card = option.card?;
            let card_index = option_card_indices.len();
            option_card_indices.insert(option_index, card_index);
            Some(RewardCard::new(card, option.upgrades.unwrap_or_default()))
        })
        .collect::<Vec<_>>();
    let context =
        crate::ai::card_reward_policy_v1::build_card_reward_decision_context_with_current_route_v1(
            &session.run_state,
            &session.engine_state,
            cards,
        );
    let trace = crate::ai::strategic::strategic_trace_for_card_reward(&context);
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let action = candidate_action_for_reward_option(index, option, &option_card_indices);
            let order = trace
                .compiled_for_action(&action)
                .map(|compiled| {
                    let preferred = trace
                        .would_choose
                        .as_ref()
                        .is_some_and(|preferred| preferred == &action);
                    RewardOptionStrategicRetentionKey {
                        score_milli: Some((compiled.score * 1000.0).round() as i32),
                        verdict_retention_order: compiled.verdict.retention_order(),
                        strategic_score_sort_key: -((compiled.score * 1000.0).round() as i32),
                        verdict_label: format!("{:?}", compiled.verdict),
                        preferred,
                    }
                })
                .unwrap_or_else(RewardOptionStrategicRetentionKey::missing);
            (index, order)
        })
        .collect()
}

fn candidate_action_for_reward_option(
    option_index: usize,
    option: &CardRewardBranchOption,
    option_card_indices: &BTreeMap<usize, usize>,
) -> CandidateAction {
    match option.source {
        CardRewardBranchOptionSource::PermanentReward
        | CardRewardBranchOptionSource::CombatGeneratedToHand => CandidateAction::TakeCard {
            index: option_card_indices
                .get(&option_index)
                .copied()
                .unwrap_or(option_index),
            card: option
                .card
                .expect("card reward option source should carry a card"),
        },
        CardRewardBranchOptionSource::SkipCardReward => CandidateAction::SkipCardReward,
        CardRewardBranchOptionSource::SingingBowl => {
            CandidateAction::TakeSingingBowl { max_hp_gain: 2 }
        }
    }
}

impl RewardOptionStrategicRetentionKey {
    fn unavailable() -> Self {
        Self {
            verdict_retention_order: 0,
            strategic_score_sort_key: 0,
            verdict_label: "strategy_unavailable".to_string(),
            score_milli: None,
            preferred: false,
        }
    }

    fn missing() -> Self {
        Self {
            verdict_retention_order: AcquisitionVerdict::Reject.retention_order(),
            strategic_score_sort_key: 0,
            verdict_label: "missing_strategic_candidate".to_string(),
            score_milli: None,
            preferred: false,
        }
    }

    fn decision_signal(&self) -> Option<BranchExperimentChoiceDecisionSignalV1> {
        let score = self.score_milli?;
        Some(BranchExperimentChoiceDecisionSignalV1 {
            source: BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1.to_string(),
            verdict: self.verdict_label.clone(),
            tier: self.verdict_retention_order as i32,
            score,
            confidence_milli: 650,
            component_net_rank: score.clamp(-1_200, 1_200),
            preferred: self.preferred,
        })
    }
}

pub(super) fn reward_option_semantic_class(profile: &CardRewardSemanticProfileV1) -> String {
    let signature = summarize_branch_trajectory_v1(std::slice::from_ref(profile));
    let setup = join_or_dash(&signature.setup_keys);
    let package = join_or_dash(&signature.package_keys);
    if !signature.setup_keys.is_empty() && !signature.package_keys.is_empty() {
        return format!("closed_package:{setup}->{package}");
    }
    if !signature.package_keys.is_empty() {
        return format!("payoff:{package}");
    }
    if !signature.setup_keys.is_empty() {
        return format!("setup:{setup}");
    }
    if signature.defense_picks > 0 || signature.draw_energy_picks > 0 {
        return format!("stabilizer:{}", stabilizer_role_key(profile));
    }
    if signature.transition_frontload_picks > 0 {
        return "pure_transition_frontload".to_string();
    }
    "other".to_string()
}

fn reward_option_semantic_class_for_option(option: &CardRewardBranchOption) -> String {
    match option.source {
        CardRewardBranchOptionSource::PermanentReward
        | CardRewardBranchOptionSource::CombatGeneratedToHand => {
            let profile = card_reward_semantic_profile_v1(&RewardCard::new(
                option
                    .card
                    .expect("card reward option source should carry a card"),
                option.upgrades.unwrap_or_default(),
            ));
            reward_option_semantic_class(&profile)
        }
        CardRewardBranchOptionSource::SkipCardReward => "decline:skip_card_reward".to_string(),
        CardRewardBranchOptionSource::SingingBowl => "decline:singing_bowl".to_string(),
    }
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join("+")
    }
}

fn stabilizer_role_key(profile: &CardRewardSemanticProfileV1) -> String {
    let roles = profile
        .roles
        .iter()
        .filter(|role| {
            !matches!(
                role,
                CardRewardSemanticRoleV1::FrontloadDamage
                    | CardRewardSemanticRoleV1::AoeDamage
                    | CardRewardSemanticRoleV1::PackagePayoff
            )
        })
        .map(|role| format!("{role:?}"))
        .collect::<Vec<_>>();
    if roles.is_empty() {
        "none".to_string()
    } else {
        roles.join("+")
    }
}

pub(crate) fn active_or_visible_reward_cards(
    session: &RunControlSession,
) -> Option<Vec<RewardCard>> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => reward
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward)),
        EngineState::RewardOverlay { reward_state, .. } => reward_state
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward_state)),
        EngineState::PendingChoice(PendingChoice::CardRewardSelect {
            cards,
            destination: CardDestination::Hand,
            can_skip: false,
        }) => Some(
            cards
                .iter()
                .copied()
                .map(|card| RewardCard::new(card, 0))
                .collect(),
        ),
        _ => None,
    }
}

fn card_reward_option_source(session: &RunControlSession) -> Option<CardRewardBranchOptionSource> {
    match &session.engine_state {
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            Some(CardRewardBranchOptionSource::PermanentReward)
        }
        EngineState::PendingChoice(PendingChoice::CardRewardSelect {
            destination: CardDestination::Hand,
            can_skip: false,
            ..
        }) => Some(CardRewardBranchOptionSource::CombatGeneratedToHand),
        _ => None,
    }
}

fn card_reward_skip_command(session: &RunControlSession) -> Option<String> {
    let EngineState::RewardScreen(reward) = &session.engine_state else {
        return None;
    };
    if reward.pending_card_choice.is_some() {
        return None;
    }
    let reward_index = reward
        .items
        .iter()
        .position(|item| matches!(item, RewardItem::Card { .. }))?;
    Some(format!("branch-skip-card-reward {reward_index}"))
}

fn completed_event_reward_skip(session: &RunControlSession) -> bool {
    session
        .run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.completed && !event.combat_pending)
}

fn card_reward_bowl_available(session: &RunControlSession) -> bool {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => {
            reward.pending_card_choice.is_some() || reward.has_card_reward_item()
        }
        EngineState::RewardOverlay { reward_state, .. } => {
            reward_state.pending_card_choice.is_some() || reward_state.has_card_reward_item()
        }
        _ => false,
    }
}

fn has_singing_bowl(session: &RunControlSession) -> bool {
    session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::SingingBowl)
}

fn first_visible_card_reward(
    reward: &crate::state::rewards::RewardState,
) -> Option<Vec<RewardCard>> {
    reward.items.iter().find_map(|item| match item {
        RewardItem::Card { cards } => Some(cards.clone()),
        _ => None,
    })
}

pub(crate) fn card_offer_labels(cards: Vec<RewardCard>) -> Vec<String> {
    cards
        .into_iter()
        .map(|card| format_reward_card_label(&card))
        .collect()
}

pub(super) fn format_reward_card_label(card: &RewardCard) -> String {
    let name = crate::content::cards::get_card_definition(card.id).name;
    match card.upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        preserve_decline_option_indices, select_reward_option_indices, RewardOptionAnnotated,
    };
    use crate::ai::strategic::AcquisitionVerdict;

    fn entry(
        index: usize,
        verdict: AcquisitionVerdict,
        strategic_score_sort_key: i32,
        class_key: &str,
    ) -> RewardOptionAnnotated {
        RewardOptionAnnotated {
            index,
            verdict_retention_order: verdict.retention_order(),
            strategic_score_sort_key,
            class_key: class_key.to_string(),
            verdict_label: format!("{verdict:?}"),
            is_decline: false,
        }
    }

    fn decline_entry(
        index: usize,
        verdict: AcquisitionVerdict,
        strategic_score_sort_key: i32,
    ) -> RewardOptionAnnotated {
        RewardOptionAnnotated {
            is_decline: true,
            ..entry(index, verdict, strategic_score_sort_key, "decline")
        }
    }

    #[test]
    fn reward_option_diversity_does_not_cross_strategic_verdict_tiers() {
        let annotated = vec![
            entry(0, AcquisitionVerdict::StrongTake, -1000, "frontload"),
            entry(1, AcquisitionVerdict::StrongTake, -900, "frontload"),
            entry(2, AcquisitionVerdict::ContextTake, -2000, "block"),
        ];

        let selected = select_reward_option_indices(
            &annotated,
            2,
            AcquisitionVerdict::Reject.retention_order(),
        );

        assert_eq!(
            selected,
            vec![0, 1],
            "semantic diversity may break ties within a strategic tier, but must not promote a lower verdict over an available higher-verdict candidate"
        );
    }

    #[test]
    fn reward_option_decline_can_replace_weaker_capped_context_branch() {
        let reject_order = AcquisitionVerdict::Reject.retention_order();
        let annotated = vec![
            entry(0, AcquisitionVerdict::StrongTake, -1000, "frontload"),
            entry(1, AcquisitionVerdict::ContextTake, -900, "engine"),
            decline_entry(2, AcquisitionVerdict::ContextTake, -800),
        ];

        let selected = preserve_decline_option_indices(&annotated, vec![0, 1], 2, reject_order);

        assert_eq!(
            selected,
            vec![0, 2],
            "a capped branch portfolio should retain one clean/decline representative without displacing the strongest take candidate"
        );
    }

    #[test]
    fn reward_option_decline_does_not_displace_strong_take_branches() {
        let reject_order = AcquisitionVerdict::Reject.retention_order();
        let annotated = vec![
            entry(0, AcquisitionVerdict::StrongTake, -1000, "frontload"),
            entry(1, AcquisitionVerdict::StrongTake, -900, "engine"),
            decline_entry(2, AcquisitionVerdict::ContextTake, -800),
        ];

        let selected = preserve_decline_option_indices(&annotated, vec![0, 1], 2, reject_order);

        assert_eq!(
            selected,
            vec![0, 1],
            "decline preservation must not become a hidden rule that overrides strong acquisition verdicts"
        );
    }
}
