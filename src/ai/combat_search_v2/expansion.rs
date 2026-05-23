use super::*;
use std::collections::BTreeMap;

const LARGEST_GROUP_SAMPLE_LIMIT: usize = 8;

#[derive(Default)]
pub(super) struct ActionExpansionDiagnosticsCollector {
    states_observed: u64,
    total_atomic_actions: u64,
    total_fanout_groups: u64,
    fanout_groups_max: usize,
    max_group_size: usize,
    kind_counts: BTreeMap<ActionExpansionKind, MutableKindCount>,
    largest_groups: Vec<ActionExpansionGroupObservation>,
}

#[derive(Clone, Debug)]
pub(super) struct ActionExpansionSummary {
    pub(super) action_count: usize,
    pub(super) group_count: usize,
    groups: Vec<ActionExpansionGroupSummary>,
}

#[derive(Clone, Debug)]
struct ActionExpansionGroupSummary {
    key: ActionExpansionGroupKey,
    action_count: usize,
}

#[derive(Clone, Debug)]
struct ActionExpansionGroupObservation {
    observed_at_state_query: u64,
    key: ActionExpansionGroupKey,
    action_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActionExpansionGroupKey {
    kind: ActionExpansionKind,
    signature: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActionExpansionKind {
    PlayCard,
    EndTurn,
    UsePotion,
    DiscardPotion,
    DiscoverChoice,
    HandSelect,
    GridSelect,
    ScryDiscard,
    Cancel,
    Proceed,
    Other,
}

#[derive(Clone, Debug, Default)]
struct MutableKindCount {
    atomic_actions: u64,
    fanout_groups: u64,
    max_group_size: usize,
}

impl ActionExpansionDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &ActionExpansionSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.total_atomic_actions = self
            .total_atomic_actions
            .saturating_add(summary.action_count as u64);
        self.total_fanout_groups = self
            .total_fanout_groups
            .saturating_add(summary.group_count as u64);
        self.fanout_groups_max = self.fanout_groups_max.max(summary.group_count);

        for group in &summary.groups {
            self.max_group_size = self.max_group_size.max(group.action_count);
            let count = self.kind_counts.entry(group.key.kind).or_default();
            count.atomic_actions = count
                .atomic_actions
                .saturating_add(group.action_count as u64);
            count.fanout_groups = count.fanout_groups.saturating_add(1);
            count.max_group_size = count.max_group_size.max(group.action_count);
            self.remember_largest_group(ActionExpansionGroupObservation {
                observed_at_state_query: self.states_observed,
                key: group.key.clone(),
                action_count: group.action_count,
            });
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsExpansion {
        CombatSearchV2DiagnosticsExpansion {
            grouping_policy: "typed_fanout_groups_with_no_action_merge",
            behavioral_effect: "diagnostic_only_search_expansion_unchanged",
            states_observed: self.states_observed,
            total_atomic_actions: self.total_atomic_actions,
            total_fanout_groups: self.total_fanout_groups,
            fanout_groups_avg: rounded_ratio(self.total_fanout_groups, self.states_observed),
            fanout_groups_max: self.fanout_groups_max,
            max_group_size: self.max_group_size,
            action_kind_counts: self.action_kind_counts(),
            largest_groups: self.largest_group_samples(),
            notes: vec![
                "fanout groups explain where branching comes from; they are not equivalence classes",
                "largest_groups only includes groups with more than one atomic action",
                "targeted card and potion actions stay atomic and order-sensitive in the search",
                "future compression must prove safety before using these groups for pruning",
            ],
        }
    }

    fn remember_largest_group(&mut self, observation: ActionExpansionGroupObservation) {
        if observation.action_count <= 1 {
            return;
        }
        self.largest_groups.push(observation);
        self.largest_groups.sort_by(|left, right| {
            right
                .action_count
                .cmp(&left.action_count)
                .then_with(|| left.key.kind.cmp(&right.key.kind))
                .then_with(|| left.key.signature.cmp(&right.key.signature))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_groups.truncate(LARGEST_GROUP_SAMPLE_LIMIT);
    }

    fn action_kind_counts(&self) -> Vec<CombatSearchV2DiagnosticsActionKindCount> {
        self.kind_counts
            .iter()
            .map(|(kind, count)| CombatSearchV2DiagnosticsActionKindCount {
                kind: kind.label().to_string(),
                atomic_actions: count.atomic_actions,
                fanout_groups: count.fanout_groups,
                max_group_size: count.max_group_size,
            })
            .collect()
    }

    fn largest_group_samples(&self) -> Vec<CombatSearchV2DiagnosticsActionGroupSample> {
        self.largest_groups
            .iter()
            .map(|group| CombatSearchV2DiagnosticsActionGroupSample {
                observed_at_state_query: group.observed_at_state_query,
                kind: group.key.kind.label().to_string(),
                group_key: group.key.signature.clone(),
                atomic_actions: group.action_count,
            })
            .collect()
    }
}

pub(super) fn summarize_action_expansion(
    engine: &EngineState,
    combat: &CombatState,
    choices: &[CombatActionChoice],
) -> ActionExpansionSummary {
    let mut groups: BTreeMap<ActionExpansionGroupKey, usize> = BTreeMap::new();
    for choice in choices {
        let key = group_key_for_input(engine, combat, &choice.input);
        *groups.entry(key).or_insert(0) += 1;
    }

    let groups = groups
        .into_iter()
        .map(|(key, action_count)| ActionExpansionGroupSummary { key, action_count })
        .collect::<Vec<_>>();

    ActionExpansionSummary {
        action_count: choices.len(),
        group_count: groups.len(),
        groups,
    }
}

fn group_key_for_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> ActionExpansionGroupKey {
    match input {
        ClientInput::PlayCard { card_index, .. } => ActionExpansionGroupKey {
            kind: ActionExpansionKind::PlayCard,
            signature: combat
                .zones
                .hand
                .get(*card_index)
                .map(|card| {
                    format!(
                        "play_card/hand:{card_index}/card:{}+{}/uuid:{}/cost:{}",
                        crate::content::cards::java_id(card.id),
                        card.upgrades,
                        card.uuid,
                        card.cost_for_turn_java()
                    )
                })
                .unwrap_or_else(|| format!("play_card/hand:{card_index}/missing_card")),
        },
        ClientInput::EndTurn => ActionExpansionGroupKey {
            kind: ActionExpansionKind::EndTurn,
            signature: "end_turn".to_string(),
        },
        ClientInput::UsePotion { potion_index, .. } => ActionExpansionGroupKey {
            kind: ActionExpansionKind::UsePotion,
            signature: combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(Option::as_ref)
                .map(|potion| {
                    format!(
                        "use_potion/slot:{potion_index}/potion:{:?}/uuid:{}",
                        potion.id, potion.uuid
                    )
                })
                .unwrap_or_else(|| format!("use_potion/slot:{potion_index}/missing_potion")),
        },
        ClientInput::DiscardPotion(slot) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::DiscardPotion,
            signature: format!("discard_potion/slot:{slot}"),
        },
        ClientInput::SubmitDiscoverChoice(_) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::DiscoverChoice,
            signature: format!("discover_choice/{}", pending_choice_label(engine)),
        },
        ClientInput::SubmitHandSelect(uuids) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::HandSelect,
            signature: format!(
                "hand_select/{}/selected:{}",
                pending_choice_label(engine),
                uuids.len()
            ),
        },
        ClientInput::SubmitGridSelect(uuids) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::GridSelect,
            signature: format!(
                "grid_select/{}/selected:{}",
                pending_choice_label(engine),
                uuids.len()
            ),
        },
        ClientInput::SubmitScryDiscard(indices) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::ScryDiscard,
            signature: format!("scry_discard/selected:{}", indices.len()),
        },
        ClientInput::Cancel => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Cancel,
            signature: format!("cancel/{}", pending_choice_label(engine)),
        },
        ClientInput::Proceed => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Proceed,
            signature: format!("proceed/{}", engine_state_label(engine)),
        },
        _ => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Other,
            signature: format!("{input:?}"),
        },
    }
}

fn pending_choice_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::PendingChoice(choice) => match choice {
            crate::state::core::PendingChoice::HandSelect { .. } => "pending_hand_select",
            crate::state::core::PendingChoice::GridSelect { .. } => "pending_grid_select",
            crate::state::core::PendingChoice::DiscoverySelect(_) => "pending_discovery_select",
            crate::state::core::PendingChoice::CardRewardSelect { .. } => {
                "pending_card_reward_select"
            }
            crate::state::core::PendingChoice::ForeignInfluenceSelect { .. } => {
                "pending_foreign_influence_select"
            }
            crate::state::core::PendingChoice::ChooseOneSelect { .. } => {
                "pending_choose_one_select"
            }
            crate::state::core::PendingChoice::StanceChoice => "pending_stance_choice",
            _ => "pending_other_choice",
        },
        _ => "not_pending_choice",
    }
}

fn engine_state_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::CombatStart(_) => "combat_start",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::TreasureRoom(_) => "treasure_room",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::EventRoom => "event_room",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

impl ActionExpansionGroupKey {
    fn cmp_tuple(&self) -> (ActionExpansionKind, &str) {
        (self.kind, &self.signature)
    }
}

impl Ord for ActionExpansionGroupKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_tuple().cmp(&other.cmp_tuple())
    }
}

impl PartialOrd for ActionExpansionGroupKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ActionExpansionKind {
    fn label(self) -> &'static str {
        match self {
            ActionExpansionKind::PlayCard => "play_card",
            ActionExpansionKind::EndTurn => "end_turn",
            ActionExpansionKind::UsePotion => "use_potion",
            ActionExpansionKind::DiscardPotion => "discard_potion",
            ActionExpansionKind::DiscoverChoice => "discover_choice",
            ActionExpansionKind::HandSelect => "hand_select",
            ActionExpansionKind::GridSelect => "grid_select",
            ActionExpansionKind::ScryDiscard => "scry_discard",
            ActionExpansionKind::Cancel => "cancel",
            ActionExpansionKind::Proceed => "proceed",
            ActionExpansionKind::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::runtime::combat::CombatCard;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn targeted_card_actions_share_a_diagnostic_fanout_group() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 42)];
        let mut first = test_monster(EnemyId::JawWorm);
        first.id = 11;
        let mut second = test_monster(EnemyId::Cultist);
        second.id = 12;
        combat.entities.monsters = vec![first, second];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(11),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(12),
                },
            ),
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
        ];

        let summary = summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &choices);

        assert_eq!(summary.action_count, 3);
        assert_eq!(summary.group_count, 2);
        assert!(summary.groups.iter().any(|group| {
            group.key.kind == ActionExpansionKind::PlayCard && group.action_count == 2
        }));
    }

    #[test]
    fn potion_policy_filtered_actions_are_observed_after_filtering() {
        let mut combat = blank_test_combat();
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
        let legal = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: None,
                },
            ),
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
        ];
        let filtered = filtered_legal_actions(legal, CombatSearchV2PotionPolicy::Never, &combat);

        let summary =
            summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &filtered);

        assert_eq!(summary.action_count, 1);
        assert!(summary.groups.iter().all(|group| {
            group.key.kind != ActionExpansionKind::UsePotion
                && group.key.kind != ActionExpansionKind::DiscardPotion
        }));
    }

    #[test]
    fn expansion_collector_reports_largest_groups_without_changing_behavior() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 42)];
        let mut first = test_monster(EnemyId::JawWorm);
        first.id = 11;
        let mut second = test_monster(EnemyId::Cultist);
        second.id = 12;
        combat.entities.monsters = vec![first, second];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(11),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(12),
                },
            ),
        ];
        let summary = summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &choices);
        let mut collector = ActionExpansionDiagnosticsCollector::default();

        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(
            report.behavioral_effect,
            "diagnostic_only_search_expansion_unchanged"
        );
        assert_eq!(report.total_atomic_actions, 2);
        assert_eq!(report.total_fanout_groups, 1);
        assert_eq!(report.max_group_size, 2);
        assert_eq!(report.largest_groups[0].kind, "play_card");
    }
}
