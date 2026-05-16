use crate::content::cards::{get_card_definition, CardId, CardRarity};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const LIBRARY_ENTRY_WIDTH: usize = 2;

/// TheLibrary event.
/// Java: 2 options:
///   [Read] Generate 20 unique class cards (rollRarity + getCard, deduped), player picks 1
///   [Sleep] Heal 33% HP (A15: 20% HP)
///
/// Screen 0: initial choice (Read / Sleep)
/// Screen 1: 20 cards to choose from (only when Read was picked)
/// Screen 2: Leave

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let heal_pct = if run_state.ascension_level >= 15 {
                0.20
            } else {
                0.33
            };
            let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
            vec![
                EventChoiceMeta::new("[Read] Choose a card from 20 offerings."),
                EventChoiceMeta::new(format!("[Sleep] Heal {} HP.", heal_amt)),
            ]
        }
        1 => {
            // Show 20 card offerings from extra_data
            let mut choices = Vec::with_capacity(20);
            for idx in 0..library_entry_count(&event_state.extra_data) {
                let Some((card_id, upgrades)) =
                    library_card_entry_at(run_state, &event_state.extra_data, idx)
                else {
                    continue;
                };
                let def = get_card_definition(card_id);
                let upgrade_suffix = if upgrades > 0 { "+" } else { "" };
                choices.push(EventChoiceMeta::new(format!(
                    "{}{} ({:?} {:?})",
                    def.name, upgrade_suffix, def.rarity, def.card_type
                )));
            }
            choices
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Read: generate 20 cards and show them
                    generate_library_cards(run_state, &mut event_state.extra_data);
                    event_state.current_screen = 1;
                }
                _ => {
                    // Sleep: heal
                    let heal_pct = if run_state.ascension_level >= 15 {
                        0.20
                    } else {
                        0.33
                    };
                    let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
                    run_state
                        .heal_with_source(heal_amt, DomainEventSource::Event(EventId::TheLibrary));
                    event_state.current_screen = 2;
                }
            }
        }
        1 => {
            // Pick one of the 20 cards
            if let Some((card_id, upgrades)) =
                library_card_entry_at(run_state, &event_state.extra_data, choice_idx)
            {
                run_state.add_card_to_deck_with_upgrades_from(
                    card_id,
                    upgrades,
                    DomainEventSource::Event(EventId::TheLibrary),
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

/// Generate 20 unique class cards via Java's rollRarity + getCard with dedup.
///
/// Java logic (TheLibrary.buttonEffect case 0):
///   for (int i = 0; i < 20; ++i) {
///       card = getCard(rollRarity());
///       while (group contains card) {
///           card = getCard(rollRarity());  // re-roll both rarity and card
///       }
///       group.add(card);
///   }
///
/// rollRarity() uses cardRng.random(0,99) + cardBlizzRandomizer
/// getCard(rarity) uses cardRng.random(pool.size()-1) via pool.getRandomCard(true)
fn generate_library_cards(run_state: &mut RunState, extra_data: &mut Vec<i32>) {
    extra_data.clear();

    let mut selected: Vec<CardId> = Vec::with_capacity(20);

    for _ in 0..20 {
        let mut card_id = roll_and_get_card(run_state);

        // Dedup: re-roll if we already have this card
        // Java does while(containsDupe) { re-roll both rarity and card }
        // Safety limit to prevent infinite loop (shouldn't happen with 70+ card pool)
        let mut attempts = 0;
        while selected.contains(&card_id) && attempts < 100 {
            card_id = roll_and_get_card(run_state);
            attempts += 1;
        }

        selected.push(card_id);
        let upgrades = run_state.preview_obtain_card_upgrades(card_id, 0);
        push_library_card_entry(extra_data, card_id, upgrades);
    }
}

fn push_library_card_entry(extra_data: &mut Vec<i32>, card_id: CardId, upgrades: u8) {
    extra_data.push(card_id as i32);
    extra_data.push(upgrades as i32);
}

fn library_entry_count(extra_data: &[i32]) -> usize {
    extra_data.len() / LIBRARY_ENTRY_WIDTH
}

fn library_card_entry_at(
    run_state: &RunState,
    extra_data: &[i32],
    idx: usize,
) -> Option<(CardId, u8)> {
    let offset = idx.checked_mul(LIBRARY_ENTRY_WIDTH)?;
    let card_raw = *extra_data.get(offset)?;
    let upgrades_raw = *extra_data.get(offset + 1)?;
    if upgrades_raw < 0 {
        return None;
    }
    Some((
        decode_library_card_id(run_state, card_raw)?,
        upgrades_raw as u8,
    ))
}

fn decode_library_card_id(run_state: &RunState, raw: i32) -> Option<CardId> {
    for rarity in [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare] {
        for &card_id in
            crate::engine::campfire_handler::card_pool_for_class(run_state.player_class, rarity)
        {
            if card_id as i32 == raw {
                return Some(card_id);
            }
        }
    }
    None
}

/// Roll a rarity and get a random card from that rarity pool.
/// Mirrors Java: rollRarity() + getCard(rarity)
///
/// rollRarity: cardRng.random(0,99) + cardBlizzRandomizer
///   - roll < 3 → RARE (fallback rates, since we're in event room)
///   - roll < 40 → UNCOMMON
///   - else → COMMON
///
/// getCard(rarity): pool.getRandomCard(true) → cardRng.random(pool.size()-1)
fn roll_and_get_card(run_state: &mut RunState) -> CardId {
    // Step 1: rollRarity — uses cardRng
    let roll = run_state.rng_pool.card_rng.random_range(0, 99) + run_state.card_blizz_randomizer;

    // Event room uses fallback rarity rates (no combat room rarity adjustments)
    let rarity = if roll < 3 {
        CardRarity::Rare
    } else if roll < 40 {
        CardRarity::Uncommon
    } else {
        CardRarity::Common
    };

    // Step 2: getCard(rarity) — uses cardRng via pool.getRandomCard(true)
    let pool = crate::engine::campfire_handler::nonempty_card_pool_for_class(
        run_state.player_class,
        rarity,
    );
    if pool.is_empty() {
        return match run_state.player_class {
            "Silent" => CardId::StrikeG,
            _ => CardId::Strike,
        };
    }
    let idx = run_state
        .rng_pool
        .card_rng
        .random_range(0, pool.len() as i32 - 1) as usize;
    pool[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardType;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::selection::DomainEvent;

    #[test]
    fn read_preserves_preview_obtain_upgrades_and_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.relics.push(RelicState::new(RelicId::MoltenEgg));
        rs.relics.push(RelicState::new(RelicId::ToxicEgg));
        rs.relics.push(RelicState::new(RelicId::FrozenEgg));
        rs.event_state = Some(EventState::new(EventId::TheLibrary));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let event_state = rs.event_state.as_ref().unwrap();
        assert_eq!(event_state.current_screen, 1);
        assert_eq!(library_entry_count(&event_state.extra_data), 20);
        assert_eq!(get_choices(&rs, event_state).len(), 20);

        for idx in 0..library_entry_count(&event_state.extra_data) {
            let (card_id, upgrades) =
                library_card_entry_at(&rs, &event_state.extra_data, idx).unwrap();
            let def = get_card_definition(card_id);
            assert!(matches!(
                def.card_type,
                CardType::Attack | CardType::Skill | CardType::Power
            ));
            assert_eq!(
                upgrades, 1,
                "Library must store each candidate after Java onPreviewObtainCard hooks"
            );
        }

        let (selected_id, selected_upgrades) =
            library_card_entry_at(&rs, &event_state.extra_data, 0).unwrap();
        handle_choice(&mut engine_state, &mut rs, 0);

        let obtained = rs.master_deck.last().unwrap();
        assert_eq!(obtained.id, selected_id);
        assert_eq!(
            obtained.upgrades, selected_upgrades,
            "selected preview-upgraded copy must not be upgraded again on obtain"
        );
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::TheLibrary),
            } if card.id == selected_id && card.upgrades == selected_upgrades
        )));
    }

    #[test]
    fn sleep_heals_through_player_heal_semantics_and_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.current_hp = 10;
        rs.max_hp = 80;
        rs.event_state = Some(EventState::new(EventId::TheLibrary));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(rs.current_hp, 36);
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 26,
                current_hp: 36,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::TheLibrary),
            }
        )));
    }

    #[test]
    fn sleep_is_blocked_by_mark_of_the_bloom_like_java_player_heal() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.current_hp = 10;
        rs.max_hp = 80;
        rs.relics.push(RelicState::new(RelicId::MarkOfTheBloom));
        rs.event_state = Some(EventState::new(EventId::TheLibrary));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(rs.current_hp, 10);
        assert!(!rs
            .take_emitted_events()
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }
}
