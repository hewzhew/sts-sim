use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

fn get_damage(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        (run_state.max_hp as f32 * 0.3).round() as i32
    } else {
        (run_state.max_hp as f32 * 0.2).round() as i32
    }
}

fn has_upgradable_cards(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        c.id == crate::content::cards::CardId::SearingBlow
            || (c.upgrades == 0
                && def.card_type != crate::content::cards::CardType::Status
                && def.card_type != crate::content::cards::CardType::Curse)
    })
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let damage = get_damage(run_state);
    let mut choices = Vec::new();

    if has_upgradable_cards(run_state) {
        choices.push(EventChoiceMeta::new(format!(
            "[Enter the Light] Take {} damage. Upgrade 2 random cards.",
            damage
        )));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Enter the Light] No upgradable cards.",
            "No upgradable cards in your deck.",
        ));
    }

    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Enter Light: take damage + upgrade 2 random cards
                    let damage = get_damage(run_state);
                    let damage = apply_enter_light_damage(run_state, damage);
                    run_state.change_hp_with_source(
                        -damage,
                        DomainEventSource::Event(EventId::ShiningLight),
                    );
                    run_state.upgrade_random_cards_with_source(
                        2,
                        DomainEventSource::Event(EventId::ShiningLight),
                    );
                }
                _ => {
                    // Leave
                }
            }
            event_state.current_screen = 1;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

fn apply_enter_light_damage(run_state: &RunState, mut damage: i32) -> i32 {
    // Java: DamageInfo(player, damage), normal damage. Out of combat there is no
    // block, but relic hooks still matter: Torii runs in onAttacked and
    // Tungsten Rod runs later in onLoseHpLast.
    if damage > 1
        && damage <= 5
        && run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::Torii)
    {
        damage = 1;
    }
    if damage > 0
        && run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
    {
        damage -= 1;
    }
    damage
}

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn shining_run(current_hp: i32, max_hp: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.event_state = Some(EventState::new(EventId::ShiningLight));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn enter_light_damage_and_random_upgrades_use_event_source() {
        let mut run_state = shining_run(80, 80);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 64);
        assert_eq!(
            run_state
                .master_deck
                .iter()
                .filter(|card| card.upgrades > 0)
                .count(),
            2
        );
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -16,
                current_hp: 64,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::ShiningLight),
            }
        )));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    DomainEvent::CardUpgraded {
                        source: DomainEventSource::Event(EventId::ShiningLight),
                        ..
                    }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn enter_light_normal_damage_applies_torii_then_tungsten() {
        let mut run_state = shining_run(20, 20);
        run_state.relics.push(RelicState::new(RelicId::Torii));
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 20);
        assert!(!run_state
            .take_emitted_events()
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }

    #[test]
    fn leave_does_not_damage_or_upgrade() {
        let mut run_state = shining_run(80, 80);
        let before = run_state.master_deck.clone();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.current_hp, 80);
        assert_eq!(
            run_state
                .master_deck
                .iter()
                .map(|card| (card.id, card.upgrades))
                .collect::<Vec<_>>(),
            before
                .iter()
                .map(|card| (card.id, card.upgrades))
                .collect::<Vec<_>>()
        );
        assert!(run_state.take_emitted_events().is_empty());
    }
}
