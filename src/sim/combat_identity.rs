use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState, PowerPayload};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CombatCardIdentityAudit {
    pub active_cards: usize,
    pub master_deck_cards: usize,
    pub action_payload_cards: usize,
    pub action_payload_placeholder_cards: usize,
    pub card_uuid_counter: u32,
    pub max_referenced_card_uuid: Option<u32>,
    pub stale_card_uuid_counter: bool,
    pub duplicate_active_uuid_groups: Vec<CombatCardIdentityGroup>,
    pub uuid_card_id_conflict_groups: Vec<CombatCardIdentityGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatCardIdentityGroup {
    pub uuid: u32,
    pub occurrence_count: usize,
    pub distinct_card_labels: Vec<String>,
    pub distinct_card_id_labels: Vec<String>,
    pub locations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CombatCardIdentityOccurrence {
    location: String,
    card_label: String,
    card_id_label: String,
}

impl CombatCardIdentityAudit {
    pub fn has_uuid_card_id_conflict(&self) -> bool {
        !self.uuid_card_id_conflict_groups.is_empty()
    }

    pub fn capture_rejection_reason(&self) -> Option<String> {
        let group = self.uuid_card_id_conflict_groups.first()?;
        Some(format!(
            "combat card identity conflict: active uuid {} maps to multiple card ids [{}] at [{}]",
            group.uuid,
            group.distinct_card_id_labels.join(", "),
            group.locations.join(", ")
        ))
    }

    pub fn stale_counter_rejection_reason(&self) -> Option<String> {
        if !self.stale_card_uuid_counter {
            return None;
        }
        Some(format!(
            "combat card identity conflict: card_uuid_counter {} is not fresh after max referenced uuid {}",
            self.card_uuid_counter,
            self.max_referenced_card_uuid.unwrap_or(0)
        ))
    }
}

pub fn audit_combat_card_identity(combat: &CombatState) -> CombatCardIdentityAudit {
    let mut active_by_uuid: BTreeMap<u32, Vec<CombatCardIdentityOccurrence>> = BTreeMap::new();
    collect_active_cards("hand", &combat.zones.hand, &mut active_by_uuid);
    collect_active_cards("draw", &combat.zones.draw_pile, &mut active_by_uuid);
    collect_active_cards("discard", &combat.zones.discard_pile, &mut active_by_uuid);
    collect_active_cards("exhaust", &combat.zones.exhaust_pile, &mut active_by_uuid);
    collect_active_cards("limbo", &combat.zones.limbo, &mut active_by_uuid);
    for (idx, queued) in combat.zones.queued_cards.iter().enumerate() {
        push_card_occurrence(&mut active_by_uuid, format!("queued[{idx}]"), &queued.card);
    }

    let mut max_referenced_card_uuid = None;
    for card in combat
        .meta
        .master_deck_snapshot
        .iter()
        .chain(combat.zones.hand.iter())
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
    {
        remember_max_uuid(&mut max_referenced_card_uuid, card.uuid);
    }
    for queued in &combat.zones.queued_cards {
        remember_max_uuid(&mut max_referenced_card_uuid, queued.card.uuid);
    }

    let mut duplicate_active_uuid_groups = Vec::new();
    let mut uuid_card_id_conflict_groups = Vec::new();
    for (uuid, occurrences) in active_by_uuid {
        if occurrences.len() > 1 {
            duplicate_active_uuid_groups.push(group_summary(uuid, &occurrences));
        }
        let distinct_ids = occurrences
            .iter()
            .map(|occurrence| occurrence.card_id_label.as_str())
            .collect::<BTreeSet<_>>();
        if distinct_ids.len() > 1 {
            uuid_card_id_conflict_groups.push(group_summary(uuid, &occurrences));
        }
    }

    let action_payload_cards = action_payload_cards(&combat.engine.action_queue);
    let action_payload_placeholder_cards = action_payload_cards
        .iter()
        .filter(|card| card.uuid == 0)
        .count();
    for card in action_payload_cards.iter().filter(|card| card.uuid != 0) {
        remember_max_uuid(&mut max_referenced_card_uuid, card.uuid);
    }
    let stale_card_uuid_counter =
        max_referenced_card_uuid.is_some_and(|max_uuid| combat.zones.card_uuid_counter < max_uuid);

    CombatCardIdentityAudit {
        active_cards: active_card_count(combat),
        master_deck_cards: combat.meta.master_deck_snapshot.len(),
        action_payload_cards: action_payload_cards.len(),
        action_payload_placeholder_cards,
        card_uuid_counter: combat.zones.card_uuid_counter,
        max_referenced_card_uuid,
        stale_card_uuid_counter,
        duplicate_active_uuid_groups,
        uuid_card_id_conflict_groups,
    }
}

pub fn validate_combat_card_identity_for_capture(combat: &CombatState) -> Result<(), String> {
    let audit = audit_combat_card_identity(combat);
    if let Some(reason) = audit.capture_rejection_reason() {
        return Err(reason);
    }
    if let Some(reason) = audit.stale_counter_rejection_reason() {
        return Err(reason);
    }
    Ok(())
}

fn remember_max_uuid(max_uuid: &mut Option<u32>, uuid: u32) {
    *max_uuid = Some(max_uuid.map_or(uuid, |current| current.max(uuid)));
}

fn collect_active_cards(
    zone: &str,
    cards: &[CombatCard],
    by_uuid: &mut BTreeMap<u32, Vec<CombatCardIdentityOccurrence>>,
) {
    for (idx, card) in cards.iter().enumerate() {
        push_card_occurrence(by_uuid, format!("{zone}[{idx}]"), card);
    }
}

fn push_card_occurrence(
    by_uuid: &mut BTreeMap<u32, Vec<CombatCardIdentityOccurrence>>,
    location: String,
    card: &CombatCard,
) {
    by_uuid
        .entry(card.uuid)
        .or_default()
        .push(CombatCardIdentityOccurrence {
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

fn action_payload_cards(queue: &VecDeque<Action>) -> Vec<&CombatCard> {
    let mut cards = Vec::new();
    for action in queue {
        collect_action_payload_cards(action, &mut cards);
    }
    cards
}

fn collect_action_payload_cards<'a>(action: &'a Action, cards: &mut Vec<&'a CombatCard>) {
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

fn group_summary(
    uuid: u32,
    occurrences: &[CombatCardIdentityOccurrence],
) -> CombatCardIdentityGroup {
    CombatCardIdentityGroup {
        uuid,
        occurrence_count: occurrences.len(),
        distinct_card_labels: occurrences
            .iter()
            .map(|occurrence| occurrence.card_label.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        distinct_card_id_labels: occurrences
            .iter()
            .map(|occurrence| occurrence.card_id_label.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        locations: occurrences
            .iter()
            .map(|occurrence| occurrence.location.clone())
            .collect(),
    }
}

fn card_label(card: &CombatCard) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::action::Action;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::blank_test_combat;

    #[test]
    fn combat_card_identity_reports_uuid_card_id_conflict() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 10),
        ];

        let audit = audit_combat_card_identity(&combat);

        assert_eq!(audit.active_cards, 2);
        assert_eq!(audit.master_deck_cards, 0);
        assert_eq!(audit.duplicate_active_uuid_groups.len(), 1);
        assert_eq!(audit.uuid_card_id_conflict_groups.len(), 1);
        assert!(validate_combat_card_identity_for_capture(&combat)
            .expect_err("capture validation should reject conflicting card identity")
            .contains("active uuid 10"));
    }

    #[test]
    fn combat_card_identity_allows_same_uuid_same_card_id_instances() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Strike, 10),
        ];
        combat.zones.card_uuid_counter = 11;

        let audit = audit_combat_card_identity(&combat);

        assert_eq!(audit.duplicate_active_uuid_groups.len(), 1);
        assert!(audit.uuid_card_id_conflict_groups.is_empty());
        validate_combat_card_identity_for_capture(&combat)
            .expect("same-card same-uuid Java same-instance paths should remain capturable");
    }

    #[test]
    fn combat_card_identity_counts_action_payload_placeholders_separately() {
        let mut combat = blank_test_combat();
        combat
            .engine
            .action_queue
            .push_back(Action::MakeCopyInHand {
                original: Box::new(CombatCard::new(CardId::Strike, 0)),
                amount: 1,
            });

        let audit = audit_combat_card_identity(&combat);

        assert!(audit.duplicate_active_uuid_groups.is_empty());
        assert_eq!(audit.action_payload_cards, 1);
        assert_eq!(audit.action_payload_placeholder_cards, 1);
        validate_combat_card_identity_for_capture(&combat)
            .expect("action payload cards are not active card identity conflicts");
    }

    #[test]
    fn combat_card_identity_rejects_stale_uuid_counter() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 20)];
        combat.zones.card_uuid_counter = 19;

        let audit = audit_combat_card_identity(&combat);

        assert_eq!(audit.max_referenced_card_uuid, Some(20));
        assert!(audit.stale_card_uuid_counter);
        assert!(validate_combat_card_identity_for_capture(&combat)
            .expect_err("fresh card uuid counter should be above all existing cards")
            .contains("card_uuid_counter 19"));
    }
}
