use super::*;
use crate::content::cards;
use std::collections::BTreeMap;

const LARGEST_EQUIVALENCE_GROUP_SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct ActionEquivalenceResult {
    pub(super) choices: Vec<IndexedActionChoice>,
    pub(super) summary: ActionEquivalenceSummary,
}

#[derive(Clone, Debug)]
pub(super) struct ActionEquivalenceSummary {
    atomic_actions_in: usize,
    representative_actions_out: usize,
    groups: Vec<ActionEquivalenceGroupSummary>,
}

#[derive(Clone, Debug)]
struct ActionEquivalenceGroupSummary {
    key: ActionEquivalenceKey,
    representative_original_action_id: usize,
    removed_original_action_ids: Vec<usize>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ActionEquivalenceKey {
    kind: ActionEquivalenceKind,
    signature: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActionEquivalenceKind {
    StarterBasicPlayCard,
}

#[derive(Clone, Debug)]
struct PendingEquivalenceGroup {
    representative_original_action_id: usize,
    removed_original_action_ids: Vec<usize>,
}

#[derive(Default)]
pub(super) struct ActionEquivalenceDiagnosticsCollector {
    states_observed: u64,
    states_compressed: u64,
    atomic_actions_in: u64,
    representative_actions_out: u64,
    actions_removed: u64,
    max_group_size: usize,
    kind_counts: BTreeMap<ActionEquivalenceKind, MutableEquivalenceKindCount>,
    largest_groups: Vec<ActionEquivalenceObservation>,
}

#[derive(Clone, Debug, Default)]
struct MutableEquivalenceKindCount {
    groups: u64,
    actions_in: u64,
    actions_removed: u64,
    max_group_size: usize,
}

#[derive(Clone, Debug)]
struct ActionEquivalenceObservation {
    observed_at_state_query: u64,
    key: ActionEquivalenceKey,
    representative_original_action_id: usize,
    removed_original_action_ids: Vec<usize>,
}

pub(super) fn compress_equivalent_actions(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<CombatActionChoice>,
) -> ActionEquivalenceResult {
    let atomic_actions_in = choices.len();
    let mut representatives = Vec::with_capacity(choices.len());
    let mut seen: BTreeMap<ActionEquivalenceKey, usize> = BTreeMap::new();
    let mut groups: Vec<(ActionEquivalenceKey, PendingEquivalenceGroup)> = Vec::new();

    for (original_action_id, choice) in choices.into_iter().enumerate() {
        let Some(key) = equivalence_key_for_choice(engine, combat, &choice) else {
            representatives.push(IndexedActionChoice {
                original_action_id,
                choice,
            });
            continue;
        };

        if let Some(group_index) = seen.get(&key).copied() {
            groups[group_index]
                .1
                .removed_original_action_ids
                .push(original_action_id);
        } else {
            representatives.push(IndexedActionChoice {
                original_action_id,
                choice,
            });
            seen.insert(key.clone(), groups.len());
            groups.push((
                key,
                PendingEquivalenceGroup {
                    representative_original_action_id: original_action_id,
                    removed_original_action_ids: Vec::new(),
                },
            ));
        }
    }

    let groups = groups
        .into_iter()
        .filter_map(|(key, group)| {
            if group.removed_original_action_ids.is_empty() {
                None
            } else {
                Some(ActionEquivalenceGroupSummary {
                    key,
                    representative_original_action_id: group.representative_original_action_id,
                    removed_original_action_ids: group.removed_original_action_ids,
                })
            }
        })
        .collect::<Vec<_>>();

    ActionEquivalenceResult {
        summary: ActionEquivalenceSummary {
            atomic_actions_in,
            representative_actions_out: representatives.len(),
            groups,
        },
        choices: representatives,
    }
}

fn equivalence_key_for_choice(
    engine: &EngineState,
    combat: &CombatState,
    choice: &CombatActionChoice,
) -> Option<ActionEquivalenceKey> {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return None;
    }

    match &choice.input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat.zones.hand.get(*card_index)?;
            if !cards::is_starter_basic(card.id) {
                return None;
            }
            Some(ActionEquivalenceKey {
                kind: ActionEquivalenceKind::StarterBasicPlayCard,
                signature: starter_basic_card_signature(combat, card, *target),
            })
        }
        _ => None,
    }
}

fn starter_basic_card_signature(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> String {
    format!(
        "play_card/starter_basic/card:{}+{}/target:{}/misc:{}/damage_override:{:?}/block_override:{:?}/cost_modifier:{}/cost_for_turn:{:?}/base_damage_mut:{}/base_block_mut:{}/base_magic_num_mut:{}/multi_damage:{:?}/exhaust_override:{:?}/retain_override:{:?}/free_to_play_once:{}/energy_on_use:{}",
        cards::java_id(card.id),
        card.upgrades,
        crate::sim::combat_action::target_label(combat, target),
        card.misc_value,
        card.base_damage_override,
        card.base_block_override,
        card.cost_modifier,
        card.cost_for_turn,
        card.base_damage_mut,
        card.base_block_mut,
        card.base_magic_num_mut,
        card.multi_damage,
        card.exhaust_override,
        card.retain_override,
        card.free_to_play_once,
        card.energy_on_use,
    )
}

impl ActionEquivalenceDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &ActionEquivalenceSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.atomic_actions_in = self
            .atomic_actions_in
            .saturating_add(summary.atomic_actions_in as u64);
        self.representative_actions_out = self
            .representative_actions_out
            .saturating_add(summary.representative_actions_out as u64);
        let actions_removed = summary.actions_removed();
        self.actions_removed = self.actions_removed.saturating_add(actions_removed as u64);
        if actions_removed > 0 {
            self.states_compressed = self.states_compressed.saturating_add(1);
        }

        for group in &summary.groups {
            let group_size = group.group_size();
            self.max_group_size = self.max_group_size.max(group_size);
            let count = self.kind_counts.entry(group.key.kind).or_default();
            count.groups = count.groups.saturating_add(1);
            count.actions_in = count.actions_in.saturating_add(group_size as u64);
            count.actions_removed = count
                .actions_removed
                .saturating_add(group.removed_original_action_ids.len() as u64);
            count.max_group_size = count.max_group_size.max(group_size);
            self.remember_largest_group(group);
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsEquivalence {
        CombatSearchV2DiagnosticsEquivalence {
            equivalence_policy: "conservative_starter_basic_duplicate_play_card_by_target",
            behavioral_effect:
                "safe_representative_child_generation_for_proven_duplicate_actions_only",
            states_observed: self.states_observed,
            states_compressed: self.states_compressed,
            atomic_actions_in: self.atomic_actions_in,
            representative_actions_out: self.representative_actions_out,
            actions_removed: self.actions_removed,
            removed_action_ratio: rounded_ratio(self.actions_removed, self.atomic_actions_in),
            max_group_size: self.max_group_size,
            group_kind_counts: self.group_kind_counts(),
            largest_groups: self.largest_group_samples(),
            notes: vec![
                "only combat player-turn play-card actions are eligible in v1",
                "only starter basic cards are eligible in v1",
                "card runtime fields and target must match; card uuid is intentionally ignored",
                "representative action traces keep the original legal action id",
                "non-eligible actions stay atomic and order-sensitive",
            ],
        }
    }

    fn remember_largest_group(&mut self, group: &ActionEquivalenceGroupSummary) {
        self.largest_groups.push(ActionEquivalenceObservation {
            observed_at_state_query: self.states_observed,
            key: group.key.clone(),
            representative_original_action_id: group.representative_original_action_id,
            removed_original_action_ids: group.removed_original_action_ids.clone(),
        });
        self.largest_groups.sort_by(|left, right| {
            right
                .group_size()
                .cmp(&left.group_size())
                .then_with(|| left.key.kind.cmp(&right.key.kind))
                .then_with(|| left.key.signature.cmp(&right.key.signature))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_groups
            .truncate(LARGEST_EQUIVALENCE_GROUP_SAMPLE_LIMIT);
    }

    fn group_kind_counts(&self) -> Vec<CombatSearchV2DiagnosticsEquivalenceKindCount> {
        self.kind_counts
            .iter()
            .map(
                |(kind, count)| CombatSearchV2DiagnosticsEquivalenceKindCount {
                    kind: kind.label().to_string(),
                    groups: count.groups,
                    actions_in: count.actions_in,
                    actions_removed: count.actions_removed,
                    max_group_size: count.max_group_size,
                },
            )
            .collect()
    }

    fn largest_group_samples(&self) -> Vec<CombatSearchV2DiagnosticsEquivalenceGroupSample> {
        self.largest_groups
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsEquivalenceGroupSample {
                observed_at_state_query: sample.observed_at_state_query,
                kind: sample.key.kind.label().to_string(),
                equivalence_key: sample.key.signature.clone(),
                representative_original_action_id: sample.representative_original_action_id,
                removed_original_action_ids: sample.removed_original_action_ids.clone(),
                group_size: sample.group_size(),
            })
            .collect()
    }
}

impl ActionEquivalenceSummary {
    fn actions_removed(&self) -> usize {
        self.atomic_actions_in
            .saturating_sub(self.representative_actions_out)
    }
}

impl ActionEquivalenceGroupSummary {
    fn group_size(&self) -> usize {
        self.removed_original_action_ids.len().saturating_add(1)
    }
}

impl ActionEquivalenceObservation {
    fn group_size(&self) -> usize {
        self.removed_original_action_ids.len().saturating_add(1)
    }
}

impl ActionEquivalenceKind {
    fn label(self) -> &'static str {
        match self {
            ActionEquivalenceKind::StarterBasicPlayCard => "starter_basic_play_card",
        }
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn compresses_duplicate_starter_basic_cards_to_same_target() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 1;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(result.choices.len(), 1);
        assert_eq!(result.choices[0].original_action_id, 0);
        assert_eq!(result.summary.actions_removed(), 1);
        assert_eq!(
            result.summary.groups[0].removed_original_action_ids,
            vec![1]
        );
    }

    #[test]
    fn keeps_duplicate_starter_basic_cards_split_by_target() {
        let mut combat = blank_test_combat();
        let mut first = test_monster(EnemyId::LouseNormal);
        first.id = 1;
        let mut second = test_monster(EnemyId::LouseNormal);
        second.id = 2;
        combat.entities.monsters = vec![first, second];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(2),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(2),
                },
            ),
        ];

        let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(result.choices.len(), 2);
        assert_eq!(result.summary.actions_removed(), 2);
        assert_eq!(result.summary.groups.len(), 2);
    }

    #[test]
    fn does_not_compress_non_starter_basic_duplicates() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 1;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::TwinStrike, 10),
            CombatCard::new(CardId::TwinStrike, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(result.choices.len(), 2);
        assert_eq!(result.summary.actions_removed(), 0);
    }

    #[test]
    fn does_not_compress_cards_with_different_runtime_state() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 1;
        combat.entities.monsters = vec![monster];
        let mut free = CombatCard::new(CardId::Strike, 10);
        free.free_to_play_once = true;
        combat.zones.hand = vec![free, CombatCard::new(CardId::Strike, 11)];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(result.choices.len(), 2);
        assert_eq!(result.summary.actions_removed(), 0);
    }

    #[test]
    fn collector_reports_removed_duplicate_actions() {
        let mut collector = ActionEquivalenceDiagnosticsCollector::default();
        let summary = ActionEquivalenceSummary {
            atomic_actions_in: 3,
            representative_actions_out: 2,
            groups: vec![ActionEquivalenceGroupSummary {
                key: ActionEquivalenceKey {
                    kind: ActionEquivalenceKind::StarterBasicPlayCard,
                    signature: "play_card/starter_basic/card:Strike_R+0/target:monster_slot:0"
                        .to_string(),
                },
                representative_original_action_id: 0,
                removed_original_action_ids: vec![1],
            }],
        };

        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(
            report.behavioral_effect,
            "safe_representative_child_generation_for_proven_duplicate_actions_only"
        );
        assert_eq!(report.states_observed, 1);
        assert_eq!(report.states_compressed, 1);
        assert_eq!(report.actions_removed, 1);
        assert_eq!(report.max_group_size, 2);
        assert_eq!(report.largest_groups.len(), 1);
    }
}
