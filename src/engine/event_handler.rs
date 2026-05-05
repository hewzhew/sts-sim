use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventOption, EventState};
use crate::state::run::RunState;
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum LiveEventRebuildResult {
    Ready {
        event_id: EventId,
        current_screen: usize,
        event_state: EventState,
        options: Vec<EventOption>,
    },
    UnknownEventName {
        raw_event_name: Option<String>,
        current_screen: usize,
    },
    MissingSemanticsState {
        event_id: EventId,
        current_screen: usize,
        missing_keys: Vec<String>,
    },
    StateDecodeFailed {
        event_id: EventId,
        current_screen: usize,
    },
    UnsupportedEvent {
        event_id: EventId,
        current_screen: usize,
    },
    OptionCountMismatch {
        event_id: EventId,
        current_screen: usize,
        expected: usize,
        actual: usize,
    },
    DisabledMismatch {
        event_id: EventId,
        current_screen: usize,
        mismatched_indices: Vec<usize>,
    },
}

impl LiveEventRebuildResult {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    pub fn screen_semantics_incomplete(&self) -> bool {
        matches!(
            self,
            Self::MissingSemanticsState { .. }
                | Self::StateDecodeFailed { .. }
                | Self::OptionCountMismatch { .. }
                | Self::DisabledMismatch { .. }
        )
    }
}

pub fn event_id_from_name(raw: &str) -> Option<EventId> {
    match raw.trim() {
        "Big Fish" => Some(EventId::BigFish),
        "Cleric" => Some(EventId::Cleric),
        "Dead Adventurer" => Some(EventId::DeadAdventurer),
        "Golden Idol" => Some(EventId::GoldenIdol),
        "Living Wall" => Some(EventId::LivingWall),
        "Mushrooms" => Some(EventId::Mushrooms),
        "Scrap Ooze" => Some(EventId::ScrapOoze),
        "Shining Light" => Some(EventId::ShiningLight),
        "Ssssserpent" | "The Ssssserpent" | "Liars Game" => Some(EventId::Ssssserpent),
        "World of Goop" => Some(EventId::WorldOfGoop),
        "Golden Wing" | "Wing Statue" => Some(EventId::GoldenWing),
        "Match and Keep" => Some(EventId::MatchAndKeep),
        "Golden Shrine" => Some(EventId::GoldenShrine),
        "Addict" => Some(EventId::Addict),
        "Back to Basics" => Some(EventId::BackTotheBasics),
        "Beggar" => Some(EventId::Beggar),
        "Colosseum" => Some(EventId::Colosseum),
        "Ghosts" => Some(EventId::Ghosts),
        "Drug Dealer" => Some(EventId::DrugDealer),
        "Knowing Skull" => Some(EventId::KnowingSkull),
        "Masked Bandits" => Some(EventId::MaskedBandits),
        "Mausoleum" => Some(EventId::Mausoleum),
        "Nest" => Some(EventId::Nest),
        "Nloth" => Some(EventId::Nloth),
        "The Joust" => Some(EventId::TheJoust),
        "The Library" => Some(EventId::TheLibrary),
        "Vampires" => Some(EventId::Vampires),
        "Cursed Tome" => Some(EventId::CursedTome),
        "Winding Halls" => Some(EventId::WindingHalls),
        "Forgotten Altar" => Some(EventId::ForgottenAltar),
        "Mind Bloom" => Some(EventId::MindBloom),
        "Moai Head" => Some(EventId::MoaiHead),
        "Mysterious Sphere" => Some(EventId::MysteriousSphere),
        "Sensory Stone" => Some(EventId::SensoryStone),
        "Tomb Red Mask" => Some(EventId::TombRedMask),
        "Accursed Blacksmith" => Some(EventId::AccursedBlacksmith),
        "Bonfire Spirits" => Some(EventId::BonfireSpirits),
        "Bonfire Elementals" => Some(EventId::BonfireElementals),
        "Designer" => Some(EventId::Designer),
        "Duplicator" => Some(EventId::Duplicator),
        "Face Trader" => Some(EventId::FaceTrader),
        "Fountain of Curse Cleansing" => Some(EventId::FountainOfCurseCleansing),
        "Gremlin Wheel Game" => Some(EventId::GremlinWheelGame),
        "Lab" => Some(EventId::Lab),
        "Note For Yourself" => Some(EventId::NoteForYourself),
        "Purification Shrine" => Some(EventId::Purifier),
        "Transmogrifier" => Some(EventId::Transmorgrifier),
        "Upgrade Shrine" => Some(EventId::UpgradeShrine),
        "We Meet Again" => Some(EventId::WeMeetAgain),
        "Falling" => Some(EventId::Falling),
        "Woman in Blue" => Some(EventId::WomanInBlue),
        "Neow" | "Neow Event" => Some(EventId::Neow),
        other => match other.to_ascii_lowercase().as_str() {
            "world of goop" => Some(EventId::WorldOfGoop),
            "big fish" => Some(EventId::BigFish),
            "ghosts" => Some(EventId::Ghosts),
            "designer" => Some(EventId::Designer),
            _ => None,
        },
    }
}

fn live_event_raw_name(screen_state: &Value) -> Option<String> {
    screen_state
        .get("event_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            screen_state
                .get("event_id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
        })
        .map(ToString::to_string)
}

pub fn analyze_live_event_rebuild(
    run_state: &RunState,
    screen_state: &Value,
) -> LiveEventRebuildResult {
    let current_screen = screen_state
        .get("current_screen_index")
        .or_else(|| screen_state.get("current_screen"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let raw_event_name = live_event_raw_name(screen_state);
    let Some(event_id) = raw_event_name.as_deref().and_then(event_id_from_name) else {
        return LiveEventRebuildResult::UnknownEventName {
            raw_event_name,
            current_screen,
        };
    };

    let expected_keys = live_event_semantics_state_keys(event_id, current_screen)
        .map(|keys| keys.to_vec())
        .unwrap_or_default();
    let semantics_state = screen_state
        .get("event_semantics_state")
        .unwrap_or(&Value::Null);
    if live_event_requires_semantics_state(event_id, current_screen) {
        let missing_keys = expected_keys
            .iter()
            .filter(|key| semantics_state.get(**key).is_none())
            .map(|key| (*key).to_string())
            .collect::<Vec<_>>();
        if !missing_keys.is_empty() {
            return LiveEventRebuildResult::MissingSemanticsState {
                event_id,
                current_screen,
                missing_keys,
            };
        }
    }

    let Some(event_state) =
        try_build_event_state_from_screen_state(run_state, event_id, current_screen, screen_state)
    else {
        return LiveEventRebuildResult::StateDecodeFailed {
            event_id,
            current_screen,
        };
    };
    let Some(options) = try_get_structured_event_options_for_state(run_state, &event_state) else {
        return LiveEventRebuildResult::UnsupportedEvent {
            event_id,
            current_screen,
        };
    };

    if let Some(live_options) = screen_state.get("options").and_then(Value::as_array) {
        if options.len() != live_options.len() {
            return LiveEventRebuildResult::OptionCountMismatch {
                event_id,
                current_screen,
                expected: options.len(),
                actual: live_options.len(),
            };
        }

        let mismatched_indices = options
            .iter()
            .zip(live_options.iter())
            .enumerate()
            .filter_map(|(index, (option, live_option))| {
                (option.ui.disabled
                    != live_option
                        .get("disabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false))
                .then_some(index)
            })
            .collect::<Vec<_>>();
        if !mismatched_indices.is_empty() {
            return LiveEventRebuildResult::DisabledMismatch {
                event_id,
                current_screen,
                mismatched_indices,
            };
        }
    }

    LiveEventRebuildResult::Ready {
        event_id,
        current_screen,
        event_state,
        options,
    }
}

pub fn live_event_protocol_audit(run_state: &RunState, screen_state: &Value) -> Value {
    let rebuild = analyze_live_event_rebuild(run_state, screen_state);
    let current_screen = screen_state
        .get("current_screen_index")
        .or_else(|| screen_state.get("current_screen"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let event_id = live_event_raw_name(screen_state)
        .as_deref()
        .and_then(event_id_from_name);
    let expected_keys = event_id
        .and_then(|event_id| live_event_semantics_state_keys(event_id, current_screen))
        .map(|keys| keys.iter().map(|key| key.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let semantics_state = screen_state
        .get("event_semantics_state")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    let semantics_present = !semantics_state.is_null();

    let (status, missing_keys, option_count_match, disabled_match, detail) = match &rebuild {
        LiveEventRebuildResult::Ready { .. } => ("ready", Vec::new(), true, true, Value::Null),
        LiveEventRebuildResult::UnknownEventName { raw_event_name, .. } => (
            "unknown_event_name",
            Vec::new(),
            false,
            false,
            json!({ "raw_event_name": raw_event_name }),
        ),
        LiveEventRebuildResult::MissingSemanticsState { missing_keys, .. } => (
            "missing_semantics_state",
            missing_keys.clone(),
            false,
            false,
            Value::Null,
        ),
        LiveEventRebuildResult::StateDecodeFailed { .. } => {
            ("state_decode_failed", Vec::new(), false, false, Value::Null)
        }
        LiveEventRebuildResult::UnsupportedEvent { .. } => {
            ("unsupported_event", Vec::new(), false, false, Value::Null)
        }
        LiveEventRebuildResult::OptionCountMismatch {
            expected, actual, ..
        } => (
            "option_count_mismatch",
            Vec::new(),
            false,
            false,
            json!({ "expected": expected, "actual": actual }),
        ),
        LiveEventRebuildResult::DisabledMismatch {
            mismatched_indices, ..
        } => (
            "disabled_mismatch",
            Vec::new(),
            true,
            false,
            json!({ "mismatched_indices": mismatched_indices }),
        ),
    };

    json!({
        "event_id": event_id.map(|id| format!("{:?}", id)),
        "current_screen": current_screen,
        "rebuild_status": status,
        "rebuild_detail": detail,
        "screen_semantics_incomplete": rebuild.screen_semantics_incomplete(),
        "event_semantics_required": event_id
            .map(|event_id| live_event_requires_semantics_state(event_id, current_screen))
            .unwrap_or(false),
        "event_semantics_present": semantics_present,
        "event_semantics_keys": expected_keys,
        "event_semantics_missing_keys": missing_keys,
        "structured_live_ready": rebuild.is_ready(),
        "option_count_match": option_count_match,
        "disabled_match": disabled_match,
    })
}

pub fn handle_event_choice(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    choice_idx: usize,
) -> Result<(), &'static str> {
    let event_state = match &mut run_state.event_state {
        Some(e) => e,
        None => return Err("Not currently in an Event room."),
    };

    // Fast-path resolution if the event is completed. Usually clicking any valid choice when completed leaves the room.
    if event_state.completed {
        run_state.event_state = None;
        // The RunLoop needs to transition us back to the Map.
        *engine_state = crate::state::core::EngineState::MapNavigation;
        return Ok(());
    }

    // We dispatch based on the exact Event ID
    match event_state.id {
        EventId::Cleric => {
            crate::content::events::cleric::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::GoldenShrine => crate::content::events::golden_shrine::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::GoldenIdol => {
            crate::content::events::golden_idol::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::LivingWall => {
            crate::content::events::living_wall::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Vampires => {
            crate::content::events::vampires::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::BigFish => {
            crate::content::events::big_fish::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Ssssserpent => {
            crate::content::events::sssserpent::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::WorldOfGoop => {
            crate::content::events::goop_puddle::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::ShiningLight => crate::content::events::shining_light::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::ScrapOoze => {
            crate::content::events::scrap_ooze::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::GoldenWing => {
            crate::content::events::golden_wing::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Purifier => crate::content::events::purification_shrine::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::UpgradeShrine => crate::content::events::upgrade_shrine::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Transmorgrifier => crate::content::events::transmogrifier::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Lab => {
            crate::content::events::lab::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Duplicator => {
            crate::content::events::duplicator::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Mushrooms => {
            crate::content::events::mushrooms::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::MaskedBandits => crate::content::events::masked_bandits::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::MysteriousSphere => crate::content::events::mysterious_sphere::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Colosseum => {
            crate::content::events::colosseum::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Addict => {
            crate::content::events::addict::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::KnowingSkull => crate::content::events::knowing_skull::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::ForgottenAltar => crate::content::events::forgotten_altar::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Ghosts => {
            crate::content::events::ghosts::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Nest => {
            crate::content::events::nest::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Mausoleum => {
            crate::content::events::mausoleum::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Falling => {
            crate::content::events::falling::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::MoaiHead => {
            crate::content::events::moai_head::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::TombRedMask => crate::content::events::tomb_red_mask::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::WindingHalls => crate::content::events::winding_halls::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::FaceTrader => {
            crate::content::events::face_trader::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::FountainOfCurseCleansing => {
            crate::content::events::fountain::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Nloth => {
            crate::content::events::nloth::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::CursedTome => {
            crate::content::events::cursed_tome::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::WomanInBlue => crate::content::events::woman_in_blue::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::BackTotheBasics => crate::content::events::back_to_basics::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Beggar => {
            crate::content::events::beggar::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::BonfireSpirits => crate::content::events::bonfire_spirits::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::Designer => {
            crate::content::events::designer::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::TheLibrary => {
            crate::content::events::the_library::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::WeMeetAgain => crate::content::events::we_meet_again::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::SensoryStone => crate::content::events::sensory_stone::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::MindBloom => {
            crate::content::events::mind_bloom::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::DeadAdventurer => crate::content::events::dead_adventurer::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::NoteForYourself => crate::content::events::note_for_yourself::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::MatchAndKeep => crate::content::events::match_and_keep::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::AccursedBlacksmith => crate::content::events::accursed_blacksmith::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::BonfireElementals => crate::content::events::bonfire_elementals::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::GremlinWheelGame => crate::content::events::gremlin_wheel::handle_choice(
            engine_state,
            run_state,
            choice_idx,
        ),
        EventId::DrugDealer => {
            crate::content::events::drug_dealer::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::TheJoust => {
            crate::content::events::the_joust::handle_choice(engine_state, run_state, choice_idx)
        }
        EventId::Neow => {
            crate::content::events::neow::handle_choice(engine_state, run_state, choice_idx)
        }
    }

    Ok(())
}

pub fn get_event_options(run_state: &RunState) -> Vec<EventOption> {
    if let Some(event_state) = &run_state.event_state {
        if event_state.completed {
            return vec![EventOption::unknown(EventChoiceMeta::new("Leave."))];
        }

        return try_get_structured_event_options_for_state(run_state, event_state).unwrap_or_else(
            || {
                get_event_choices(run_state)
                    .into_iter()
                    .map(EventOption::unknown)
                    .collect()
            },
        );
    }

    Vec::new()
}

pub fn try_get_structured_event_options_for_state(
    run_state: &RunState,
    event_state: &EventState,
) -> Option<Vec<EventOption>> {
    Some(match event_state.id {
        EventId::GoldenIdol => {
            crate::content::events::golden_idol::get_options(run_state, event_state)
        }
        EventId::Ghosts => crate::content::events::ghosts::get_options(run_state, event_state),
        EventId::Vampires => crate::content::events::vampires::get_options(run_state, event_state),
        EventId::CursedTome => {
            crate::content::events::cursed_tome::get_options(run_state, event_state)
        }
        EventId::WindingHalls => {
            crate::content::events::winding_halls::get_options(run_state, event_state)
        }
        EventId::ForgottenAltar => {
            crate::content::events::forgotten_altar::get_options(run_state, event_state)
        }
        EventId::WeMeetAgain => {
            crate::content::events::we_meet_again::get_options(run_state, event_state)
        }
        EventId::Falling => crate::content::events::falling::get_options(run_state, event_state),
        EventId::MindBloom => {
            crate::content::events::mind_bloom::get_options(run_state, event_state)
        }
        EventId::Designer => crate::content::events::designer::get_options(run_state, event_state),
        EventId::WomanInBlue => {
            crate::content::events::woman_in_blue::get_options(run_state, event_state)
        }
        EventId::Cleric => crate::content::events::cleric::get_options(run_state, event_state),
        _ => return None,
    })
}

pub fn try_build_event_state_from_screen_state(
    run_state: &RunState,
    event_id: EventId,
    current_screen: usize,
    screen_state: &Value,
) -> Option<EventState> {
    let internal_state = match event_id {
        EventId::Designer if current_screen == 1 => {
            decode_designer_internal_state(screen_state.get("event_semantics_state")?)?
        }
        EventId::WeMeetAgain if current_screen == 0 => decode_we_meet_again_internal_state(
            run_state,
            screen_state.get("event_semantics_state")?,
        )?,
        EventId::Falling if current_screen == 1 => {
            decode_falling_internal_state(run_state, screen_state.get("event_semantics_state")?)?
        }
        _ => 0,
    };
    Some(EventState {
        id: event_id,
        current_screen,
        internal_state,
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    })
}

pub fn event_semantics_state(run_state: &RunState, event_state: &EventState) -> Option<Value> {
    match event_state.id {
        EventId::Designer if event_state.current_screen == 1 => Some(json!({
            "adjust_upgrades_one": event_state.internal_state & 1 != 0,
            "clean_up_removes_cards": event_state.internal_state & 2 != 0,
        })),
        EventId::WeMeetAgain if event_state.current_screen == 0 => {
            let potion_slot = ((event_state.internal_state >> 16) & 0xFF) as usize;
            let gold_amount = event_state.internal_state & 0xFF;
            let card_idx = ((event_state.internal_state >> 8) & 0xFF) as usize;
            Some(json!({
                "potion_slot": if potion_slot == 0xFF { None } else { Some(potion_slot) },
                "gold_amount": gold_amount,
                "card_uuid": if card_idx == 0xFF {
                    None
                } else {
                    run_state.master_deck.get(card_idx).map(|card| card.uuid)
                },
            }))
        }
        EventId::Falling if event_state.current_screen == 1 => {
            let skill_idx = (event_state.internal_state & 0x3FF) as usize;
            let power_idx = ((event_state.internal_state >> 10) & 0x3FF) as usize;
            let attack_idx = ((event_state.internal_state >> 20) & 0x3FF) as usize;
            Some(json!({
                "skill_uuid": if skill_idx == 0x3FF { None } else { run_state.master_deck.get(skill_idx).map(|card| card.uuid) },
                "power_uuid": if power_idx == 0x3FF { None } else { run_state.master_deck.get(power_idx).map(|card| card.uuid) },
                "attack_uuid": if attack_idx == 0x3FF { None } else { run_state.master_deck.get(attack_idx).map(|card| card.uuid) },
            }))
        }
        _ => None,
    }
}

pub fn live_event_requires_semantics_state(event_id: EventId, current_screen: usize) -> bool {
    matches!(
        (event_id, current_screen),
        (EventId::Designer, 1) | (EventId::WeMeetAgain, 0) | (EventId::Falling, 1)
    )
}

pub fn live_event_semantics_state_keys(
    event_id: EventId,
    current_screen: usize,
) -> Option<&'static [&'static str]> {
    match (event_id, current_screen) {
        (EventId::Designer, 1) => Some(&["adjust_upgrades_one", "clean_up_removes_cards"]),
        (EventId::WeMeetAgain, 0) => Some(&["potion_slot", "gold_amount", "card_uuid"]),
        (EventId::Falling, 1) => Some(&["skill_uuid", "power_uuid", "attack_uuid"]),
        _ => None,
    }
}

fn decode_designer_internal_state(event_semantics_state: &Value) -> Option<i32> {
    let adjust_upgrades_one = event_semantics_state
        .get("adjust_upgrades_one")
        .and_then(Value::as_bool)? as i32;
    let clean_up_removes_cards = event_semantics_state
        .get("clean_up_removes_cards")
        .and_then(Value::as_bool)? as i32;
    Some(adjust_upgrades_one | (clean_up_removes_cards << 1))
}

fn decode_we_meet_again_internal_state(
    run_state: &RunState,
    event_semantics_state: &Value,
) -> Option<i32> {
    let potion_slot = event_semantics_state
        .get("potion_slot")
        .and_then(|value| {
            if value.is_null() {
                Some(0xFFusize)
            } else {
                value.as_u64().map(|slot| slot as usize)
            }
        })
        .unwrap_or(0xFF);
    let gold_amount = event_semantics_state
        .get("gold_amount")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .clamp(0, 255) as i32;
    let card_uuid = event_semantics_state
        .get("card_uuid")
        .and_then(|value| {
            if value.is_null() {
                None
            } else {
                value.as_u64()
            }
        })
        .map(|uuid| uuid as u32);
    let card_idx = card_uuid
        .and_then(|uuid| {
            run_state
                .master_deck
                .iter()
                .position(|card| card.uuid == uuid)
                .map(|idx| idx as i32)
        })
        .unwrap_or(0xFF);
    Some(gold_amount | (card_idx << 8) | ((potion_slot as i32) << 16))
}

fn decode_falling_internal_state(
    run_state: &RunState,
    event_semantics_state: &Value,
) -> Option<i32> {
    fn idx_for_uuid(run_state: &RunState, value: Option<&Value>) -> i32 {
        let Some(uuid) = value
            .and_then(|value| {
                if value.is_null() {
                    None
                } else {
                    value.as_u64()
                }
            })
            .map(|uuid| uuid as u32)
        else {
            return 0x3FF;
        };
        run_state
            .master_deck
            .iter()
            .position(|card| card.uuid == uuid)
            .map(|idx| idx as i32)
            .unwrap_or(0x3FF)
    }

    let skill_idx = idx_for_uuid(run_state, event_semantics_state.get("skill_uuid"));
    let power_idx = idx_for_uuid(run_state, event_semantics_state.get("power_uuid"));
    let attack_idx = idx_for_uuid(run_state, event_semantics_state.get("attack_uuid"));
    Some((skill_idx & 0x3FF) | ((power_idx & 0x3FF) << 10) | ((attack_idx & 0x3FF) << 20))
}

/// Provides the User Interface with the valid text prompts and exact visual rendering of this event's current screen
pub fn get_event_choices(run_state: &RunState) -> Vec<EventChoiceMeta> {
    if let Some(event_state) = &run_state.event_state {
        if event_state.completed {
            return vec![EventChoiceMeta::new("Leave.")];
        }

        match event_state.id {
            EventId::Cleric => crate::content::events::cleric::get_choices(run_state, event_state),
            EventId::GoldenShrine => {
                crate::content::events::golden_shrine::get_choices(run_state, event_state)
            }
            EventId::GoldenIdol => {
                crate::content::events::golden_idol::get_choices(run_state, event_state)
            }
            EventId::LivingWall => {
                crate::content::events::living_wall::get_choices(run_state, event_state)
            }
            EventId::Vampires => {
                crate::content::events::vampires::get_choices(run_state, event_state)
            }
            EventId::BigFish => {
                crate::content::events::big_fish::get_choices(run_state, event_state)
            }
            EventId::Ssssserpent => {
                crate::content::events::sssserpent::get_choices(run_state, event_state)
            }
            EventId::WorldOfGoop => {
                crate::content::events::goop_puddle::get_choices(run_state, event_state)
            }
            EventId::ShiningLight => {
                crate::content::events::shining_light::get_choices(run_state, event_state)
            }
            EventId::ScrapOoze => {
                crate::content::events::scrap_ooze::get_choices(run_state, event_state)
            }
            EventId::GoldenWing => {
                crate::content::events::golden_wing::get_choices(run_state, event_state)
            }
            EventId::Purifier => {
                crate::content::events::purification_shrine::get_choices(run_state, event_state)
            }
            EventId::UpgradeShrine => {
                crate::content::events::upgrade_shrine::get_choices(run_state, event_state)
            }
            EventId::Transmorgrifier => {
                crate::content::events::transmogrifier::get_choices(run_state, event_state)
            }
            EventId::Lab => crate::content::events::lab::get_choices(run_state, event_state),
            EventId::Duplicator => {
                crate::content::events::duplicator::get_choices(run_state, event_state)
            }
            EventId::Mushrooms => {
                crate::content::events::mushrooms::get_choices(run_state, event_state)
            }
            EventId::MaskedBandits => {
                crate::content::events::masked_bandits::get_choices(run_state, event_state)
            }
            EventId::MysteriousSphere => {
                crate::content::events::mysterious_sphere::get_choices(run_state, event_state)
            }
            EventId::Colosseum => {
                crate::content::events::colosseum::get_choices(run_state, event_state)
            }
            EventId::Addict => crate::content::events::addict::get_choices(run_state, event_state),
            EventId::KnowingSkull => {
                crate::content::events::knowing_skull::get_choices(run_state, event_state)
            }
            EventId::ForgottenAltar => {
                crate::content::events::forgotten_altar::get_choices(run_state, event_state)
            }
            EventId::Ghosts => crate::content::events::ghosts::get_choices(run_state, event_state),
            EventId::Nest => crate::content::events::nest::get_choices(run_state, event_state),
            EventId::Mausoleum => {
                crate::content::events::mausoleum::get_choices(run_state, event_state)
            }
            EventId::Falling => {
                crate::content::events::falling::get_choices(run_state, event_state)
            }
            EventId::MoaiHead => {
                crate::content::events::moai_head::get_choices(run_state, event_state)
            }
            EventId::TombRedMask => {
                crate::content::events::tomb_red_mask::get_choices(run_state, event_state)
            }
            EventId::WindingHalls => {
                crate::content::events::winding_halls::get_choices(run_state, event_state)
            }
            EventId::FaceTrader => {
                crate::content::events::face_trader::get_choices(run_state, event_state)
            }
            EventId::FountainOfCurseCleansing => {
                crate::content::events::fountain::get_choices(run_state, event_state)
            }
            EventId::Nloth => crate::content::events::nloth::get_choices(run_state, event_state),
            EventId::CursedTome => {
                crate::content::events::cursed_tome::get_choices(run_state, event_state)
            }
            EventId::WomanInBlue => {
                crate::content::events::woman_in_blue::get_choices(run_state, event_state)
            }
            EventId::BackTotheBasics => {
                crate::content::events::back_to_basics::get_choices(run_state, event_state)
            }
            EventId::Beggar => crate::content::events::beggar::get_choices(run_state, event_state),
            EventId::BonfireSpirits => {
                crate::content::events::bonfire_spirits::get_choices(run_state, event_state)
            }
            EventId::Designer => {
                crate::content::events::designer::get_choices(run_state, event_state)
            }
            EventId::TheLibrary => {
                crate::content::events::the_library::get_choices(run_state, event_state)
            }
            EventId::WeMeetAgain => {
                crate::content::events::we_meet_again::get_choices(run_state, event_state)
            }
            EventId::SensoryStone => {
                crate::content::events::sensory_stone::get_choices(run_state, event_state)
            }
            EventId::MindBloom => {
                crate::content::events::mind_bloom::get_choices(run_state, event_state)
            }
            EventId::DeadAdventurer => {
                crate::content::events::dead_adventurer::get_choices(run_state, event_state)
            }
            EventId::NoteForYourself => {
                crate::content::events::note_for_yourself::get_choices(run_state, event_state)
            }
            EventId::MatchAndKeep => {
                crate::content::events::match_and_keep::get_choices(run_state, event_state)
            }
            EventId::AccursedBlacksmith => {
                crate::content::events::accursed_blacksmith::get_choices(run_state, event_state)
            }
            EventId::BonfireElementals => {
                crate::content::events::bonfire_elementals::get_choices(run_state, event_state)
            }
            EventId::GremlinWheelGame => {
                crate::content::events::gremlin_wheel::get_choices(run_state, event_state)
            }
            EventId::DrugDealer => {
                crate::content::events::drug_dealer::get_choices(run_state, event_state)
            }
            EventId::TheJoust => {
                crate::content::events::the_joust::get_choices(run_state, event_state)
            }
            EventId::Neow => crate::content::events::neow::get_choices(run_state, event_state),
        }
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use serde_json::json;

    #[test]
    fn event_id_from_name_accepts_live_aliases() {
        assert_eq!(event_id_from_name("Neow Event"), Some(EventId::Neow));
        assert_eq!(event_id_from_name("Liars Game"), Some(EventId::Ssssserpent));
        assert_eq!(event_id_from_name("Wing Statue"), Some(EventId::GoldenWing));
    }

    #[test]
    fn designer_event_semantics_state_round_trips_to_internal_state() {
        let rs = RunState::new(1, 0, true, "Ironclad");
        let event_state = EventState {
            id: EventId::Designer,
            current_screen: 1,
            internal_state: 0b11,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };

        let semantics = event_semantics_state(&rs, &event_state).unwrap();
        let screen_state = json!({
            "event_name": "Designer",
            "current_screen": 1,
            "event_semantics_state": semantics,
        });
        let rebuilt =
            try_build_event_state_from_screen_state(&rs, EventId::Designer, 1, &screen_state)
                .unwrap();

        assert_eq!(rebuilt.internal_state, event_state.internal_state);
    }

    #[test]
    fn we_meet_again_event_semantics_state_round_trips_to_internal_state() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.potions[0] = Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::StrengthPotion,
            90_001,
        ));
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::PommelStrike,
            90_101,
        ));
        let card_idx = rs.master_deck.len() - 1;
        let event_state = EventState {
            id: EventId::WeMeetAgain,
            current_screen: 0,
            internal_state: 75 | ((card_idx as i32) << 8),
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };

        let semantics = event_semantics_state(&rs, &event_state).unwrap();
        let screen_state = json!({
            "event_name": "We Meet Again",
            "current_screen": 0,
            "event_semantics_state": semantics,
        });
        let rebuilt =
            try_build_event_state_from_screen_state(&rs, EventId::WeMeetAgain, 0, &screen_state)
                .unwrap();

        assert_eq!(rebuilt.internal_state, event_state.internal_state);
    }

    #[test]
    fn falling_event_semantics_state_round_trips_to_internal_state() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.add_card_to_deck(CardId::ShrugItOff);
        let skill_idx = rs.master_deck.len() - 1;
        rs.add_card_to_deck(CardId::Inflame);
        let power_idx = rs.master_deck.len() - 1;
        let attack_idx = rs
            .master_deck
            .iter()
            .position(|card| card.id == CardId::Strike)
            .unwrap();
        let event_state = EventState {
            id: EventId::Falling,
            current_screen: 1,
            internal_state: (skill_idx as i32 & 0x3FF)
                | (((power_idx as i32) & 0x3FF) << 10)
                | (((attack_idx as i32) & 0x3FF) << 20),
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };

        let semantics = event_semantics_state(&rs, &event_state).unwrap();
        let screen_state = json!({
            "event_name": "Falling",
            "current_screen": 1,
            "event_semantics_state": semantics,
        });
        let rebuilt =
            try_build_event_state_from_screen_state(&rs, EventId::Falling, 1, &screen_state)
                .unwrap();

        assert_eq!(rebuilt.internal_state, event_state.internal_state);
    }
}
