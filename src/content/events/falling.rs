use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

// internal_state packs pre-selected card indices:
// bits[0..9] = skill card deck index (0x3FF = no skill)
// bits[10..19] = power card deck index (0x3FF = no power)
// bits[20..29] = attack card deck index (0x3FF = no attack)
const NO_CARD: i32 = 0x3FF;

fn skill_idx(s: i32) -> usize { (s & 0x3FF) as usize }
fn power_idx(s: i32) -> usize { ((s >> 10) & 0x3FF) as usize }
fn attack_idx(s: i32) -> usize { ((s >> 20) & 0x3FF) as usize }

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let s = event_state.internal_state;
            let has_skill = (s & 0x3FF) != NO_CARD;
            let has_power = ((s >> 10) & 0x3FF) != NO_CARD;
            let has_attack = ((s >> 20) & 0x3FF) != NO_CARD;

            if !has_skill && !has_power && !has_attack {
                return vec![EventChoiceMeta::new("[Land Safely] Nothing happens.")];
            }

            let mut choices = vec![];
            if has_skill {
                choices.push(EventChoiceMeta::new("[Skill] Remove a Skill."));
            } else {
                choices.push(EventChoiceMeta::disabled("[Skill] No Skills.", "No Skills"));
            }
            if has_power {
                choices.push(EventChoiceMeta::new("[Power] Remove a Power."));
            } else {
                choices.push(EventChoiceMeta::disabled("[Power] No Powers.", "No Powers"));
            }
            if has_attack {
                choices.push(EventChoiceMeta::new("[Attack] Remove an Attack."));
            } else {
                choices.push(EventChoiceMeta::disabled("[Attack] No Attacks.", "No Attacks"));
            }
            choices
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        },
        1 => {
            let s = event_state.internal_state;
            let card_idx = match choice_idx {
                0 => skill_idx(s),
                1 => power_idx(s),
                _ => attack_idx(s),
            };
            if card_idx < run_state.master_deck.len() {
                run_state.master_deck.remove(card_idx);
            }
            event_state.current_screen = 2;
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

/// Initialize Falling state: pre-select cards using miscRng (matches Java constructor).
/// Java calls CardHelper.returnCardOfType(type, miscRng) for each present type,
/// which uses miscRng.random(cards.size() - 1)
pub fn init_falling_state(run_state: &mut RunState) -> i32 {
    let mut s_idx = NO_CARD;
    let mut p_idx = NO_CARD;
    let mut a_idx = NO_CARD;

    // Skills
    let skills: Vec<usize> = run_state.master_deck.iter().enumerate()
        .filter(|(_, c)| crate::content::cards::get_card_definition(c.id).card_type == crate::content::cards::CardType::Skill)
        .map(|(i, _)| i).collect();
    if !skills.is_empty() {
        let pick = run_state.rng_pool.misc_rng.random_range(0, (skills.len() - 1) as i32) as usize;
        s_idx = skills[pick] as i32;
    }

    // Powers
    let powers: Vec<usize> = run_state.master_deck.iter().enumerate()
        .filter(|(_, c)| crate::content::cards::get_card_definition(c.id).card_type == crate::content::cards::CardType::Power)
        .map(|(i, _)| i).collect();
    if !powers.is_empty() {
        let pick = run_state.rng_pool.misc_rng.random_range(0, (powers.len() - 1) as i32) as usize;
        p_idx = powers[pick] as i32;
    }

    // Attacks
    let attacks: Vec<usize> = run_state.master_deck.iter().enumerate()
        .filter(|(_, c)| crate::content::cards::get_card_definition(c.id).card_type == crate::content::cards::CardType::Attack)
        .map(|(i, _)| i).collect();
    if !attacks.is_empty() {
        let pick = run_state.rng_pool.misc_rng.random_range(0, (attacks.len() - 1) as i32) as usize;
        a_idx = attacks[pick] as i32;
    }

    (s_idx & 0x3FF) | ((p_idx & 0x3FF) << 10) | ((a_idx & 0x3FF) << 20)
}

