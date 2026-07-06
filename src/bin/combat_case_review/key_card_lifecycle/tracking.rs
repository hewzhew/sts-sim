use sts_simulator::content::cards::java_id;
use sts_simulator::runtime::combat::CombatState;
use sts_simulator::state::core::ClientInput;

use super::targets::key_card_targets;
use super::types::{
    CardZoneAtStep, CardZoneLabel, KeyCardLifecycle, KeyCardLifecycleReport, KeyCardPlay,
    TrackedKeyCard,
};
use super::zones::zone_for_uuid;

pub(super) fn report_without_focus(tracked_cards: Vec<TrackedKeyCard>) -> KeyCardLifecycleReport {
    KeyCardLifecycleReport {
        schema: "key_card_lifecycle_v0",
        contract: "exact_replay_key_card_visibility_and_play_timing_no_strategy_verdict",
        basis_line: "none",
        witness_action_count: None,
        replayed_actions: 0,
        truncated_by_preview: false,
        truncated: false,
        timed_out: false,
        tracked_cards: tracked_cards
            .into_iter()
            .map(|tracked| lifecycle_from_tracked(tracked, 0, CardZoneLabel::Missing))
            .collect(),
    }
}

pub(super) fn tracked_key_cards(combat: &CombatState) -> Vec<TrackedKeyCard> {
    key_card_targets(combat)
        .into_iter()
        .map(|target| {
            let initial_zone = zone_for_uuid(combat, target.card.uuid);
            TrackedKeyCard {
                card: target.card,
                reason: target.reason,
                initial_zone,
                first_seen_zone: CardZoneAtStep {
                    step_index: 0,
                    zone: initial_zone,
                },
                first_play: None,
            }
        })
        .collect()
}

pub(super) fn note_played_key_card(
    tracked_cards: &mut [TrackedKeyCard],
    step_index: usize,
    action_key: &str,
    input: &ClientInput,
) {
    let ClientInput::PlayCard { card_index: _, .. } = input else {
        return;
    };
    for tracked in tracked_cards {
        if tracked.first_play.is_none()
            && action_key.contains(&format!("#{}", tracked.card.uuid))
            && action_key.contains("combat/play_card/")
        {
            tracked.first_play = Some(KeyCardPlay {
                step_index,
                action_key: action_key.to_string(),
                input: input.clone(),
            });
        }
    }
}

pub(super) fn note_seen_zones(
    tracked_cards: &mut [TrackedKeyCard],
    step_index: usize,
    combat: &CombatState,
) {
    for tracked in tracked_cards {
        let zone = zone_for_uuid(combat, tracked.card.uuid);
        if tracked.first_seen_zone.zone == CardZoneLabel::Missing && zone != CardZoneLabel::Missing
        {
            tracked.first_seen_zone = CardZoneAtStep { step_index, zone };
        }
    }
}

pub(super) fn finish_lifecycles(
    tracked_cards: Vec<TrackedKeyCard>,
    replayed_actions: usize,
    combat: &CombatState,
) -> Vec<KeyCardLifecycle> {
    tracked_cards
        .into_iter()
        .map(|tracked| {
            let final_zone = zone_for_uuid(combat, tracked.card.uuid);
            lifecycle_from_tracked(tracked, replayed_actions, final_zone)
        })
        .collect()
}

fn lifecycle_from_tracked(
    tracked: TrackedKeyCard,
    final_step_index: usize,
    final_zone: CardZoneLabel,
) -> KeyCardLifecycle {
    KeyCardLifecycle {
        card: format!("{}+{}", java_id(tracked.card.id), tracked.card.upgrades),
        uuid: tracked.card.uuid,
        upgrades: tracked.card.upgrades,
        reason: tracked.reason,
        initial_zone: tracked.initial_zone,
        first_seen_zone: tracked.first_seen_zone,
        played_in_replay: tracked.first_play.is_some(),
        first_play: tracked.first_play,
        final_zone: CardZoneAtStep {
            step_index: final_step_index,
            zone: final_zone,
        },
    }
}
