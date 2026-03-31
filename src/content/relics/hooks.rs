use crate::combat::{CombatState};
use crate::action::ActionInfo;
use crate::content::relics::RelicId;
use smallvec::SmallVec;

/// Triggers relics at the start of battle.
pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    for &relic_index in &state.player.relic_buses.at_battle_start {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::Akabeko => actions.extend(crate::content::relics::akabeko::Akabeko::at_battle_start()),
            RelicId::Anchor => actions.extend(crate::content::relics::anchor::Anchor::at_battle_start()),
            RelicId::BagOfMarbles => actions.extend(crate::content::relics::bag_of_marbles::BagOfMarbles::at_battle_start(state)),
            RelicId::BagOfPreparation => actions.extend(crate::content::relics::bag_of_preparation::BagOfPreparation::at_battle_start()),
            RelicId::BloodVial => actions.extend(crate::content::relics::blood_vial::BloodVial::at_battle_start()),
            RelicId::BronzeScales => actions.extend(crate::content::relics::bronze_scales::BronzeScales::at_battle_start(state.player.id)),
            RelicId::ClockworkSouvenir => actions.extend(crate::content::relics::clockwork_souvenir::ClockworkSouvenir::at_battle_start()),
            RelicId::Dodecahedron => actions.extend(crate::content::relics::dodecahedron::Dodecahedron::at_battle_start(state)),
            RelicId::FossilizedHelix => actions.extend(crate::content::relics::fossilized_helix::FossilizedHelix::at_battle_start()),
            RelicId::Girya => actions.extend(crate::content::relics::girya::Girya::at_battle_start(relic_state.counter)),
            RelicId::HornCleat => actions.extend(crate::content::relics::horn_cleat::HornCleat::at_battle_start()),
            RelicId::Lantern => actions.extend(crate::content::relics::lantern::at_battle_start()),
            RelicId::Mango => actions.extend(crate::content::relics::mango::at_battle_start()),
            RelicId::NinjaScroll => actions.extend(crate::content::relics::ninja_scroll::at_battle_start()),
            RelicId::NuclearBattery => actions.extend(crate::content::relics::nuclear_battery::at_battle_start()),
            RelicId::OddlySmoothStone => actions.extend(crate::content::relics::oddly_smooth_stone::at_battle_start()),
            RelicId::Pantograph => {
                // Heals 25 at start of BOSS combats. Currently, engine doesn't explicitly flag "boss" well in state.
                let mut is_boss_combat = false;
                for m in &state.monsters {
                    if let Some(enemy_id) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
                        if matches!(enemy_id, 
                            crate::content::monsters::EnemyId::SlimeBoss |
                            crate::content::monsters::EnemyId::Hexaghost |
                            crate::content::monsters::EnemyId::TheGuardian |
                            crate::content::monsters::EnemyId::BronzeAutomaton |
                            crate::content::monsters::EnemyId::TheCollector |
                            crate::content::monsters::EnemyId::Champ |
                            crate::content::monsters::EnemyId::AwakenedOne |
                            crate::content::monsters::EnemyId::TimeEater |
                            crate::content::monsters::EnemyId::Donu |
                            crate::content::monsters::EnemyId::Deca |
                            crate::content::monsters::EnemyId::CorruptHeart
                        ) {
                            is_boss_combat = true;
                            break;
                        }
                    }
                }
                if is_boss_combat {
                    actions.push(ActionInfo {
                        action: crate::action::Action::Heal { target: 0, amount: 25 },
                        insertion_mode: crate::action::AddTo::Bottom,
                    });
                }
            },
            RelicId::PreservedInsect => actions.extend(crate::content::relics::preserved_insect::at_battle_start(state)),
            RelicId::CrackedCore => actions.extend(crate::content::relics::cracked_core::at_battle_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::DataDisk => actions.extend(crate::content::relics::data_disk::at_battle_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::DuVuDoll => actions.extend(crate::content::relics::du_vu_doll::at_battle_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::Enchiridion => actions.extend(crate::content::relics::enchiridion::at_battle_start(state, &mut state.player.relics.clone()[relic_index])),
            // GamblingChip: moved to at_turn_start (Java: atTurnStartPostDraw)
            RelicId::GremlinMask => actions.extend(crate::content::relics::gremlin_mask::at_battle_start(state, &state.player)),
            RelicId::HolyWater => actions.extend(crate::content::relics::holy_water::at_battle_start(state)),
            RelicId::SnakeRing => actions.extend(crate::content::relics::snake_ring::at_battle_start()),
            RelicId::SneckoEye => actions.extend(crate::content::relics::snecko_eye::at_battle_start()),
            RelicId::Vajra => actions.extend(crate::content::relics::vajra::at_battle_start()),
            RelicId::RedMask => actions.extend(crate::content::relics::red_mask::at_battle_start(state)),
            RelicId::PenNib => actions.extend(crate::content::relics::pen_nib::at_battle_start(relic_state.counter)),
            // P1 relics
            RelicId::PhilosopherStone => actions.extend(crate::content::relics::philosopher_stone::at_battle_start(state)),
            RelicId::MarkOfPain => actions.extend(crate::content::relics::mark_of_pain::at_battle_start()),
            RelicId::ThreadAndNeedle => actions.extend(crate::content::relics::thread_and_needle::at_battle_start()),
            RelicId::MutagenicStrength => actions.extend(crate::content::relics::mutagenic_strength::at_battle_start()),
            RelicId::NeowsLament => actions.extend(crate::content::relics::neows_lament::at_battle_start(state, relic_state.counter)),
            RelicId::TwistedFunnel => actions.extend(crate::content::relics::twisted_funnel::at_battle_start(state)),
            RelicId::Sling => actions.extend(crate::content::relics::sling::at_battle_start()),
            RelicId::RedSkull => actions.extend(crate::content::relics::red_skull::at_battle_start(state.player.current_hp, state.player.max_hp)),
            RelicId::SlaversCollar => actions.extend(crate::content::relics::slavers_collar::at_battle_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::TeardropLocket => {
                // Java: this.addToTop(new ChangeStanceAction("Calm"))
                actions.push(ActionInfo {
                    action: crate::action::Action::EnterStance("Calm".to_string()),
                    insertion_mode: crate::action::AddTo::Top,
                });
            },
            RelicId::PureWater => {
                // Java: this.addToBot(new MakeTempCardInHandAction(new Miracle(), 1, false))
                actions.push(ActionInfo {
                    action: crate::action::Action::MakeTempCardInHand { card_id: crate::content::cards::CardId::Miracle, amount: 1, upgraded: false },
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            },
            RelicId::SymbioticVirus => {
                // Java: atPreBattle() → AbstractDungeon.player.channelOrb(new Dark())
                actions.push(ActionInfo {
                    action: crate::action::Action::ChannelOrb(crate::combat::OrbId::Dark),
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            },
            RelicId::RunicCapacitor => {
                // Java: atBattleStart() → addToBot(IncreaseMaxOrbAction(3))
                actions.push(ActionInfo {
                    action: crate::action::Action::IncreaseMaxOrb(3),
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            },
            RelicId::Toolbox => {
                // Java: atBattleStartPreDraw() → addToBot(new ChooseOneColorless())
                // 3 random colorless cards → hand, can_skip=false
                actions.push(ActionInfo {
                    action: crate::action::Action::SuspendForCardReward {
                        pool: crate::action::CardRewardPool::Colorless,
                        destination: crate::action::CardDestination::Hand,
                        can_skip: false,
                    },
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            },
            _ => unreachable!("Relic present in at_battle_start bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

/// Triggers relics when the draw pile is shuffled.
pub fn on_shuffle(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    for &relic_index in &state.player.relic_buses.on_shuffle {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::Abacus => actions.extend(crate::content::relics::abacus::Abacus::on_shuffle()),
            RelicId::Melange => actions.extend(crate::content::relics::melange::on_shuffle()),
            RelicId::Sundial => actions.extend(crate::content::relics::sundial::on_shuffle(relic_state.counter)),
            _ => unreachable!("Relic present in on_shuffle bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_spawn_monster(state: &mut CombatState, target_idx: usize) {
    if state.player.relic_buses.on_spawn_monster.contains(&r_idx(state, RelicId::PhilosopherStone)) {
        if state.player.has_relic(RelicId::PhilosopherStone) {
            let m_id = state.monsters[target_idx].id;
            state.action_queue.push_back(crate::action::Action::ApplyPower {
                source: m_id,
                target: m_id,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 1,
            });
        }
    }
}

fn r_idx(state: &CombatState, id: RelicId) -> usize {
    state.player.relics.iter().position(|r| r.id == id).unwrap_or(999)
}

pub fn on_exhaust(state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_exhaust {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::CharonsAshes => actions.extend(crate::content::relics::charons_ashes::CharonsAshes::on_exhaust(state)),
            RelicId::DeadBranch => actions.extend(crate::content::relics::dead_branch::on_exhaust(state, &mut state.player.relics.clone()[relic_index])),
            _ => unreachable!("Relic present in on_exhaust bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_lose_hp(state: &CombatState, amount: i32) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_lose_hp {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::CentennialPuzzle => actions.extend(crate::content::relics::centennial_puzzle::CentennialPuzzle::on_lose_hp(relic_state.used_up)),
            RelicId::EmotionChip => actions.extend(crate::content::relics::emotion_chip::on_lose_hp(state, &mut state.player.relics.clone()[relic_index], amount)),
            RelicId::LizardTail => actions.extend(crate::content::relics::lizard_tail::on_lose_hp(state, relic_state.used_up)),
            RelicId::SelfFormingClay => actions.extend(crate::content::relics::self_forming_clay::on_lose_hp()),
            RelicId::TungstenRod => actions.extend(crate::content::relics::tungsten_rod::on_lose_hp(amount)),
            RelicId::RunicCube => actions.extend(crate::content::relics::runic_cube::was_hp_lost(amount)),
            _ => unreachable!("Relic present in on_lose_hp bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_victory(state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_victory {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::BurningBlood => actions.extend(crate::content::relics::burning_blood::BurningBlood::on_victory()),
            RelicId::DarkBlood => actions.extend(crate::content::relics::dark_blood::DarkBlood::on_victory()),
            RelicId::BlackBlood => actions.extend(crate::content::relics::black_blood::BlackBlood::on_victory()),
            RelicId::BlackStar => actions.extend(crate::content::relics::black_star::BlackStar::on_victory()),
            RelicId::FaceOfCleric => actions.extend(crate::content::relics::face_of_cleric::on_victory(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::MeatOnTheBone => actions.extend(crate::content::relics::meat_on_the_bone::on_victory(state, relic_state.used_up)),
            _ => unreachable!("Relic present in on_victory bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

/// Triggers relics at the start of the player's turn.
pub fn at_turn_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    for &relic_index in &state.player.relic_buses.at_turn_start {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::AncientTeaSet => actions.extend(crate::content::relics::ancient_tea_set::AncientTeaSet::at_turn_start(relic_state.counter)),
            RelicId::Brimstone => actions.extend(crate::content::relics::brimstone::Brimstone::at_turn_start(state)),
            RelicId::CaptainsWheel => actions.extend(crate::content::relics::captains_wheel::CaptainsWheel::at_turn_start(relic_state.counter)),
            RelicId::HappyFlower => actions.extend(crate::content::relics::happy_flower::HappyFlower::at_turn_start(relic_state.counter)),
            RelicId::HornCleat => actions.extend(crate::content::relics::horn_cleat::HornCleat::at_turn_start(relic_state.counter)),
            RelicId::IncenseBurner => actions.extend(crate::content::relics::incense_burner::at_turn_start(relic_state.counter)),
            RelicId::Inserter => actions.extend(crate::content::relics::inserter::Inserter::at_turn_start(relic_state.counter)),
            RelicId::Lantern => actions.extend(crate::content::relics::lantern::at_turn_start(relic_state.used_up)),
            RelicId::MercuryHourglass => actions.extend(crate::content::relics::mercury_hourglass::at_turn_start(state)),
            RelicId::OrnamentalFan => actions.extend(crate::content::relics::ornamental_fan::at_turn_start()),
            RelicId::Damaru => actions.extend(crate::content::relics::damaru::at_turn_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::EmotionChip => actions.extend(crate::content::relics::emotion_chip::at_turn_start(state, &mut state.player.relics.clone()[relic_index])),
            // FrozenCore moved to at_end_of_turn (Java: onPlayerEndTurn)
            RelicId::HoveringKite => actions.extend(crate::content::relics::hovering_kite::at_turn_start(&mut state.player.relics.clone()[relic_index])),
            RelicId::Pocketwatch => actions.extend(crate::content::relics::pocketwatch::at_turn_start(relic_state.counter)),
            RelicId::WarpedTongs => actions.extend(crate::content::relics::warped_tongs::at_turn_start(state)),
            RelicId::ArtOfWar => actions.extend(crate::content::relics::art_of_war::at_turn_start(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::GamblingChip => actions.extend(crate::content::relics::gambling_chip::at_turn_start(state, &state.player)),
            _ => unreachable!("Relic present in at_turn_start bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

/// Triggers relics at the end of the player's turn.
pub fn at_end_of_turn(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    // Use mutable tracking since some relics like CloakClasp could arguably mutate but we do state.player
    for &relic_index in &state.player.relic_buses.at_end_of_turn {
        let relic_state = &mut state.player.relics.clone()[relic_index];
        match relic_state.id {
            RelicId::CloakClasp => actions.extend(crate::content::relics::cloak_clasp::at_end_of_turn(state, relic_state)),
            RelicId::GoldPlatedCables => actions.extend(crate::content::relics::gold_plated_cables::at_end_of_turn(state, &state.player)),
            RelicId::Orichalcum => actions.extend(crate::content::relics::orichalcum::at_end_of_turn(state)),
            RelicId::StoneCalendar => actions.extend(crate::content::relics::stone_calendar::at_end_of_turn(state)),
            RelicId::Pocketwatch => actions.extend(crate::content::relics::pocketwatch::at_end_of_turn(state)),
            RelicId::FrozenCore => actions.extend(crate::content::relics::frozen_core::at_end_of_turn(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::NilrysCodex => {
                // Java: onPlayerEndTurn() → addToBot(new CodexAction())
                // CodexAction: 3 random from ALL pools → draw pile, canSkip=true
                // Java also checks areMonstersBasicallyDead() — we skip if combat ending
                let all_dead = state.monsters.iter().all(|m| m.current_hp <= 0 || m.is_dying || m.is_escaped);
                if !all_dead {
                    actions.push(ActionInfo {
                        action: crate::action::Action::SuspendForCardReward {
                            pool: crate::action::CardRewardPool::ClassAll,
                            destination: crate::action::CardDestination::DrawPileRandom,
                            can_skip: true,
                        },
                        insertion_mode: crate::action::AddTo::Bottom,
                    });
                }
            },
            _ => unreachable!("Relic present in at_end_of_turn bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}


/// Triggers relics after a card is used.
pub fn on_use_card(state: &CombatState, card_id: crate::content::cards::CardId) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let base_def = crate::content::cards::get_card_definition(card_id);
    let is_attack = base_def.card_type == crate::content::cards::CardType::Attack;

    for &relic_index in &state.player.relic_buses.on_use_card {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::BirdFacedUrn => actions.extend(crate::content::relics::bird_faced_urn::BirdFacedUrn::on_use_card(card_id)),
            RelicId::BlueCandle => actions.extend(crate::content::relics::blue_candle::BlueCandle::on_use_card(card_id)),
            RelicId::InkBottle => actions.extend(crate::content::relics::ink_bottle::on_use_card(relic_state.counter)),
            RelicId::Kunai => actions.extend(crate::content::relics::kunai::on_use_card(card_id, relic_state.counter)),
            RelicId::Nunchaku => if is_attack { actions.extend(crate::content::relics::nunchaku::on_use_card(relic_state.counter)) },
            RelicId::OrnamentalFan => if is_attack { actions.extend(crate::content::relics::ornamental_fan::on_use_card(relic_state.counter)) },
            RelicId::ArtOfWar => actions.extend(crate::content::relics::art_of_war::on_use_card(state, &mut state.player.relics.clone()[relic_index], card_id)),
            RelicId::PenNib => if is_attack { actions.extend(crate::content::relics::pen_nib::on_use_card(relic_state.counter)) },
            RelicId::Duality => {
                let card = crate::combat::CombatCard {
                    id: card_id,
                    uuid: 0,
                    cost_modifier: 0,
                    cost_for_turn: None,
                    base_damage_mut: 0,
                    base_block_mut: 0,
                    base_magic_num_mut: 0,
                    upgrades: 0,
                    misc_value: 0,
                    multi_damage: smallvec::SmallVec::new(),
                    exhaust_override: None,
                    retain_override: None,
                    free_to_play_once: false,
                    energy_on_use: 0,
                };
                actions.extend(crate::content::relics::mummified_hand::on_use_card(&card, state));
                actions.extend(crate::content::relics::duality::on_use_card(state, &mut state.player.relics.clone()[relic_index], &card));
            }
            RelicId::Shuriken => actions.extend(crate::content::relics::shuriken::on_use_card(card_id, relic_state.counter)),
            RelicId::LetterOpener => actions.extend(crate::content::relics::letter_opener::on_use_card(state, card_id, relic_state.counter)),
            RelicId::Necronomicon => {
                // Need card cost and card struct for replay
                let card_def = crate::content::cards::get_card_definition(card_id);
                let cost = card_def.cost as i32;
                // Build a minimal CombatCard for replay
                let combat_card = crate::combat::CombatCard {
                    id: card_id, uuid: 0, cost_modifier: 0, cost_for_turn: None,
                    base_damage_mut: 0, base_block_mut: 0, base_magic_num_mut: 0,
                    upgrades: 0, misc_value: 0, multi_damage: smallvec::SmallVec::new(),
                    exhaust_override: None, retain_override: None, free_to_play_once: false,
                    energy_on_use: 0,
                };
                actions.extend(crate::content::relics::necronomicon::on_use_card(card_id, cost, relic_state.counter, &combat_card, None));
            }
            RelicId::OrangePellets => actions.extend(crate::content::relics::orange_pellets::on_use_card(card_id, relic_state.counter)),
            RelicId::MedicalKit => {
                // Exhaust-on-Status is handled in core.rs should_exhaust check.
                // No additional actions needed from the hook.
            },
            _ => unreachable!("Relic present in on_use_card bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_apply_power(state: &CombatState, power_id: crate::content::powers::PowerId, target: crate::core::EntityId) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_apply_power {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::ChampionBelt => actions.extend(crate::content::relics::champion_belt::ChampionBelt::on_apply_power(power_id, target)),
            RelicId::SneckoSkull => actions.extend(crate::content::relics::snecko_skull::on_apply_power(power_id)),
            _ => unreachable!("Relic present in on_apply_power bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_monster_death(state: &CombatState, _target: crate::core::EntityId) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_monster_death {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::GremlinHorn => actions.extend(crate::content::relics::gremlin_horn::GremlinHorn::on_monster_death()),
            RelicId::TheSpecimen => actions.extend(crate::content::relics::the_specimen::on_monster_death(state, _target)),
            _ => unreachable!("Relic present in on_monster_death bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_discard(state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_discard {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::HoveringKite => actions.extend(crate::content::relics::hovering_kite::on_discard(state, &mut state.player.relics.clone()[relic_index])),
            RelicId::ToughBandages => actions.extend(crate::content::relics::tough_bandages::on_discard()),
            RelicId::Tingsha => actions.extend(crate::content::relics::tingsha::on_discard(state)),
            _ => unreachable!("Relic present in on_discard bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_calculate_heal(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_calculate_heal {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::MagicFlower => amount = crate::content::relics::magic_flower::modify_heal(amount),
            RelicId::MarkOfTheBloom => { amount = 0; }, // Java: onPlayerHeal → return 0
            _ => unreachable!("Relic present in on_calculate_heal bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_attacked_to_change_damage(state: &CombatState, mut amount: i32, info: &crate::action::DamageInfo) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_attacked_to_change_damage {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::Torii => amount = crate::content::relics::torii::on_attacked_to_change_damage(info, amount),
            _ => unreachable!("Relic present in on_attacked_to_change_damage bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_lose_hp_last(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_lose_hp_last {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::TungstenRod => amount = crate::content::relics::tungsten_rod::modify_hp_loss(amount),
            _ => unreachable!("Relic present in on_lose_hp_last bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_calculate_x_cost(state: &CombatState, mut amount: i32) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_calculate_x_cost {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::ChemicalX => amount += 2,
            _ => unreachable!("Relic present in on_calculate_x_cost bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_calculate_block_retained(state: &CombatState, mut block: i32) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_calculate_block_retained {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::Calipers => block = (block - 15).max(0),
            _ => unreachable!("Relic present in on_calculate_block_retained bus but unhandled in hooks.rs match arm"),
        }
    }
    block
}

pub fn on_calculate_energy_retained(state: &CombatState) -> bool {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_calculate_energy_retained {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::IceCream => return true,
            _ => unreachable!("Relic present in on_calculate_energy_retained bus but unhandled in hooks.rs match arm"),
        }
    }
    false
}

pub fn on_scry(state: &CombatState, mut amount: usize) -> usize {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_scry {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::GoldenEye => amount += 2,
            _ => unreachable!("Relic present in on_scry bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_receive_power_modify(state: &CombatState, power_id: crate::content::powers::PowerId, mut amount: i32) -> i32 {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_receive_power_modify {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::Ginger => {
                if power_id == crate::content::powers::PowerId::Weak {
                    amount = 0;
                }
            },
            RelicId::Turnip => {
                if power_id == crate::content::powers::PowerId::Frail {
                    amount = 0;
                }
            },
            _ => unreachable!("Relic present in on_receive_power_modify bus but unhandled in hooks.rs match arm"),
        }
    }
    amount
}

pub fn on_calculate_vulnerable_multiplier(state: &CombatState) -> bool {
    let buses = &state.player.relic_buses;
    for &relic_index in &buses.on_calculate_vulnerable_multiplier {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::OddMushroom => return true,
            _ => unreachable!("Relic present in on_calculate_vulnerable_multiplier bus but unhandled in hooks.rs match arm"),
        }
    }
    false
}

pub fn on_use_potion(state: &crate::combat::CombatState, player_id: crate::core::EntityId) -> smallvec::SmallVec<[crate::action::ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    for &relic_index in &state.player.relic_buses.on_use_potion {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            crate::content::relics::RelicId::ToyOrnithopter => actions.extend(crate::content::relics::toy_ornithopter::on_use_potion(state, player_id)),
            _ => unreachable!("Relic present in on_use_potion bus but unhandled in hooks.rs match arm"),
        }
    }
    actions
}

pub fn on_change_stance(state: &mut CombatState, old_stance: crate::combat::StanceId, new_stance: crate::combat::StanceId) {
    let mut actions: smallvec::SmallVec<[crate::action::ActionInfo; 4]> = smallvec::SmallVec::new();
    let old_stance_str = old_stance.as_str();
    let new_stance_str = new_stance.as_str();

    for &relic_index in &state.player.relic_buses.on_change_stance {
        let relic_state = &state.player.relics[relic_index];
        match relic_state.id {
            RelicId::VioletLotus => actions.extend(crate::content::relics::violet_lotus::on_change_stance(old_stance_str, new_stance_str)),
            _ => unreachable!("Relic present in on_change_stance bus but unhandled in hooks.rs match arm"),
        }
    }
    
    for action in actions {
        state.action_queue.push_back(action.action);
    }
}
