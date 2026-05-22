use crate::content::cards::{get_card_definition, CardType};
use crate::content::potions::get_potion_definition;
use crate::runtime::combat::CombatCard;
use crate::state::core::EngineState;

use super::session::RunControlSession;
use super::view_model::{
    build_run_control_view_model, combat_card_label, deck_summary, reward_card_label,
    DecisionCandidate,
};

mod combat;
mod map;

pub use combat::{render_combat_zone_panel, CombatZonePanel};
pub use map::render_map_panel;

pub fn render_run_control_main(session: &RunControlSession) -> String {
    let view = build_run_control_view_model(session);
    let mut out = String::new();
    push_header(&mut out, session);
    push_line(&mut out, view.decision.label);
    push_visible_screen(session, &mut out);
    push_line(&mut out, "");
    push_candidates(session, &view.candidates, &mut out);
    push_line(&mut out, "");
    push_line(
        &mut out,
        format!("Inspect: {}", inspectable_panels(session)),
    );
    push_line(&mut out, format!("Command: {}", main_command_hint(session)));
    out
}

pub fn render_deck_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!("Deck {} cards:", session.run_state.master_deck.len()),
    );
    for (idx, card) in session.run_state.master_deck.iter().enumerate() {
        push_line(&mut out, format!("  {idx} {}", card_line(card, false)));
    }
    push_line(&mut out, "");
    push_line(&mut out, "Summary:");
    push_line(&mut out, format!("  {}", deck_summary(&session.run_state)));
    push_line(&mut out, "");
    push_line(&mut out, "Commands: main | inspect <deck_idx> | raw | q");
    out
}

pub fn render_relics_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!("Relics {}:", session.run_state.relics.len()),
    );
    if session.run_state.relics.is_empty() {
        push_line(&mut out, "  none");
    }
    for (idx, relic) in session.run_state.relics.iter().enumerate() {
        let mut flags = Vec::new();
        if relic.counter >= 0 {
            flags.push(format!("counter={}", relic.counter));
        }
        if relic.amount != 0 {
            flags.push(format!("amount={}", relic.amount));
        }
        if relic.used_up {
            flags.push("used".to_string());
        }
        let suffix = if flags.is_empty() {
            String::new()
        } else {
            format!(" | {}", flags.join(", "))
        };
        push_line(&mut out, format!("  {idx} {:?}{suffix}", relic.id));
    }
    push_line(&mut out, "");
    push_line(&mut out, "Commands: main | raw | q");
    out
}

pub fn render_potions_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!("Potions {} slots:", session.run_state.potions.len()),
    );
    for (idx, slot) in session.run_state.potions.iter().enumerate() {
        match slot {
            Some(potion) => {
                let def = get_potion_definition(potion.id);
                push_line(
                    &mut out,
                    format!(
                        "  {idx} {} | {:?} | potency={} | target_required={} | can_use={} | can_discard={}",
                        def.name,
                        def.rarity,
                        def.base_potency,
                        potion.requires_target,
                        potion.can_use,
                        potion.can_discard
                    ),
                );
            }
            None => push_line(&mut out, format!("  {idx} Empty")),
        }
    }
    push_line(&mut out, "");
    push_line(
        &mut out,
        "Commands: main | potion <slot> [target] | discard-potion <slot> | q",
    );
    out
}

pub fn render_inspect_panel(session: &RunControlSession, id: &str) -> String {
    let view = build_run_control_view_model(session);
    let mut out = String::new();
    if let Some(candidate) = view.candidates.iter().find(|candidate| candidate.id == id) {
        push_line(&mut out, format!("Candidate {}:", candidate.id));
        push_line(&mut out, format!("  {}", candidate.label));
        push_line(
            &mut out,
            format!("  command: {}", candidate.action.command_hint()),
        );
        if let Some(note) = candidate.note.as_ref() {
            push_line(&mut out, format!("  note: {note}"));
        }
        if let Some(resolution) = candidate.resolution.as_ref() {
            push_line(&mut out, "  resolution:");
            for line in resolution.detail_lines() {
                push_line(&mut out, format!("    {line}"));
            }
        }
        if let Some(card_detail) = inspect_card_for_visible_candidate(session, id) {
            push_line(&mut out, "");
            out.push_str(&card_detail);
        }
        return out;
    }

    if let Some(card_detail) = inspect_card_reference(session, id) {
        return card_detail;
    }

    format!("Nothing visible with id '{id}'. Use main/deck/map or inspect a visible candidate id.")
}

fn push_header(out: &mut String, session: &RunControlSession) {
    let view = build_run_control_view_model(session);
    push_line(
        out,
        "================================================================================",
    );
    push_line(
        out,
        format!("{} | {}", view.header.location, view.header.title),
    );
    push_line(out, view.header.config);
    push_line(
        out,
        "--------------------------------------------------------------------------------",
    );
}

fn push_visible_screen(session: &RunControlSession, out: &mut String) {
    match &session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => combat::push_combat_screen(session, out),
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            if let Some(cards) = reward.pending_card_choice.as_ref() {
                push_line(out, "");
                push_line(out, "Cards:");
                for (idx, card) in cards.iter().enumerate() {
                    push_line(
                        out,
                        format!("  {idx} {}", reward_card_brief(card.id, card.upgrades)),
                    );
                }
                if reward.skippable {
                    push_line(out, format!("  {} Skip", cards.len()));
                }
            }
        }
        EngineState::MapNavigation => {
            push_line(out, "");
            push_line(out, "Type `map` for the visible route summary.");
        }
        _ => {}
    }
}

fn push_candidates(
    session: &RunControlSession,
    candidates: &[DecisionCandidate],
    out: &mut String,
) {
    push_line(out, candidate_section_title(session));
    if candidates.is_empty() {
        push_line(out, "  none");
        return;
    }
    for candidate in candidates {
        push_line(out, format!("  {} | {}", candidate.id, candidate.label));
        if let Some(note) = candidate
            .note
            .as_ref()
            .filter(|note| note.as_str() != "routine")
        {
            push_line(out, format!("      {note}"));
        }
    }
}

fn candidate_section_title(session: &RunControlSession) -> &'static str {
    match &session.engine_state {
        EngineState::EventRoom => {
            if session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            }) {
                "Options:"
            } else {
                "Available action:"
            }
        }
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => "Actions:",
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => "Choices:",
        EngineState::MapNavigation => "Paths:",
        _ => "Available actions:",
    }
}

pub(super) fn debug_words(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn inspectable_panels(session: &RunControlSession) -> &'static str {
    match session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "deck | draw | discard | exhaust | relics | potions | inspect <id> | details | raw"
        }
        _ => "deck | map | relics | potions | inspect <id> | details | raw",
    }
}

fn main_command_hint(session: &RunControlSession) -> String {
    let view = build_run_control_view_model(session);
    let first = view.candidates.first();
    let primary = match first {
        Some(candidate) if view.candidates.len() == 1 => {
            format!("Enter/{}: {}", candidate.id, candidate.label)
        }
        Some(_) => state_command_hint(session),
        None => "type a command".to_string(),
    };
    let views = match session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "draw | discard | exhaust | potions | relics | case | raw | help | q"
        }
        _ => "deck | map | relics | potions | case | raw | help | q",
    };
    format!("{primary} | {views}")
}

fn state_command_hint(session: &RunControlSession) -> String {
    match session.engine_state {
        EngineState::Shop(_) => "type visible id: card-2 / relic-1 / potion-0 / leave".to_string(),
        EngineState::Campfire => "rest | smith-<deck_idx> | recall".to_string(),
        EngineState::MapNavigation => "type a path id, e.g. 0 or 5".to_string(),
        EngineState::RewardScreen(_) => "type visible id, pick <idx>, or skip".to_string(),
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => "type visible action id, end, or n".to_string(),
        _ => "type visible id".to_string(),
    }
}

pub(super) fn card_line(card: &CombatCard, runtime_cost: bool) -> String {
    let def = get_card_definition(card.id);
    let name = combat_card_label(card);
    let cost = if runtime_cost {
        card.cost_for_turn_java()
    } else {
        def.cost as i32
    };
    let mut parts = vec![
        name,
        format!("{:?}", def.card_type),
        format!("{:?}", def.rarity),
        format!("cost {}", format_cost(cost)),
    ];
    if def.base_damage > 0 {
        parts.push(format!(
            "damage {}",
            def.base_damage + def.upgrade_damage * card.upgrades as i32
        ));
    }
    if def.base_block > 0 {
        parts.push(format!(
            "block {}",
            def.base_block + def.upgrade_block * card.upgrades as i32
        ));
    }
    if def.base_magic > 0 {
        parts.push(format!(
            "magic {}",
            def.base_magic + def.upgrade_magic * card.upgrades as i32
        ));
    }
    if def.exhaust {
        parts.push("exhaust".to_string());
    }
    parts.join(" | ")
}

fn reward_card_brief(id: crate::content::cards::CardId, upgrades: u8) -> String {
    let def = get_card_definition(id);
    format!(
        "{} | {:?} | {:?} | cost {}",
        reward_card_label(id, upgrades),
        def.card_type,
        def.rarity,
        format_cost(def.cost as i32)
    )
}

fn inspect_card_for_visible_candidate(session: &RunControlSession, id: &str) -> Option<String> {
    let idx = id.split('.').next()?.parse::<usize>().ok()?;
    match &session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            let combat = session
                .active_combat
                .as_ref()
                .map(|active| &active.combat_state)?;
            let card = combat.zones.hand.get(idx)?;
            Some(card_detail(
                &combat_card_label(card),
                card.id,
                card.upgrades,
            ))
        }
        EngineState::RewardScreen(reward) => {
            let cards = reward.pending_card_choice.as_ref()?;
            let card = cards.get(idx)?;
            Some(card_detail(
                &reward_card_label(card.id, card.upgrades),
                card.id,
                card.upgrades,
            ))
        }
        EngineState::RunPendingChoice(_) => inspect_deck_card(session, idx),
        _ => None,
    }
}

fn inspect_card_reference(session: &RunControlSession, id: &str) -> Option<String> {
    let idx = id.parse::<usize>().ok()?;
    inspect_deck_card(session, idx)
}

fn inspect_deck_card(session: &RunControlSession, idx: usize) -> Option<String> {
    let card = session.run_state.master_deck.get(idx)?;
    Some(card_detail(
        &combat_card_label(card),
        card.id,
        card.upgrades,
    ))
}

fn card_detail(name: &str, id: crate::content::cards::CardId, upgrades: u8) -> String {
    let def = get_card_definition(id);
    let mut out = String::new();
    push_line(&mut out, format!("Card: {name}"));
    push_line(
        &mut out,
        format!(
            "  type={:?} rarity={:?} target={:?} cost={}",
            def.card_type,
            def.rarity,
            def.target,
            format_cost(def.cost as i32)
        ),
    );
    if def.card_type == CardType::Attack || def.base_damage > 0 {
        push_line(
            &mut out,
            format!(
                "  damage={} upgrade_delta={}",
                def.base_damage + def.upgrade_damage * upgrades as i32,
                def.upgrade_damage
            ),
        );
    }
    if def.base_block > 0 {
        push_line(
            &mut out,
            format!(
                "  block={} upgrade_delta={}",
                def.base_block + def.upgrade_block * upgrades as i32,
                def.upgrade_block
            ),
        );
    }
    if def.base_magic > 0 {
        push_line(
            &mut out,
            format!(
                "  magic={} upgrade_delta={}",
                def.base_magic + def.upgrade_magic * upgrades as i32,
                def.upgrade_magic
            ),
        );
    }
    let flags = [
        (def.exhaust, "exhaust"),
        (def.ethereal, "ethereal"),
        (def.innate, "innate"),
    ]
    .iter()
    .filter_map(|(enabled, label)| enabled.then_some(*label))
    .collect::<Vec<_>>();
    if !flags.is_empty() {
        push_line(&mut out, format!("  flags={}", flags.join(", ")));
    }
    out
}

pub(super) fn format_range(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

pub(super) fn format_first_floor(floor: Option<i32>) -> String {
    floor
        .map(|floor| format!("floor {floor}"))
        .unwrap_or_else(|| "none".to_string())
}

fn format_cost(cost: i32) -> String {
    match cost {
        -1 => "X".to_string(),
        -2 => "unplayable".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::session::{RunControlConfig, RunControlSession};

    #[test]
    fn deck_panel_shows_real_cards_before_summary() {
        let session = RunControlSession::new(RunControlConfig::default());
        let rendered = render_deck_panel(&session);

        assert!(rendered.contains("Deck 10 cards:"));
        assert!(rendered.contains("0 Strike"));
        assert!(rendered.contains("Summary:"));
        assert!(rendered.contains("attacks=6"));
    }
}
