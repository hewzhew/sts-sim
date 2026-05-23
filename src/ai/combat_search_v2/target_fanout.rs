use super::*;
use crate::content::cards::{self, CardTarget};
use crate::content::potions::PotionId;
use std::collections::BTreeMap;

const LARGEST_TARGET_FANOUT_SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct TargetFanoutSummary {
    targeted_actions: usize,
    groups: Vec<TargetFanoutGroupSummary>,
}

#[derive(Default)]
pub(super) struct TargetFanoutDiagnosticsCollector {
    states_observed: u64,
    targeted_actions_total: u64,
    target_fanout_groups_total: u64,
    multi_target_fanout_groups: u64,
    total_targets_in_groups: u64,
    max_targets_per_group: usize,
    lethal_target_groups: u64,
    unique_lethal_target_groups: u64,
    uniform_damage_groups: u64,
    max_target_hp_span: i32,
    kind_counts: BTreeMap<TargetFanoutKind, MutableTargetFanoutKindCount>,
    largest_target_fanouts: Vec<TargetFanoutGroupObservation>,
}

#[derive(Clone, Debug)]
struct TargetFanoutGroupSummary {
    kind: TargetFanoutKind,
    source_key: String,
    target_count: usize,
    lethal_targets: usize,
    min_target_hp_with_block: i32,
    max_target_hp_with_block: i32,
    min_damage_hint: i32,
    max_damage_hint: i32,
    first_action_key: String,
}

#[derive(Clone, Debug)]
struct TargetFanoutCandidate {
    kind: TargetFanoutKind,
    source_key: String,
    target_hp_with_block: i32,
    damage_hint: i32,
    lethal: bool,
    action_key: String,
}

#[derive(Clone, Debug)]
struct TargetFanoutGroupObservation {
    observed_at_state_query: u64,
    group: TargetFanoutGroupSummary,
}

#[derive(Clone, Debug, Default)]
struct MutableTargetFanoutKindCount {
    groups: u64,
    actions: u64,
    multi_target_groups: u64,
    lethal_target_groups: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TargetFanoutKind {
    PlayCard,
    UsePotion,
}

pub(super) fn summarize_target_fanout(
    combat: &CombatState,
    choices: &[CombatActionChoice],
) -> TargetFanoutSummary {
    let mut grouped: BTreeMap<(TargetFanoutKind, String), Vec<TargetFanoutCandidate>> =
        BTreeMap::new();

    for choice in choices {
        if let Some(candidate) = target_fanout_candidate(combat, choice) {
            grouped
                .entry((candidate.kind, candidate.source_key.clone()))
                .or_default()
                .push(candidate);
        }
    }

    let targeted_actions = grouped.values().map(Vec::len).sum();
    let groups = grouped
        .into_values()
        .filter_map(summarize_group)
        .collect::<Vec<_>>();

    TargetFanoutSummary {
        targeted_actions,
        groups,
    }
}

fn target_fanout_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
) -> Option<TargetFanoutCandidate> {
    match &choice.input {
        ClientInput::PlayCard {
            card_index,
            target: Some(target),
        } => card_target_candidate(combat, choice, *card_index, *target),
        ClientInput::UsePotion {
            potion_index,
            target: Some(target),
        } => potion_target_candidate(combat, choice, *potion_index, *target),
        _ => None,
    }
}

fn card_target_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
    card_index: usize,
    target: usize,
) -> Option<TargetFanoutCandidate> {
    let card = combat.zones.hand.get(card_index)?;
    let target_hp_with_block = monster_hp_with_block(combat, target)?;
    let target_kind = cards::effective_target(card);
    if !matches!(target_kind, CardTarget::Enemy | CardTarget::SelfAndEnemy) {
        return None;
    }
    let evaluated = cards::evaluate_card_for_play(card, combat, Some(target));
    let damage_hint = evaluated.base_damage_mut.max(0);
    Some(TargetFanoutCandidate {
        kind: TargetFanoutKind::PlayCard,
        source_key: format!(
            "play_card/hand:{card_index}/card:{}+{}/uuid:{}/cost:{}",
            cards::java_id(card.id),
            card.upgrades,
            card.uuid,
            card.cost_for_turn_java()
        ),
        target_hp_with_block,
        damage_hint,
        lethal: damage_hint >= target_hp_with_block && damage_hint > 0,
        action_key: choice.action_key.clone(),
    })
}

fn potion_target_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
    potion_index: usize,
    target: usize,
) -> Option<TargetFanoutCandidate> {
    let potion = combat.entities.potions.get(potion_index)?.as_ref()?;
    let target_hp_with_block = monster_hp_with_block(combat, target)?;
    let damage_hint = potion_single_target_damage_hint(combat, potion.id);
    Some(TargetFanoutCandidate {
        kind: TargetFanoutKind::UsePotion,
        source_key: format!(
            "use_potion/slot:{potion_index}/potion:{:?}/uuid:{}",
            potion.id, potion.uuid
        ),
        target_hp_with_block,
        damage_hint,
        lethal: damage_hint >= target_hp_with_block && damage_hint > 0,
        action_key: choice.action_key.clone(),
    })
}

fn potion_single_target_damage_hint(combat: &CombatState, id: PotionId) -> i32 {
    match id {
        PotionId::FirePotion => potion_potency(combat, id),
        _ => 0,
    }
}

fn potion_potency(combat: &CombatState, id: PotionId) -> i32 {
    let mut potency = crate::content::potions::get_potion_definition(id).base_potency;
    if combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::SacredBark)
    {
        potency *= 2;
    }
    potency
}

fn summarize_group(candidates: Vec<TargetFanoutCandidate>) -> Option<TargetFanoutGroupSummary> {
    let first = candidates.first()?;
    let mut min_target_hp_with_block = i32::MAX;
    let mut max_target_hp_with_block = i32::MIN;
    let mut min_damage_hint = i32::MAX;
    let mut max_damage_hint = i32::MIN;
    let mut lethal_targets = 0usize;

    for candidate in &candidates {
        min_target_hp_with_block = min_target_hp_with_block.min(candidate.target_hp_with_block);
        max_target_hp_with_block = max_target_hp_with_block.max(candidate.target_hp_with_block);
        min_damage_hint = min_damage_hint.min(candidate.damage_hint);
        max_damage_hint = max_damage_hint.max(candidate.damage_hint);
        if candidate.lethal {
            lethal_targets = lethal_targets.saturating_add(1);
        }
    }

    Some(TargetFanoutGroupSummary {
        kind: first.kind,
        source_key: first.source_key.clone(),
        target_count: candidates.len(),
        lethal_targets,
        min_target_hp_with_block,
        max_target_hp_with_block,
        min_damage_hint,
        max_damage_hint,
        first_action_key: first.action_key.clone(),
    })
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

impl TargetFanoutDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &TargetFanoutSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.targeted_actions_total = self
            .targeted_actions_total
            .saturating_add(summary.targeted_actions as u64);
        self.target_fanout_groups_total = self
            .target_fanout_groups_total
            .saturating_add(summary.groups.len() as u64);

        for group in &summary.groups {
            let target_count = group.target_count as u64;
            self.total_targets_in_groups =
                self.total_targets_in_groups.saturating_add(target_count);
            self.max_targets_per_group = self.max_targets_per_group.max(group.target_count);
            self.max_target_hp_span = self.max_target_hp_span.max(group.target_hp_span());

            let kind_count = self.kind_counts.entry(group.kind).or_default();
            kind_count.groups = kind_count.groups.saturating_add(1);
            kind_count.actions = kind_count.actions.saturating_add(target_count);

            if group.target_count > 1 {
                self.multi_target_fanout_groups = self.multi_target_fanout_groups.saturating_add(1);
                kind_count.multi_target_groups = kind_count.multi_target_groups.saturating_add(1);
            }
            if group.lethal_targets > 0 {
                self.lethal_target_groups = self.lethal_target_groups.saturating_add(1);
                kind_count.lethal_target_groups = kind_count.lethal_target_groups.saturating_add(1);
            }
            if group.lethal_targets == 1 {
                self.unique_lethal_target_groups =
                    self.unique_lethal_target_groups.saturating_add(1);
            }
            if group.min_damage_hint == group.max_damage_hint {
                self.uniform_damage_groups = self.uniform_damage_groups.saturating_add(1);
            }
            self.remember_largest_target_fanout(group);
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsTargetFanout {
        CombatSearchV2DiagnosticsTargetFanout {
            grouping_policy: "targeted_card_and_potion_actions_grouped_by_source",
            behavioral_effect: "diagnostic_only_no_target_prune_no_merge",
            states_observed: self.states_observed,
            targeted_actions_total: self.targeted_actions_total,
            target_fanout_groups_total: self.target_fanout_groups_total,
            multi_target_fanout_groups: self.multi_target_fanout_groups,
            avg_targets_per_group: rounded_ratio(
                self.total_targets_in_groups,
                self.target_fanout_groups_total,
            ),
            max_targets_per_group: self.max_targets_per_group,
            lethal_target_groups: self.lethal_target_groups,
            unique_lethal_target_groups: self.unique_lethal_target_groups,
            uniform_damage_groups: self.uniform_damage_groups,
            max_target_hp_span: self.max_target_hp_span,
            group_kind_counts: self.group_kind_counts(),
            largest_target_fanouts: self.largest_target_fanout_samples(),
            notes: vec![
                "target fanout groups are not equivalence classes",
                "damage hints are visible ordering diagnostics, not simulator damage proofs",
                "non-damage targeted potions are counted with zero damage hint",
                "future target pruning must prove monster-specific safety before removing branches",
            ],
        }
    }

    fn remember_largest_target_fanout(&mut self, group: &TargetFanoutGroupSummary) {
        if group.target_count <= 1 {
            return;
        }
        self.largest_target_fanouts
            .push(TargetFanoutGroupObservation {
                observed_at_state_query: self.states_observed,
                group: group.clone(),
            });
        self.largest_target_fanouts.sort_by(|left, right| {
            right
                .group
                .target_count
                .cmp(&left.group.target_count)
                .then_with(|| right.group.lethal_targets.cmp(&left.group.lethal_targets))
                .then_with(|| {
                    right
                        .group
                        .target_hp_span()
                        .cmp(&left.group.target_hp_span())
                })
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_target_fanouts
            .truncate(LARGEST_TARGET_FANOUT_SAMPLE_LIMIT);
    }

    fn group_kind_counts(&self) -> Vec<CombatSearchV2DiagnosticsTargetFanoutKindCount> {
        self.kind_counts
            .iter()
            .map(
                |(kind, count)| CombatSearchV2DiagnosticsTargetFanoutKindCount {
                    kind: kind.label().to_string(),
                    groups: count.groups,
                    actions: count.actions,
                    multi_target_groups: count.multi_target_groups,
                    lethal_target_groups: count.lethal_target_groups,
                },
            )
            .collect()
    }

    fn largest_target_fanout_samples(&self) -> Vec<CombatSearchV2DiagnosticsTargetFanoutSample> {
        self.largest_target_fanouts
            .iter()
            .map(|observation| {
                let group = &observation.group;
                CombatSearchV2DiagnosticsTargetFanoutSample {
                    observed_at_state_query: observation.observed_at_state_query,
                    kind: group.kind.label().to_string(),
                    source_key: group.source_key.clone(),
                    target_count: group.target_count,
                    lethal_targets: group.lethal_targets,
                    min_target_hp_with_block: group.min_target_hp_with_block,
                    max_target_hp_with_block: group.max_target_hp_with_block,
                    target_hp_span: group.target_hp_span(),
                    min_damage_hint: group.min_damage_hint,
                    max_damage_hint: group.max_damage_hint,
                    first_action_key: group.first_action_key.clone(),
                }
            })
            .collect()
    }
}

impl TargetFanoutGroupSummary {
    fn target_hp_span(&self) -> i32 {
        self.max_target_hp_with_block - self.min_target_hp_with_block
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

impl TargetFanoutKind {
    fn label(self) -> &'static str {
        match self {
            TargetFanoutKind::PlayCard => "play_card",
            TargetFanoutKind::UsePotion => "use_potion",
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
    fn groups_same_card_across_multiple_targets() {
        let mut combat = blank_test_combat();
        let mut low_hp = test_monster(EnemyId::LouseNormal);
        low_hp.current_hp = 6;
        low_hp.max_hp = 6;
        low_hp.id = 1;
        let mut high_hp = test_monster(EnemyId::JawWorm);
        high_hp.current_hp = 30;
        high_hp.max_hp = 30;
        high_hp.id = 2;
        combat.entities.monsters = vec![low_hp, high_hp];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
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
                    card_index: 0,
                    target: Some(2),
                },
            ),
        ];

        let summary = summarize_target_fanout(&combat, &choices);

        assert_eq!(summary.targeted_actions, 2);
        assert_eq!(summary.groups.len(), 1);
        assert_eq!(summary.groups[0].target_count, 2);
        assert_eq!(summary.groups[0].lethal_targets, 1);
        assert_eq!(summary.groups[0].target_hp_span(), 24);
    }

    #[test]
    fn collector_reports_largest_fanouts_without_pruning_claim() {
        let mut combat = blank_test_combat();
        let mut first = test_monster(EnemyId::LouseNormal);
        first.current_hp = 6;
        first.max_hp = 6;
        first.id = 1;
        let mut second = test_monster(EnemyId::JawWorm);
        second.current_hp = 30;
        second.max_hp = 30;
        second.id = 2;
        combat.entities.monsters = vec![first, second];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let summary = summarize_target_fanout(
            &combat,
            &[
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
                        card_index: 0,
                        target: Some(2),
                    },
                ),
            ],
        );
        let mut collector = TargetFanoutDiagnosticsCollector::default();

        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(
            report.behavioral_effect,
            "diagnostic_only_no_target_prune_no_merge"
        );
        assert_eq!(report.targeted_actions_total, 2);
        assert_eq!(report.multi_target_fanout_groups, 1);
        assert_eq!(report.lethal_target_groups, 1);
        assert_eq!(report.unique_lethal_target_groups, 1);
        assert_eq!(report.largest_target_fanouts.len(), 1);
    }

    #[test]
    fn fire_potion_target_fanout_reports_damage_hint() {
        let mut combat = blank_test_combat();
        let mut first = test_monster(EnemyId::LouseNormal);
        first.current_hp = 20;
        first.max_hp = 20;
        first.id = 1;
        let mut second = test_monster(EnemyId::JawWorm);
        second.current_hp = 30;
        second.max_hp = 30;
        second.id = 2;
        combat.entities.monsters = vec![first, second];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 77))];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: Some(2),
                },
            ),
        ];

        let summary = summarize_target_fanout(&combat, &choices);

        assert_eq!(summary.groups.len(), 1);
        assert_eq!(summary.groups[0].min_damage_hint, 20);
        assert_eq!(summary.groups[0].max_damage_hint, 20);
        assert_eq!(summary.groups[0].lethal_targets, 1);
    }
}
