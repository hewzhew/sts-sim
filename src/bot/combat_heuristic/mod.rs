use crate::combat::CombatState;
use crate::state::core::ClientInput;
use std::collections::HashSet;

mod apply;
mod scoring;
mod sim;

use apply::apply_play;
use scoring::{evaluate, play_priority};
use sim::{build_sim_state, fast_hash, get_plays, Play, SimState, MAX_STATES};

pub fn decide_heuristic(combat: &CombatState) -> ClientInput {
    let init = build_sim_state(combat);

    if (0..init.monster_count as usize).all(|i| init.monsters[i].is_gone) {
        return ClientInput::EndTurn;
    }

    if let Some(potion_input) = super::potions::should_use_potion(combat) {
        return potion_input;
    }

    let mut best_score = evaluate(&init);
    let mut best_first: Option<Play> = None;
    let mut seen = HashSet::with_capacity(8192);
    seen.insert(fast_hash(&init));

    dfs(&init, None, &mut best_score, &mut best_first, &mut seen);

    if best_first.is_none() {
        let mut fallback: Option<(Play, i64, i32)> = None;
        for (card_idx, target) in get_plays(&init) {
            let mut next = init;
            apply_play(&mut next, card_idx, target);
            let score = evaluate(&next);
            let priority = play_priority(&init, card_idx);
            if score > best_score || (score == best_score && priority > 0) {
                match fallback {
                    Some((_, best_fallback_score, best_fallback_priority))
                        if score < best_fallback_score
                            || (score == best_fallback_score
                                && priority <= best_fallback_priority) => {}
                    _ => fallback = Some(((card_idx, target), score, priority)),
                }
            }
        }
        if let Some((play, _, _)) = fallback {
            best_first = Some(play);
        }
    }

    match best_first {
        Some((idx, target)) => ClientInput::PlayCard {
            card_index: idx,
            target,
        },
        None => ClientInput::EndTurn,
    }
}

pub fn describe_end_turn_options(combat: &CombatState) -> Vec<String> {
    let state = build_sim_state(combat);
    let end_score = evaluate(&state);
    let plays = get_plays(&state);
    if plays.is_empty() {
        return vec![format!("END score={} no_legal_plays", end_score)];
    }

    let mut lines = vec![format!(
        "END score={} legal_plays={}",
        end_score,
        plays.len()
    )];
    let mut scored: Vec<(i64, i32, usize, Option<usize>)> = plays
        .into_iter()
        .map(|(card_idx, target)| {
            let mut next = state;
            apply_play(&mut next, card_idx, target);
            (
                evaluate(&next),
                play_priority(&state, card_idx),
                card_idx,
                target,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.cmp(a));
    for (score, priority, card_idx, target) in scored.into_iter().take(8) {
        let card = &combat.zones.hand[card_idx];
        let target_label = target
            .map(|t| t.to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "play idx={} card={} target={} score={} priority={}",
            card_idx, card.id as u16, target_label, score, priority
        ));
    }
    lines
}

fn dfs(
    state: &SimState,
    first_play: Option<Play>,
    best_score: &mut i64,
    best_first: &mut Option<Play>,
    seen: &mut HashSet<u64>,
) {
    if seen.len() >= MAX_STATES {
        return;
    }

    let mut plays = get_plays(state);
    plays.sort_unstable_by(|a, b| play_priority(state, b.0).cmp(&play_priority(state, a.0)));

    for &(card_idx, target) in &plays {
        let mut ns = *state;
        apply_play(&mut ns, card_idx, target);

        let h = fast_hash(&ns);
        if !seen.insert(h) {
            continue;
        }

        let real_first = first_play.unwrap_or((card_idx, target));
        let score = evaluate(&ns);
        if score > *best_score || (score == *best_score && best_first.is_none()) {
            *best_score = score;
            *best_first = Some(real_first);
        }

        dfs(&ns, Some(real_first), best_score, best_first, seen);
    }
}
