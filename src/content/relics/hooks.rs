use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::content::relics::RelicId;
use smallvec::SmallVec;

/// Triggers relics before the battle formally begins (Phase 1).
pub fn at_pre_battle(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.at_pre_battle.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::AncientTeaSet => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(
                    crate::content::relics::ancient_tea_set::AncientTeaSet::at_pre_battle(&mut rs),
                );
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::ArtOfWar => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::art_of_war::at_pre_battle(&mut rs));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::CentennialPuzzle => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(
                    crate::content::relics::centennial_puzzle::CentennialPuzzle::at_pre_battle(
                        &mut rs,
                    ),
                );
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::CrackedCore => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::cracked_core::at_battle_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::Enchiridion => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::enchiridion::at_battle_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::Lantern => actions.extend(crate::content::relics::lantern::at_battle_start()),
            RelicId::NuclearBattery => {
                actions.extend(crate::content::relics::nuclear_battery::at_battle_start())
            }
            RelicId::RunicCapacitor => {
                actions.extend(crate::content::relics::runic_capacitor::at_battle_start())
            }
            RelicId::SneckoEye => {
                actions.extend(crate::content::relics::snecko_eye::at_battle_start())
            }
            RelicId::SymbioticVirus => {
                actions.extend(crate::content::relics::symbiotic_virus::at_battle_start())
            }
            _ => unreachable!(
                "Relic {} present in at_pre_battle bus but unhandled in hooks.rs match arm",
                relic_id as usize
            ),
        }
    }
    actions
}

/// Triggers relics at the start of battle, before initial card draw (Phase 2).
pub fn at_battle_start_pre_draw(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state
        .entities
        .player
        .relic_buses
        .at_battle_start_pre_draw
        .clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::HolyWater => actions.extend(crate::content::relics::holy_water::at_battle_start(&*state)),
            RelicId::NinjaScroll => actions.extend(crate::content::relics::ninja_scroll::at_battle_start()),
            RelicId::PureWater => actions.extend(crate::content::relics::pure_water::at_battle_start()),
            RelicId::Toolbox => actions.extend(crate::content::relics::toolbox::at_battle_start()),
            RelicId::GamblingChip => {} // Handled elsewhere or TODO
            _ => unreachable!("Relic {} present in at_battle_start_pre_draw bus but unhandled in hooks.rs match arm", relic_id as usize),
        }
    }
    actions
}

/// Triggers relics at the start of battle (Phase 3).
/// Takes &mut CombatState so relics can directly mutate state (e.g. SlaversCollar ++energy_master).
pub fn at_battle_start(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.at_battle_start.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::Akabeko => {
                actions.extend(crate::content::relics::akabeko::Akabeko::at_battle_start())
            }
            RelicId::Anchor => {
                actions.extend(crate::content::relics::anchor::Anchor::at_battle_start())
            }
            RelicId::BagOfMarbles => actions.extend(
                crate::content::relics::bag_of_marbles::BagOfMarbles::at_battle_start(&*state),
            ),
            RelicId::BagOfPreparation => actions.extend(
                crate::content::relics::bag_of_preparation::BagOfPreparation::at_battle_start(),
            ),
            RelicId::BloodVial => {
                actions.extend(crate::content::relics::blood_vial::BloodVial::at_battle_start())
            }
            RelicId::BronzeScales => actions.extend(
                crate::content::relics::bronze_scales::BronzeScales::at_battle_start(
                    state.entities.player.id,
                ),
            ),
            RelicId::ClockworkSouvenir => actions.extend(
                crate::content::relics::clockwork_souvenir::ClockworkSouvenir::at_battle_start(),
            ),
            RelicId::CultistMask => {}
            RelicId::Dodecahedron => actions.extend(
                crate::content::relics::dodecahedron::Dodecahedron::at_battle_start(&*state),
            ),
            RelicId::FossilizedHelix => actions.extend(
                crate::content::relics::fossilized_helix::FossilizedHelix::at_battle_start(),
            ),
            RelicId::Girya => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::girya::Girya::at_battle_start(
                    counter,
                ));
            }
            RelicId::HornCleat => {
                actions.extend(crate::content::relics::horn_cleat::HornCleat::at_battle_start())
            }
            RelicId::Mango => actions.extend(crate::content::relics::mango::at_battle_start()),
            RelicId::OddlySmoothStone => {
                actions.extend(crate::content::relics::oddly_smooth_stone::at_battle_start())
            }
            RelicId::Pantograph => {
                actions.extend(crate::content::relics::pantograph::at_battle_start(&*state))
            }
            RelicId::Pocketwatch => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                crate::content::relics::pocketwatch::at_battle_start(&mut rs);
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::PreservedInsect => actions.extend(
                crate::content::relics::preserved_insect::at_battle_start(&*state),
            ),
            RelicId::DataDisk => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::data_disk::at_battle_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::DuVuDoll => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::du_vu_doll::at_battle_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::GremlinMask => {
                actions.extend(crate::content::relics::gremlin_mask::at_battle_start(
                    &*state,
                    &state.entities.player,
                ))
            }
            RelicId::SnakeRing => {
                actions.extend(crate::content::relics::snake_ring::at_battle_start())
            }
            RelicId::Vajra => actions.extend(crate::content::relics::vajra::at_battle_start()),
            RelicId::RedMask => {
                actions.extend(crate::content::relics::red_mask::at_battle_start(&*state))
            }
            RelicId::PenNib => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::pen_nib::at_battle_start(counter));
            }
            // P1 relics
            RelicId::PhilosopherStone => actions.extend(
                crate::content::relics::philosopher_stone::at_battle_start(&*state),
            ),
            RelicId::MarkOfPain => {
                actions.extend(crate::content::relics::mark_of_pain::at_battle_start())
            }
            RelicId::ThreadAndNeedle => {
                actions.extend(crate::content::relics::thread_and_needle::at_battle_start())
            }
            RelicId::MutagenicStrength => {
                actions.extend(crate::content::relics::mutagenic_strength::at_battle_start())
            }
            RelicId::NeowsLament => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::neows_lament::at_battle_start(
                    &*state, counter,
                ));
            }
            RelicId::TwistedFunnel => actions.extend(
                crate::content::relics::twisted_funnel::at_battle_start(&*state),
            ),
            RelicId::Sling => actions.extend(crate::content::relics::sling::at_battle_start()),
            RelicId::RedSkull => {
                let (hp, max_hp) = (
                    state.entities.player.current_hp,
                    state.entities.player.max_hp,
                );
                actions.extend(crate::content::relics::red_skull::at_battle_start(
                    hp, max_hp,
                ));
            }
            RelicId::SlaversCollar => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::slavers_collar::at_battle_start(
                    state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::TeardropLocket => {
                actions.extend(crate::content::relics::teardrop_locket::at_battle_start())
            }
            _ => unreachable!(
                "Relic {} present in at_battle_start bus but unhandled in hooks.rs match arm",
                relic_id as usize
            ),
        }
    }
    actions
}

/// Triggers relics when the draw pile is shuffled.
pub fn on_shuffle(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.on_shuffle.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::Abacus => actions.extend(crate::content::relics::abacus::Abacus::on_shuffle()),
            RelicId::Melange => actions.extend(crate::content::relics::melange::on_shuffle()),
            RelicId::Sundial => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::sundial::on_shuffle(counter));
            }
            _ => {
                unreachable!("Relic present in on_shuffle bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

pub fn on_spawn_monster(state: &CombatState, target_idx: usize) -> SmallVec<[Action; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.on_spawn_monster.clone();
    let target_id = state.entities.monsters[target_idx].id;
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::PhilosopherStone => actions.push(Action::ApplyPower {
                source: target_id,
                target: target_id,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 1,
            }),
            _ => unreachable!(
                "Relic {} present in on_spawn_monster bus but unhandled in hooks.rs match arm",
                relic_id as usize
            ),
        }
    }
    actions
}

pub fn on_exhaust(state: &mut CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_exhaust.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::CharonsAshes => actions
                .extend(crate::content::relics::charons_ashes::CharonsAshes::on_exhaust(&*state)),
            RelicId::DeadBranch => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::dead_branch::on_exhaust(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            _ => {
                unreachable!("Relic present in on_exhaust bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

pub fn on_lose_hp(state: &mut CombatState, amount: i32) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_lose_hp.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::CentennialPuzzle => {
                let used_up = state.entities.player.relics[relic_index].used_up;
                actions.extend(
                    crate::content::relics::centennial_puzzle::CentennialPuzzle::on_lose_hp(
                        used_up,
                    ),
                );
            }
            RelicId::EmotionChip => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::emotion_chip::on_lose_hp(
                    &*state, &mut rs, amount,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::LizardTail => {
                let used_up = state.entities.player.relics[relic_index].used_up;
                actions.extend(crate::content::relics::lizard_tail::on_lose_hp(
                    &*state, used_up,
                ));
            }
            RelicId::SelfFormingClay => {
                actions.extend(crate::content::relics::self_forming_clay::on_lose_hp())
            }
            RelicId::TungstenRod => {
                actions.extend(crate::content::relics::tungsten_rod::on_lose_hp(amount))
            }
            RelicId::RunicCube => {
                actions.extend(crate::content::relics::runic_cube::was_hp_lost(amount))
            }
            _ => {
                unreachable!("Relic present in on_lose_hp bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

pub fn on_victory(state: &mut CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_victory.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::BurningBlood => {
                actions.extend(crate::content::relics::burning_blood::BurningBlood::on_victory())
            }
            RelicId::DarkBlood => {
                actions.extend(crate::content::relics::dark_blood::DarkBlood::on_victory())
            }
            RelicId::BlackBlood => {
                actions.extend(crate::content::relics::black_blood::BlackBlood::on_victory())
            }
            RelicId::BlackStar => {
                actions.extend(crate::content::relics::black_star::BlackStar::on_victory())
            }
            RelicId::FaceOfCleric => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::face_of_cleric::on_victory(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::MeatOnTheBone => {
                let used_up = state.entities.player.relics[relic_index].used_up;
                actions.extend(crate::content::relics::meat_on_the_bone::on_victory(
                    &*state, used_up,
                ));
            }
            RelicId::Pocketwatch => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                crate::content::relics::pocketwatch::on_victory(&mut rs);
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::SlaversCollar => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::slavers_collar::on_victory(
                    state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            _ => {
                unreachable!("Relic present in on_victory bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

/// Triggers relics at the start of the player's turn.
pub fn at_turn_start(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.at_turn_start.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::AncientTeaSet => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(
                    crate::content::relics::ancient_tea_set::AncientTeaSet::at_turn_start(counter),
                );
            }
            RelicId::Brimstone => actions.extend(
                crate::content::relics::brimstone::Brimstone::at_turn_start(&*state),
            ),
            RelicId::CaptainsWheel => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(
                    crate::content::relics::captains_wheel::CaptainsWheel::at_turn_start(counter),
                );
            }
            RelicId::HappyFlower => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(
                    crate::content::relics::happy_flower::HappyFlower::at_turn_start(counter),
                );
            }
            RelicId::HornCleat => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions
                    .extend(crate::content::relics::horn_cleat::HornCleat::at_turn_start(counter));
            }
            RelicId::IncenseBurner => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::incense_burner::at_turn_start(
                    counter,
                ));
            }
            RelicId::Inserter => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::inserter::Inserter::at_turn_start(
                    counter,
                ));
            }
            RelicId::Lantern => {
                let used_up = state.entities.player.relics[relic_index].used_up;
                actions.extend(crate::content::relics::lantern::at_turn_start(used_up));
            }
            RelicId::Kunai => actions.extend(crate::content::relics::kunai::at_turn_start()),
            RelicId::Shuriken => actions.extend(crate::content::relics::shuriken::at_turn_start()),
            RelicId::MercuryHourglass => actions.extend(
                crate::content::relics::mercury_hourglass::at_turn_start(&*state),
            ),
            RelicId::Necronomicon => {
                actions.extend(crate::content::relics::necronomicon::at_turn_start())
            }
            RelicId::OrnamentalFan => {
                actions.extend(crate::content::relics::ornamental_fan::at_turn_start())
            }
            RelicId::Damaru => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::damaru::at_turn_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::EmotionChip => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::emotion_chip::at_turn_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            // FrozenCore moved to at_end_of_turn (Java: onPlayerEndTurn)
            RelicId::HoveringKite => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::hovering_kite::at_turn_start(
                    &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::StoneCalendar => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::stone_calendar::at_turn_start(
                    counter,
                ));
            }
            RelicId::WarpedTongs => {
                actions.extend(crate::content::relics::warped_tongs::at_turn_start(&*state))
            }
            RelicId::ArtOfWar => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::art_of_war::at_turn_start(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::GamblingChip => {
                actions.extend(crate::content::relics::gambling_chip::at_turn_start(
                    &*state,
                    &state.entities.player,
                ))
            }
            _ => unreachable!(
                "Relic present in at_turn_start bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

pub fn at_turn_start_post_draw(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state
        .entities
        .player
        .relic_buses
        .at_turn_start_post_draw
        .clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::Pocketwatch => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions
                    .extend(crate::content::relics::pocketwatch::at_turn_start_post_draw(&mut rs));
                state.entities.player.relics[relic_index] = rs;
            }
            _ => unreachable!(
                "Relic present in at_turn_start_post_draw bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

/// Triggers relics at the end of the player's turn.
pub fn at_end_of_turn(state: &mut CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let bus = state.entities.player.relic_buses.at_end_of_turn.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::CloakClasp => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::cloak_clasp::at_end_of_turn(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::GoldPlatedCables => {
                actions.extend(crate::content::relics::gold_plated_cables::at_end_of_turn(
                    &*state,
                    &state.entities.player,
                ))
            }
            RelicId::Orichalcum => {
                actions.extend(crate::content::relics::orichalcum::at_end_of_turn(&*state))
            }
            RelicId::StoneCalendar => {
                actions.extend(crate::content::relics::stone_calendar::at_end_of_turn(
                    &*state,
                    state.entities.player.relics[relic_index].counter,
                ))
            }
            RelicId::FrozenCore => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::frozen_core::at_end_of_turn(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::NilrysCodex => actions.extend(
                crate::content::relics::nilrys_codex::at_end_of_turn(&*state),
            ),
            _ => unreachable!(
                "Relic present in at_end_of_turn bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

/// Triggers relics after a card is used.
pub fn on_use_card(
    state: &mut CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let card_id = card.id;
    let base_def = crate::content::cards::get_card_definition(card_id);
    let is_attack = base_def.card_type == crate::content::cards::CardType::Attack;

    let bus = state.entities.player.relic_buses.on_use_card.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::BirdFacedUrn => actions
                .extend(crate::content::relics::bird_faced_urn::BirdFacedUrn::on_use_card(card_id)),
            RelicId::BlueCandle => actions
                .extend(crate::content::relics::blue_candle::BlueCandle::on_use_card(card_id)),
            RelicId::InkBottle => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::ink_bottle::on_use_card(counter));
            }
            RelicId::Kunai => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::kunai::on_use_card(card_id, counter));
            }
            RelicId::Nunchaku => {
                if is_attack {
                    let counter = state.entities.player.relics[relic_index].counter;
                    actions.extend(crate::content::relics::nunchaku::on_use_card(counter));
                }
            }
            RelicId::OrnamentalFan => {
                if is_attack {
                    let counter = state.entities.player.relics[relic_index].counter;
                    actions.extend(crate::content::relics::ornamental_fan::on_use_card(counter));
                }
            }
            RelicId::Pocketwatch => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                crate::content::relics::pocketwatch::on_use_card(&mut rs);
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::ArtOfWar => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::art_of_war::on_use_card(
                    &*state, &mut rs, card_id,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::PenNib => {
                if is_attack {
                    let counter = state.entities.player.relics[relic_index].counter;
                    actions.extend(crate::content::relics::pen_nib::on_use_card(counter));
                }
            }
            RelicId::Duality => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::duality::on_use_card(
                    &*state, &mut rs, card,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::MummifiedHand => {
                actions.extend(crate::content::relics::mummified_hand::on_use_card(
                    card, &*state,
                ));
            }
            RelicId::Shuriken => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::shuriken::on_use_card(
                    card_id, counter,
                ));
            }
            RelicId::LetterOpener => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::letter_opener::on_use_card(
                    &*state, card_id, counter,
                ));
            }
            RelicId::Necronomicon => {
                let cost = card.get_cost() as i32;
                let used_up = state.entities.player.relics[relic_index].used_up;
                actions.extend(crate::content::relics::necronomicon::on_use_card(
                    card_id, cost, used_up, card, target,
                ));
            }
            RelicId::OrangePellets => {
                let counter = state.entities.player.relics[relic_index].counter;
                actions.extend(crate::content::relics::orange_pellets::on_use_card(
                    card_id, counter,
                ));
            }
            RelicId::MedicalKit => {
                // Exhaust-on-Status is handled in core.rs should_exhaust check.
            }
            _ => {
                unreachable!("Relic present in on_use_card bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

pub fn on_apply_power(
    state: &mut CombatState,
    power_id: crate::content::powers::PowerId,
    target: crate::core::EntityId,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_apply_power.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::ChampionBelt => actions.extend(
                crate::content::relics::champion_belt::ChampionBelt::on_apply_power(
                    power_id, target,
                ),
            ),
            RelicId::SneckoSkull => actions.extend(
                crate::content::relics::snecko_skull::on_apply_power(power_id),
            ),
            _ => unreachable!(
                "Relic present in on_apply_power bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

pub fn on_monster_death(
    state: &mut CombatState,
    _target: crate::core::EntityId,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_monster_death.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::GremlinHorn => actions
                .extend(crate::content::relics::gremlin_horn::GremlinHorn::on_monster_death()),
            RelicId::TheSpecimen => actions.extend(
                crate::content::relics::the_specimen::on_monster_death(&*state, _target),
            ),
            _ => unreachable!(
                "Relic present in on_monster_death bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

pub fn on_discard(state: &mut CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let bus = state.entities.player.relic_buses.on_discard.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::HoveringKite => {
                let mut rs = state.entities.player.relics[relic_index].clone();
                actions.extend(crate::content::relics::hovering_kite::on_discard(
                    &*state, &mut rs,
                ));
                state.entities.player.relics[relic_index] = rs;
            }
            RelicId::ToughBandages => {
                actions.extend(crate::content::relics::tough_bandages::on_discard())
            }
            RelicId::Tingsha => {
                actions.extend(crate::content::relics::tingsha::on_discard(&*state))
            }
            _ => {
                unreachable!("Relic present in on_discard bus but unhandled in hooks.rs match arm")
            }
        }
    }
    actions
}

pub fn on_calculate_heal(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_calculate_heal {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::MagicFlower => {
                amount = crate::content::relics::magic_flower::modify_heal(amount)
            }
            RelicId::MarkOfTheBloom => {
                amount = crate::content::relics::mark_of_the_bloom::modify_heal(amount)
            }
            _ => unreachable!(
                "Relic present in on_calculate_heal bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    amount
}

pub fn on_attacked_to_change_damage(
    state: &CombatState,
    mut amount: i32,
    info: &crate::runtime::action::DamageInfo,
) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_attacked_to_change_damage {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::Torii => amount = crate::content::relics::torii::on_attacked_to_change_damage(info, amount),
            _ => unreachable!("Relic present in on_attacked_to_change_damage bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_lose_hp_last(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_lose_hp_last {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::TungstenRod => {
                amount = crate::content::relics::tungsten_rod::modify_hp_loss(amount)
            }
            _ => unreachable!(
                "Relic present in on_lose_hp_last bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    amount
}

pub fn on_calculate_x_cost(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_calculate_x_cost {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::ChemicalX => {
                amount = crate::content::relics::chemical_x::on_calculate_x_cost(amount)
            }
            _ => unreachable!(
                "Relic present in on_calculate_x_cost bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    amount
}

pub fn on_calculate_block_retained(state: &CombatState, mut block: i32) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_calculate_block_retained {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::Calipers => block = crate::content::relics::calipers::on_calculate_block_retained(block),
            _ => unreachable!("Relic present in on_calculate_block_retained bus but unhandled in hooks.rs match arm"),
        }
    }
    block
}

pub fn on_calculate_energy_retained(state: &CombatState) -> bool {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_calculate_energy_retained {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::IceCream => return crate::content::relics::ice_cream::on_calculate_energy_retained(),
            _ => unreachable!("Relic present in on_calculate_energy_retained bus but unhandled in hooks.rs match arm"),
        }
    }
    false
}

pub fn on_scry(state: &CombatState, mut amount: usize) -> usize {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_scry {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::GoldenEye => amount = crate::content::relics::golden_eye::on_scry(amount),
            _ => unreachable!("Relic present in on_scry bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_receive_power_modify(
    state: &CombatState,
    power_id: crate::content::powers::PowerId,
    mut amount: i32,
) -> i32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_receive_power_modify {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::Ginger => {
                amount = crate::content::relics::ginger::on_receive_power_modify(power_id, amount)
            }
            RelicId::Turnip => {
                amount = crate::content::relics::turnip::on_receive_power_modify(power_id, amount)
            }
            _ => unreachable!(
                "Relic present in on_receive_power_modify bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    amount
}

pub fn on_calculate_vulnerable_multiplier(state: &CombatState, owner_is_player: bool) -> f32 {
    let buses = &state.entities.player.relic_buses;
    for &relic_index in &buses.on_calculate_vulnerable_multiplier {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            RelicId::OddMushroom if owner_is_player => {
                return crate::content::relics::odd_mushroom::on_calculate_vulnerable_multiplier();
            }
            RelicId::PaperFrog if !owner_is_player => {
                return crate::content::relics::paper_frog::VULNERABLE_MULTIPLIER;
            }
            RelicId::OddMushroom | RelicId::PaperFrog => {}
            _ => unreachable!(
                "Relic present in on_calculate_vulnerable_multiplier bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    1.5
}

pub fn modify_player_attack_damage_for_card(
    state: &CombatState,
    card: &CombatCard,
    mut damage: f32,
) -> f32 {
    for relic in &state.entities.player.relics {
        match relic.id {
            RelicId::StrikeDummy => {
                damage = crate::content::relics::strike_dummy::modify_attack_damage_for_card(
                    card, damage,
                );
            }
            _ => {}
        }
    }
    damage
}

pub fn on_use_potion(
    state: &crate::runtime::combat::CombatState,
    player_id: crate::core::EntityId,
) -> smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    for &relic_index in &state.entities.player.relic_buses.on_use_potion {
        let relic_state = &state.entities.player.relics[relic_index];
        match relic_state.id {
            crate::content::relics::RelicId::ToyOrnithopter => actions.extend(
                crate::content::relics::toy_ornithopter::on_use_potion(state, player_id),
            ),
            _ => unreachable!(
                "Relic present in on_use_potion bus but unhandled in hooks.rs match arm"
            ),
        }
    }
    actions
}

pub fn on_change_stance(
    state: &mut CombatState,
    old_stance: crate::runtime::combat::StanceId,
    new_stance: crate::runtime::combat::StanceId,
) {
    let mut actions: smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> =
        smallvec::SmallVec::new();
    let old_stance_str = old_stance.as_str();
    let new_stance_str = new_stance.as_str();

    let bus = state.entities.player.relic_buses.on_change_stance.clone();
    for &relic_index in &bus {
        let relic_id = state.entities.player.relics[relic_index].id;
        match relic_id {
            RelicId::VioletLotus => {
                actions.extend(crate::content::relics::violet_lotus::on_change_stance(
                    old_stance_str,
                    new_stance_str,
                ))
            }
            _ => unreachable!(
                "Relic present in on_change_stance bus but unhandled in hooks.rs match arm"
            ),
        }
    }

    for action in actions {
        state.queue_action_back(action.action);
    }
}
