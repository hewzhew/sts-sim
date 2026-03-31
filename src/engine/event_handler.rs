use crate::state::core::EngineState;
use crate::state::run::RunState;
use crate::state::events::{EventChoiceMeta, EventId};

pub fn handle_event_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) -> Result<(), &'static str> {
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
        EventId::Cleric => crate::content::events::cleric::handle_choice(engine_state, run_state, choice_idx),
        EventId::GoldenShrine => crate::content::events::golden_shrine::handle_choice(engine_state, run_state, choice_idx),
        EventId::GoldenIdol => crate::content::events::golden_idol::handle_choice(engine_state, run_state, choice_idx),
        EventId::LivingWall => crate::content::events::living_wall::handle_choice(engine_state, run_state, choice_idx),
        EventId::Vampires => crate::content::events::vampires::handle_choice(engine_state, run_state, choice_idx),
        EventId::BigFish => crate::content::events::big_fish::handle_choice(engine_state, run_state, choice_idx),
        EventId::Ssssserpent => crate::content::events::sssserpent::handle_choice(engine_state, run_state, choice_idx),
        EventId::WorldOfGoop => crate::content::events::goop_puddle::handle_choice(engine_state, run_state, choice_idx),
        EventId::ShiningLight => crate::content::events::shining_light::handle_choice(engine_state, run_state, choice_idx),
        EventId::ScrapOoze => crate::content::events::scrap_ooze::handle_choice(engine_state, run_state, choice_idx),
        EventId::GoldenWing => crate::content::events::golden_wing::handle_choice(engine_state, run_state, choice_idx),
        EventId::Purifier => crate::content::events::purification_shrine::handle_choice(engine_state, run_state, choice_idx),
        EventId::UpgradeShrine => crate::content::events::upgrade_shrine::handle_choice(engine_state, run_state, choice_idx),
        EventId::Transmorgrifier => crate::content::events::transmogrifier::handle_choice(engine_state, run_state, choice_idx),
        EventId::Lab => crate::content::events::lab::handle_choice(engine_state, run_state, choice_idx),
        EventId::Duplicator => crate::content::events::duplicator::handle_choice(engine_state, run_state, choice_idx),
        EventId::Mushrooms => crate::content::events::mushrooms::handle_choice(engine_state, run_state, choice_idx),
        EventId::MaskedBandits => crate::content::events::masked_bandits::handle_choice(engine_state, run_state, choice_idx),
        EventId::MysteriousSphere => crate::content::events::mysterious_sphere::handle_choice(engine_state, run_state, choice_idx),
        EventId::Colosseum => crate::content::events::colosseum::handle_choice(engine_state, run_state, choice_idx),
        EventId::Addict => crate::content::events::addict::handle_choice(engine_state, run_state, choice_idx),
        EventId::KnowingSkull => crate::content::events::knowing_skull::handle_choice(engine_state, run_state, choice_idx),
        EventId::ForgottenAltar => crate::content::events::forgotten_altar::handle_choice(engine_state, run_state, choice_idx),
        EventId::Ghosts => crate::content::events::ghosts::handle_choice(engine_state, run_state, choice_idx),
        EventId::Nest => crate::content::events::nest::handle_choice(engine_state, run_state, choice_idx),
        EventId::Mausoleum => crate::content::events::mausoleum::handle_choice(engine_state, run_state, choice_idx),
        EventId::Falling => crate::content::events::falling::handle_choice(engine_state, run_state, choice_idx),
        EventId::MoaiHead => crate::content::events::moai_head::handle_choice(engine_state, run_state, choice_idx),
        EventId::TombRedMask => crate::content::events::tomb_red_mask::handle_choice(engine_state, run_state, choice_idx),
        EventId::WindingHalls => crate::content::events::winding_halls::handle_choice(engine_state, run_state, choice_idx),
        EventId::FaceTrader => crate::content::events::face_trader::handle_choice(engine_state, run_state, choice_idx),
        EventId::FountainOfCurseCleansing => crate::content::events::fountain::handle_choice(engine_state, run_state, choice_idx),
        EventId::Nloth => crate::content::events::nloth::handle_choice(engine_state, run_state, choice_idx),
        EventId::CursedTome => crate::content::events::cursed_tome::handle_choice(engine_state, run_state, choice_idx),
        EventId::WomanInBlue => crate::content::events::woman_in_blue::handle_choice(engine_state, run_state, choice_idx),
        EventId::BackTotheBasics => crate::content::events::back_to_basics::handle_choice(engine_state, run_state, choice_idx),
        EventId::Beggar => crate::content::events::beggar::handle_choice(engine_state, run_state, choice_idx),
        EventId::BonfireSpirits => crate::content::events::bonfire_spirits::handle_choice(engine_state, run_state, choice_idx),
        EventId::Designer => crate::content::events::designer::handle_choice(engine_state, run_state, choice_idx),
        EventId::TheLibrary => crate::content::events::the_library::handle_choice(engine_state, run_state, choice_idx),
        EventId::WeMeetAgain => crate::content::events::we_meet_again::handle_choice(engine_state, run_state, choice_idx),
        EventId::SensoryStone => crate::content::events::sensory_stone::handle_choice(engine_state, run_state, choice_idx),
        EventId::MindBloom => crate::content::events::mind_bloom::handle_choice(engine_state, run_state, choice_idx),
        EventId::DeadAdventurer => crate::content::events::dead_adventurer::handle_choice(engine_state, run_state, choice_idx),
        EventId::NoteForYourself => crate::content::events::note_for_yourself::handle_choice(engine_state, run_state, choice_idx),
        EventId::MatchAndKeep => crate::content::events::match_and_keep::handle_choice(engine_state, run_state, choice_idx),
        EventId::AccursedBlacksmith => crate::content::events::accursed_blacksmith::handle_choice(engine_state, run_state, choice_idx),
        EventId::BonfireElementals => crate::content::events::bonfire_elementals::handle_choice(engine_state, run_state, choice_idx),
        EventId::GremlinWheelGame => crate::content::events::gremlin_wheel::handle_choice(engine_state, run_state, choice_idx),
        EventId::DrugDealer => crate::content::events::drug_dealer::handle_choice(engine_state, run_state, choice_idx),
        EventId::TheJoust => crate::content::events::the_joust::handle_choice(engine_state, run_state, choice_idx),
        EventId::Neow => crate::content::events::neow::handle_choice(engine_state, run_state, choice_idx),
    }

    Ok(())
}

/// Provides the User Interface with the valid text prompts and exact visual rendering of this event's current screen
pub fn get_event_choices(run_state: &RunState) -> Vec<EventChoiceMeta> {
    if let Some(event_state) = &run_state.event_state {
        if event_state.completed {
            return vec![EventChoiceMeta::new("Leave.")];
        }

        match event_state.id {
            EventId::Cleric => crate::content::events::cleric::get_choices(run_state, event_state),
            EventId::GoldenShrine => crate::content::events::golden_shrine::get_choices(run_state, event_state),
            EventId::GoldenIdol => crate::content::events::golden_idol::get_choices(run_state, event_state),
            EventId::LivingWall => crate::content::events::living_wall::get_choices(run_state, event_state),
            EventId::Vampires => crate::content::events::vampires::get_choices(run_state, event_state),
            EventId::BigFish => crate::content::events::big_fish::get_choices(run_state, event_state),
            EventId::Ssssserpent => crate::content::events::sssserpent::get_choices(run_state, event_state),
            EventId::WorldOfGoop => crate::content::events::goop_puddle::get_choices(run_state, event_state),
            EventId::ShiningLight => crate::content::events::shining_light::get_choices(run_state, event_state),
            EventId::ScrapOoze => crate::content::events::scrap_ooze::get_choices(run_state, event_state),
            EventId::GoldenWing => crate::content::events::golden_wing::get_choices(run_state, event_state),
            EventId::Purifier => crate::content::events::purification_shrine::get_choices(run_state, event_state),
            EventId::UpgradeShrine => crate::content::events::upgrade_shrine::get_choices(run_state, event_state),
            EventId::Transmorgrifier => crate::content::events::transmogrifier::get_choices(run_state, event_state),
            EventId::Lab => crate::content::events::lab::get_choices(run_state, event_state),
            EventId::Duplicator => crate::content::events::duplicator::get_choices(run_state, event_state),
            EventId::Mushrooms => crate::content::events::mushrooms::get_choices(run_state, event_state),
            EventId::MaskedBandits => crate::content::events::masked_bandits::get_choices(run_state, event_state),
            EventId::MysteriousSphere => crate::content::events::mysterious_sphere::get_choices(run_state, event_state),
            EventId::Colosseum => crate::content::events::colosseum::get_choices(run_state, event_state),
            EventId::Addict => crate::content::events::addict::get_choices(run_state, event_state),
            EventId::KnowingSkull => crate::content::events::knowing_skull::get_choices(run_state, event_state),
            EventId::ForgottenAltar => crate::content::events::forgotten_altar::get_choices(run_state, event_state),
            EventId::Ghosts => crate::content::events::ghosts::get_choices(run_state, event_state),
            EventId::Nest => crate::content::events::nest::get_choices(run_state, event_state),
            EventId::Mausoleum => crate::content::events::mausoleum::get_choices(run_state, event_state),
            EventId::Falling => crate::content::events::falling::get_choices(run_state, event_state),
            EventId::MoaiHead => crate::content::events::moai_head::get_choices(run_state, event_state),
            EventId::TombRedMask => crate::content::events::tomb_red_mask::get_choices(run_state, event_state),
            EventId::WindingHalls => crate::content::events::winding_halls::get_choices(run_state, event_state),
            EventId::FaceTrader => crate::content::events::face_trader::get_choices(run_state, event_state),
            EventId::FountainOfCurseCleansing => crate::content::events::fountain::get_choices(run_state, event_state),
            EventId::Nloth => crate::content::events::nloth::get_choices(run_state, event_state),
            EventId::CursedTome => crate::content::events::cursed_tome::get_choices(run_state, event_state),
            EventId::WomanInBlue => crate::content::events::woman_in_blue::get_choices(run_state, event_state),
            EventId::BackTotheBasics => crate::content::events::back_to_basics::get_choices(run_state, event_state),
            EventId::Beggar => crate::content::events::beggar::get_choices(run_state, event_state),
            EventId::BonfireSpirits => crate::content::events::bonfire_spirits::get_choices(run_state, event_state),
            EventId::Designer => crate::content::events::designer::get_choices(run_state, event_state),
            EventId::TheLibrary => crate::content::events::the_library::get_choices(run_state, event_state),
            EventId::WeMeetAgain => crate::content::events::we_meet_again::get_choices(run_state, event_state),
            EventId::SensoryStone => crate::content::events::sensory_stone::get_choices(run_state, event_state),
            EventId::MindBloom => crate::content::events::mind_bloom::get_choices(run_state, event_state),
            EventId::DeadAdventurer => crate::content::events::dead_adventurer::get_choices(run_state, event_state),
            EventId::NoteForYourself => crate::content::events::note_for_yourself::get_choices(run_state, event_state),
            EventId::MatchAndKeep => crate::content::events::match_and_keep::get_choices(run_state, event_state),
            EventId::AccursedBlacksmith => crate::content::events::accursed_blacksmith::get_choices(run_state, event_state),
            EventId::BonfireElementals => crate::content::events::bonfire_elementals::get_choices(run_state, event_state),
            EventId::GremlinWheelGame => crate::content::events::gremlin_wheel::get_choices(run_state, event_state),
            EventId::DrugDealer => crate::content::events::drug_dealer::get_choices(run_state, event_state),
            EventId::TheJoust => crate::content::events::the_joust::get_choices(run_state, event_state),
            EventId::Neow => crate::content::events::neow::get_choices(run_state, event_state),
        }
    } else {
        vec![]
    }
}
