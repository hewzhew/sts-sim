use super::*;
use std::collections::{BTreeMap, BTreeSet};

const SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug, Default)]
pub(super) struct CardIdentitySummary {
    active_cards: usize,
    action_payload_cards: usize,
    action_payload_placeholder_cards: usize,
    duplicate_groups: Vec<CardIdentityGroupSummary>,
    conflict_groups: Vec<CardIdentityGroupSummary>,
}

#[derive(Clone, Debug)]
struct CardIdentityOccurrence {
    location: String,
    card_label: String,
    card_id_label: String,
}

#[derive(Clone, Debug)]
struct CardIdentityGroupSummary {
    uuid: u32,
    occurrence_count: usize,
    distinct_card_labels: Vec<String>,
    locations: Vec<String>,
}

#[derive(Default)]
pub(super) struct CardIdentityDiagnosticsCollector {
    states_observed: u64,
    active_cards_observed: u64,
    action_payload_cards_observed: u64,
    action_payload_placeholder_cards: u64,
    states_with_duplicate_active_uuid: u64,
    duplicate_active_uuid_observations: u64,
    states_with_uuid_card_id_conflict: u64,
    uuid_card_id_conflict_observations: u64,
    max_duplicate_group_size: usize,
    largest_duplicate_groups: Vec<CardIdentityObservedGroup>,
    largest_conflict_groups: Vec<CardIdentityObservedGroup>,
}

#[derive(Clone, Debug)]
struct CardIdentityObservedGroup {
    observed_at_state_query: u64,
    group: CardIdentityGroupSummary,
}

pub(super) fn summarize_card_identity(combat: &CombatState) -> CardIdentitySummary {
    let mut active_by_uuid: BTreeMap<u32, Vec<CardIdentityOccurrence>> = BTreeMap::new();
    collect_active_cards("hand", &combat.zones.hand, &mut active_by_uuid);
    collect_active_cards("draw", &combat.zones.draw_pile, &mut active_by_uuid);
    collect_active_cards("discard", &combat.zones.discard_pile, &mut active_by_uuid);
    collect_active_cards("exhaust", &combat.zones.exhaust_pile, &mut active_by_uuid);
    collect_active_cards("limbo", &combat.zones.limbo, &mut active_by_uuid);
    for (idx, queued) in combat.zones.queued_cards.iter().enumerate() {
        push_card_occurrence(&mut active_by_uuid, format!("queued[{idx}]"), &queued.card);
    }

    let mut duplicate_groups = Vec::new();
    let mut conflict_groups = Vec::new();
    for (uuid, occurrences) in active_by_uuid {
        if occurrences.len() > 1 {
            duplicate_groups.push(group_summary(uuid, &occurrences));
        }
        let distinct_ids = occurrences
            .iter()
            .map(|occurrence| occurrence.card_id_label.as_str())
            .collect::<BTreeSet<_>>();
        if distinct_ids.len() > 1 {
            conflict_groups.push(group_summary(uuid, &occurrences));
        }
    }

    let action_payload_cards = action_payload_cards(&combat.engine.action_queue);
    let action_payload_placeholder_cards = action_payload_cards
        .iter()
        .filter(|card| card.uuid == 0)
        .count();

    CardIdentitySummary {
        active_cards: active_card_count(combat),
        action_payload_cards: action_payload_cards.len(),
        action_payload_placeholder_cards,
        duplicate_groups,
        conflict_groups,
    }
}

fn collect_active_cards(
    zone: &str,
    cards: &[crate::runtime::combat::CombatCard],
    by_uuid: &mut BTreeMap<u32, Vec<CardIdentityOccurrence>>,
) {
    for (idx, card) in cards.iter().enumerate() {
        push_card_occurrence(by_uuid, format!("{zone}[{idx}]"), card);
    }
}

fn push_card_occurrence(
    by_uuid: &mut BTreeMap<u32, Vec<CardIdentityOccurrence>>,
    location: String,
    card: &crate::runtime::combat::CombatCard,
) {
    by_uuid
        .entry(card.uuid)
        .or_default()
        .push(CardIdentityOccurrence {
            location: format!("{location}:{}", card_label(card)),
            card_label: card_label(card),
            card_id_label: crate::content::cards::java_id(card.id).to_string(),
        });
}

fn active_card_count(combat: &CombatState) -> usize {
    combat.zones.hand.len()
        + combat.zones.draw_pile.len()
        + combat.zones.discard_pile.len()
        + combat.zones.exhaust_pile.len()
        + combat.zones.limbo.len()
        + combat.zones.queued_cards.len()
}

fn action_payload_cards(
    queue: &std::collections::VecDeque<crate::runtime::action::Action>,
) -> Vec<&crate::runtime::combat::CombatCard> {
    let mut cards = Vec::new();
    for action in queue {
        collect_action_payload_cards(action, &mut cards);
    }
    cards
}

fn collect_action_payload_cards<'a>(
    action: &'a crate::runtime::action::Action,
    cards: &mut Vec<&'a crate::runtime::combat::CombatCard>,
) {
    use crate::runtime::action::Action;
    use crate::runtime::combat::PowerPayload;

    match action {
        Action::AttackDamageRandomEnemyCard { card }
        | Action::PlayCardDirect { card, .. }
        | Action::MakeCopyInHand { original: card, .. }
        | Action::MakeConstructedCopyInHand { original: card, .. }
        | Action::MakeCopyInDrawPile { original: card, .. }
        | Action::MakeCopyInDiscard { original: card, .. }
        | Action::UseCardAfterUseHooks { card } => cards.push(card),
        Action::EnqueueCardPlay { item, .. } => cards.push(&item.card),
        Action::ApplyPowerWithPayload { payload, .. } => {
            if let PowerPayload::Card(card) = payload {
                cards.push(card);
            }
        }
        _ => {}
    }
}

fn group_summary(uuid: u32, occurrences: &[CardIdentityOccurrence]) -> CardIdentityGroupSummary {
    CardIdentityGroupSummary {
        uuid,
        occurrence_count: occurrences.len(),
        distinct_card_labels: occurrences
            .iter()
            .map(|occurrence| occurrence.card_label.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        locations: occurrences
            .iter()
            .map(|occurrence| occurrence.location.clone())
            .collect(),
    }
}

fn card_label(card: &crate::runtime::combat::CombatCard) -> String {
    format!(
        "{}+{}#{} cost:{} misc:{} free:{}",
        crate::content::cards::java_id(card.id),
        card.upgrades,
        card.uuid,
        card.cost_for_turn_java(),
        card.misc_value,
        card.free_to_play_once
    )
}

impl CardIdentityDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &CardIdentitySummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.active_cards_observed = self
            .active_cards_observed
            .saturating_add(summary.active_cards as u64);
        self.action_payload_cards_observed = self
            .action_payload_cards_observed
            .saturating_add(summary.action_payload_cards as u64);
        self.action_payload_placeholder_cards = self
            .action_payload_placeholder_cards
            .saturating_add(summary.action_payload_placeholder_cards as u64);

        if !summary.duplicate_groups.is_empty() {
            self.states_with_duplicate_active_uuid =
                self.states_with_duplicate_active_uuid.saturating_add(1);
            self.duplicate_active_uuid_observations = self
                .duplicate_active_uuid_observations
                .saturating_add(summary.duplicate_groups.len() as u64);
        }
        if !summary.conflict_groups.is_empty() {
            self.states_with_uuid_card_id_conflict =
                self.states_with_uuid_card_id_conflict.saturating_add(1);
            self.uuid_card_id_conflict_observations = self
                .uuid_card_id_conflict_observations
                .saturating_add(summary.conflict_groups.len() as u64);
        }
        for group in &summary.duplicate_groups {
            self.max_duplicate_group_size =
                self.max_duplicate_group_size.max(group.occurrence_count);
            remember_group(
                &mut self.largest_duplicate_groups,
                self.states_observed,
                group,
            );
        }
        for group in &summary.conflict_groups {
            remember_group(
                &mut self.largest_conflict_groups,
                self.states_observed,
                group,
            );
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsCardIdentity {
        CombatSearchV2DiagnosticsCardIdentity {
            audit_policy: "active_combat_card_uuid_scan_plus_action_payload_placeholder_count",
            behavioral_effect: "diagnostic_only_no_search_ordering_no_prune_no_merge",
            states_observed: self.states_observed,
            active_cards_observed: self.active_cards_observed,
            action_payload_cards_observed: self.action_payload_cards_observed,
            action_payload_placeholder_cards: self.action_payload_placeholder_cards,
            states_with_duplicate_active_uuid: self.states_with_duplicate_active_uuid,
            duplicate_active_uuid_observations: self.duplicate_active_uuid_observations,
            states_with_uuid_card_id_conflict: self.states_with_uuid_card_id_conflict,
            uuid_card_id_conflict_observations: self.uuid_card_id_conflict_observations,
            max_duplicate_group_size: self.max_duplicate_group_size,
            largest_duplicate_groups: observed_group_reports(&self.largest_duplicate_groups),
            largest_conflict_groups: observed_group_reports(&self.largest_conflict_groups),
            notes: vec![
                "active cards include hand/draw/discard/exhaust/limbo and queued card plays",
                "action payload cards are counted separately because queued actions can hold constructors or direct-play payloads",
                "uuid 0 action payloads are placeholders before materialization, not active card instances",
                "duplicate active uuid observations are diagnostics and do not prove unsoundness without inspecting locations",
            ],
        }
    }
}

fn remember_group(
    groups: &mut Vec<CardIdentityObservedGroup>,
    observed_at_state_query: u64,
    group: &CardIdentityGroupSummary,
) {
    groups.push(CardIdentityObservedGroup {
        observed_at_state_query,
        group: group.clone(),
    });
    groups.sort_by(|left, right| {
        right
            .group
            .occurrence_count
            .cmp(&left.group.occurrence_count)
            .then_with(|| {
                right
                    .group
                    .distinct_card_labels
                    .len()
                    .cmp(&left.group.distinct_card_labels.len())
            })
            .then_with(|| {
                left.observed_at_state_query
                    .cmp(&right.observed_at_state_query)
            })
    });
    groups.truncate(SAMPLE_LIMIT);
}

fn observed_group_reports(
    groups: &[CardIdentityObservedGroup],
) -> Vec<CombatSearchV2DiagnosticsCardIdentitySample> {
    groups
        .iter()
        .map(|group| CombatSearchV2DiagnosticsCardIdentitySample {
            observed_at_state_query: group.observed_at_state_query,
            uuid: group.group.uuid,
            occurrence_count: group.group.occurrence_count,
            distinct_card_labels: group.group.distinct_card_labels.clone(),
            locations: group.group.locations.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::action::Action;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::blank_test_combat;

    #[test]
    fn duplicate_active_uuid_is_reported() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 10),
        ];

        let summary = summarize_card_identity(&combat);
        let mut collector = CardIdentityDiagnosticsCollector::default();
        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(report.states_with_duplicate_active_uuid, 1);
        assert_eq!(report.states_with_uuid_card_id_conflict, 1);
        assert_eq!(report.largest_conflict_groups[0].uuid, 10);
        assert_eq!(report.largest_conflict_groups[0].occurrence_count, 2);
    }

    #[test]
    fn action_payload_placeholder_is_not_active_duplicate() {
        let mut combat = blank_test_combat();
        combat
            .engine
            .action_queue
            .push_back(Action::MakeCopyInHand {
                original: Box::new(CombatCard::new(CardId::Strike, 0)),
                amount: 1,
            });

        let summary = summarize_card_identity(&combat);
        let mut collector = CardIdentityDiagnosticsCollector::default();
        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(report.states_with_duplicate_active_uuid, 0);
        assert_eq!(report.action_payload_cards_observed, 1);
        assert_eq!(report.action_payload_placeholder_cards, 1);
    }
}
