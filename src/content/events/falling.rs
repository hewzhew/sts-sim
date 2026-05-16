use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventOption,
    EventOptionConstraint, EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

// internal_state packs pre-selected card indices:
// bits[0..9] = skill card deck index (0x3FF = no skill)
// bits[10..19] = power card deck index (0x3FF = no power)
// bits[20..29] = attack card deck index (0x3FF = no attack)
const NO_CARD: i32 = 0x3FF;

fn skill_idx(s: i32) -> usize {
    (s & 0x3FF) as usize
}
fn power_idx(s: i32) -> usize {
    ((s >> 10) & 0x3FF) as usize
}
fn attack_idx(s: i32) -> usize {
    ((s >> 20) & 0x3FF) as usize
}

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => vec![EventOption::new(
            EventChoiceMeta::new("[Proceed]"),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        1 => {
            let s = event_state.internal_state;
            let has_skill = (s & 0x3FF) != NO_CARD;
            let has_power = ((s >> 10) & 0x3FF) != NO_CARD;
            let has_attack = ((s >> 20) & 0x3FF) != NO_CARD;

            if !has_skill && !has_power && !has_attack {
                return vec![EventOption::new(
                    EventChoiceMeta::new("[Land Safely] Nothing happens."),
                    EventOptionSemantics {
                        action: EventActionKind::Decline,
                        effects: vec![],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                )];
            }

            let mut choices = vec![];
            if has_skill {
                let effect = card_remove_effect(run_state, skill_idx(s));
                choices.push(EventOption::new(
                    EventChoiceMeta::new("[Skill] Remove a Skill."),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![effect],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled("[Skill] No Skills.", "No Skills"),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            if has_power {
                let effect = card_remove_effect(run_state, power_idx(s));
                choices.push(EventOption::new(
                    EventChoiceMeta::new("[Power] Remove a Power."),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![effect],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled("[Power] No Powers.", "No Powers"),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            if has_attack {
                let effect = card_remove_effect(run_state, attack_idx(s));
                choices.push(EventOption::new(
                    EventChoiceMeta::new("[Attack] Remove an Attack."),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![effect],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled("[Attack] No Attacks.", "No Attacks"),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![],
                        constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            choices
        }
        _ => vec![EventOption::new(
            EventChoiceMeta::new("[Leave]"),
            EventOptionSemantics {
                action: EventActionKind::Leave,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::Complete,
                repeatable: false,
                terminal: true,
            },
        )],
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

fn card_remove_effect(run_state: &RunState, deck_idx: usize) -> EventEffect {
    match run_state.master_deck.get(deck_idx) {
        Some(card) => EventEffect::RemoveCard {
            count: 1,
            target_uuid: Some(card.uuid),
            kind: EventCardKind::Specific(card.id),
        },
        None => EventEffect::RemoveCard {
            count: 1,
            target_uuid: None,
            kind: EventCardKind::Unknown,
        },
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            let s = event_state.internal_state;
            let card_idx = match choice_idx {
                0 => skill_idx(s),
                1 => power_idx(s),
                _ => attack_idx(s),
            };
            if card_idx < run_state.master_deck.len() {
                let uuid = run_state.master_deck[card_idx].uuid;
                run_state.remove_card_from_deck_with_source(
                    uuid,
                    DomainEventSource::Event(crate::state::events::EventId::Falling),
                );
            }
            event_state.current_screen = 2;
        }
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
    let skills: Vec<usize> = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            !crate::state::core::master_deck_card_is_bottled(c, &run_state.relics)
                && crate::content::cards::get_card_definition(c.id).card_type
                    == crate::content::cards::CardType::Skill
        })
        .map(|(i, _)| i)
        .collect();
    if !skills.is_empty() {
        let pick = run_state
            .rng_pool
            .misc_rng
            .random_range(0, (skills.len() - 1) as i32) as usize;
        s_idx = skills[pick] as i32;
    }

    // Powers
    let powers: Vec<usize> = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            !crate::state::core::master_deck_card_is_bottled(c, &run_state.relics)
                && crate::content::cards::get_card_definition(c.id).card_type
                    == crate::content::cards::CardType::Power
        })
        .map(|(i, _)| i)
        .collect();
    if !powers.is_empty() {
        let pick = run_state
            .rng_pool
            .misc_rng
            .random_range(0, (powers.len() - 1) as i32) as usize;
        p_idx = powers[pick] as i32;
    }

    // Attacks
    let attacks: Vec<usize> = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            !crate::state::core::master_deck_card_is_bottled(c, &run_state.relics)
                && crate::content::cards::get_card_definition(c.id).card_type
                    == crate::content::cards::CardType::Attack
        })
        .map(|(i, _)| i)
        .collect();
    if !attacks.is_empty() {
        let pick = run_state
            .rng_pool
            .misc_rng
            .random_range(0, (attacks.len() - 1) as i32) as usize;
        a_idx = attacks[pick] as i32;
    }

    (s_idx & 0x3FF) | ((p_idx & 0x3FF) << 10) | ((a_idx & 0x3FF) << 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::selection::DomainEvent;

    #[test]
    fn falling_skill_option_exposes_remove_operation() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.add_card_to_deck(CardId::ShrugItOff);
        let skill_index = rs.master_deck.len() - 1;
        let state = EventState {
            id: crate::state::events::EventId::Falling,
            current_screen: 1,
            internal_state: (skill_index as i32 & 0x3FF)
                | ((NO_CARD & 0x3FF) << 10)
                | ((NO_CARD & 0x3FF) << 20),
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };
        let options = get_options(&rs, &state);
        assert!(matches!(
            options[0].semantics.effects.as_slice(),
            [EventEffect::RemoveCard {
                count: 1,
                target_uuid: Some(_),
                kind: EventCardKind::Specific(CardId::ShrugItOff),
            }]
        ));
    }

    #[test]
    fn falling_init_ignores_bottled_cards_like_java_card_helper() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![
            CombatCard::new(CardId::Defend, 11),
            CombatCard::new(CardId::Inflame, 12),
            CombatCard::new(CardId::Strike, 13),
        ];
        let mut bottled_skill = RelicState::new(RelicId::BottledLightning);
        bottled_skill.amount = 11;
        rs.relics.push(bottled_skill);

        let state = init_falling_state(&mut rs);

        assert_eq!(skill_idx(state), NO_CARD as usize);
        assert_eq!(power_idx(state), 1);
        assert_eq!(attack_idx(state), 2);
    }

    #[test]
    fn falling_removal_uses_event_domain_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![CombatCard::new(CardId::Strike, 11)];
        rs.event_state = Some(EventState {
            id: crate::state::events::EventId::Falling,
            current_screen: 1,
            internal_state: ((NO_CARD & 0x3FF) << 10) | (0 << 20) | (NO_CARD & 0x3FF),
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 2);

        assert!(rs.master_deck.is_empty());
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardRemoved {
                card,
                source: DomainEventSource::Event(crate::state::events::EventId::Falling),
            } if card.uuid == 11
        )));
    }
}
