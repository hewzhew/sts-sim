use super::*;
use crate::content::cards;
use crate::state::core::{PendingChoice, PileType};
use std::collections::BTreeMap;

use super::state_abstraction::{boundary_spec, StateAbstractionBoundaryId};

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
    SingleCardPendingChoiceSelection,
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
    match &choice.input {
        ClientInput::PlayCard { card_index, target } => {
            if !matches!(engine, EngineState::CombatPlayerTurn) {
                return None;
            }
            let card = combat.zones.hand.get(*card_index)?;
            if !cards::is_starter_basic(card.id) {
                return None;
            }
            Some(ActionEquivalenceKey {
                kind: ActionEquivalenceKind::StarterBasicPlayCard,
                signature: starter_basic_card_signature(combat, card, *target),
            })
        }
        ClientInput::SubmitGridSelect(uuids) => {
            pending_single_card_selection_key(engine, combat, uuids)
        }
        ClientInput::SubmitHandSelect(uuids) => {
            pending_single_card_selection_key(engine, combat, uuids)
        }
        _ => None,
    }
}

fn pending_single_card_selection_key(
    engine: &EngineState,
    combat: &CombatState,
    uuids: &[u32],
) -> Option<ActionEquivalenceKey> {
    let [uuid] = uuids else {
        return None;
    };
    let EngineState::PendingChoice(choice) = engine else {
        return None;
    };

    let (scope, cards) = match choice {
        PendingChoice::GridSelect {
            source_pile,
            reason,
            candidate_uuids,
            ..
        } if candidate_uuids.contains(uuid) => (
            format!("grid_select/source:{source_pile:?}/reason:{reason:?}"),
            pile_cards(combat, *source_pile),
        ),
        PendingChoice::HandSelect {
            reason,
            candidate_uuids,
            ..
        } if candidate_uuids.contains(uuid) => (
            format!("hand_select/reason:{reason:?}"),
            combat.zones.hand.as_slice(),
        ),
        _ => return None,
    };
    let card = cards.iter().find(|card| card.uuid == *uuid)?;
    Some(ActionEquivalenceKey {
        kind: ActionEquivalenceKind::SingleCardPendingChoiceSelection,
        signature: format!("{scope}/selected_card:{}", card_runtime_signature(card)),
    })
}

fn pile_cards(combat: &CombatState, pile: PileType) -> &[crate::runtime::combat::CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &[],
    }
}

fn starter_basic_card_signature(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> String {
    format!(
        "play_card/starter_basic/{}/target:{}",
        card_runtime_signature(card),
        crate::sim::combat_action::target_label(combat, target),
    )
}

fn card_runtime_signature(card: &crate::runtime::combat::CombatCard) -> String {
    format!(
        "card:{}+{}/misc:{}/damage_override:{:?}/block_override:{:?}/cost_modifier:{}/cost_for_turn:{:?}/base_damage_mut:{}/base_block_mut:{}/base_magic_num_mut:{}/multi_damage:{:?}/exhaust_override:{:?}/retain_override:{:?}/free_to_play_once:{}/energy_on_use:{}",
        cards::java_id(card.id),
        card.upgrades,
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
        card.energy_on_use
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
            equivalence_policy:
                "conservative_duplicate_play_card_and_single_card_pending_selection_by_runtime_signature",
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
                "combat player-turn duplicate play-card compression remains limited to starter basic cards in v1",
                "single-card pending grid/hand selections can merge runtime-identical source cards",
                "multi-card pending selections stay atomic because selection order can affect resolution",
                "card runtime fields and target or selection scope must match; card uuid is intentionally ignored",
                "each equivalence kind is attached to an explicit StateAbstractionBoundarySpec",
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
                    boundary_id: kind.boundary_id(),
                    soundness: boundary_spec(kind.boundary_id()).soundness,
                    allowed_consumers: boundary_spec(kind.boundary_id()).allowed_consumers,
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
                boundary_id: sample.key.kind.boundary_id(),
                soundness: boundary_spec(sample.key.kind.boundary_id()).soundness,
                allowed_consumers: boundary_spec(sample.key.kind.boundary_id()).allowed_consumers,
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
            ActionEquivalenceKind::SingleCardPendingChoiceSelection => {
                "single_card_pending_choice_selection"
            }
        }
    }

    fn boundary_id(self) -> StateAbstractionBoundaryId {
        match self {
            ActionEquivalenceKind::StarterBasicPlayCard => {
                StateAbstractionBoundaryId::StarterBasicDuplicatePlayCardByTarget
            }
            ActionEquivalenceKind::SingleCardPendingChoiceSelection => {
                StateAbstractionBoundaryId::PendingChoiceIdenticalRuntimeCard
            }
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
    fn compresses_single_card_pending_grid_selection_for_runtime_identical_cards() {
        let mut combat = blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Slimed, 10),
            CombatCard::new(CardId::Slimed, 11),
            CombatCard::new(CardId::Strike, 12),
        ];
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![10, 11, 12],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::core::GridSelectReason::MoveToDrawPile,
        });
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![10])),
            CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![11])),
            CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![12])),
        ];

        let result = compress_equivalent_actions(&engine, &combat, choices);

        assert_eq!(result.choices.len(), 2);
        assert_eq!(result.summary.actions_removed(), 1);
        assert_eq!(
            result.summary.groups[0].key.kind,
            ActionEquivalenceKind::SingleCardPendingChoiceSelection
        );
    }

    #[test]
    fn keeps_pending_grid_multi_select_atomic() {
        let mut combat = blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Slimed, 10),
            CombatCard::new(CardId::Slimed, 11),
        ];
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![10, 11],
            min_cards: 2,
            max_cards: 2,
            can_cancel: false,
            reason: crate::state::core::GridSelectReason::DrawPileToHand,
        });
        let choices = vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::SubmitGridSelect(vec![10, 11]),
        )];

        let result = compress_equivalent_actions(&engine, &combat, choices);

        assert_eq!(result.choices.len(), 1);
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
