pub mod core;
pub mod ironclad;
pub mod silent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerId {
    Strength,
    Vulnerable,
    Weak,
    Frail,
    Dexterity,
    Ritual,
    Poison,
    LoseStrength,
    DemonForm,
    Corruption,
    DoubleTap,
    DuplicationPower,
    FeelNoPain,
    DarkEmbrace,
    Minion,
    Rupture,
    Combust,
    Brutality,
    Barricade,
    Juggernaut,
    FlameBarrier,
    Metallicize,
    CurlUp,
    SporeCloud,
    Split,
    Entangle,
    Evolve,
    FireBreathing,
    NoDraw,
    Regen,
    Rage,
    Berserk,
    Angry,
    Anger,
    LagavulinSleep,
    ModeShift,
    SharpHide,
    Artifact,
    GuardianThreshold,
    Flight,
    Hex,
    Malleable,
    PlatedArmor,
    Confusion,
    Thievery,
    PainfulStabs,
    Stasis,
    GenericStrengthUp,
    Explosive,
    Thorns,
    Regrow,
    Constricted,
    Fading,
    Shifting,
    Reactive,
    Slow,
    Intangible,
    Curiosity,
    Unawakened,
    TimeWarp,
    Invincible,
    Vigor,
    BeatOfDeath,
    Mantra,
    Focus,
    DexterityDown,
    PenNibPower,
    NextTurnBlock,
    Energized,
    // Colorless card powers
    MagnetismPower,
    MayhemPower,
    PanachePower,
    SadisticPower,
    TheBombPower,
    Buffer,
    Electro,
    NoSkills,
    ThousandCuts,
    Shackled,
    DrawReduction,
    Surrounded,
    BackAttack,
}

use crate::combat::{CombatState, CombatCard};

pub struct PowerDefinition {
    pub id: PowerId,
    pub name: &'static str,
}

/// Java: AbstractPower.PowerType — BUFF or DEBUFF.
/// Some powers are dynamic: Strength/Focus/Dexterity are DEBUFF when amount < 0.
/// Used by: SadisticPower.onApplyPower, RemoveDebuffsAction, Artifact interception.
pub fn is_debuff(id: PowerId, amount: i32) -> bool {
    match id {
        // Dynamic: DEBUFF when amount < 0
        PowerId::Strength | PowerId::Focus | PowerId::Dexterity => amount < 0,
        // Fixed DEBUFFs (from Java constructor: this.type = PowerType.DEBUFF)
        PowerId::Vulnerable | PowerId::Weak | PowerId::Frail | PowerId::Poison
        | PowerId::Entangle | PowerId::NoDraw | PowerId::Constricted | PowerId::Confusion
        | PowerId::Hex | PowerId::Slow | PowerId::LoseStrength | PowerId::DexterityDown
        | PowerId::NoSkills | PowerId::Fading | PowerId::Shackled | PowerId::DrawReduction => true,
        // Everything else is BUFF
        _ => false,
    }
}

pub fn get_power_definition(id: PowerId) -> PowerDefinition {
    match id {
        PowerId::Strength => PowerDefinition { id, name: "Strength" },
        PowerId::Vulnerable => PowerDefinition { id, name: "Vulnerable" },
        PowerId::Weak => PowerDefinition { id, name: "Weak" },
        PowerId::Frail => PowerDefinition { id, name: "Frail" },
        PowerId::Dexterity => PowerDefinition { id, name: "Dexterity" },
        PowerId::Ritual => PowerDefinition { id, name: "Ritual" },
        PowerId::Poison => PowerDefinition { id, name: "Poison" },
        PowerId::LoseStrength => PowerDefinition { id, name: "Downfall" },
        PowerId::DemonForm => PowerDefinition { id, name: "Demon Form" },
        PowerId::Corruption => PowerDefinition { id, name: "Corruption" },
        PowerId::DoubleTap => PowerDefinition { id, name: "Double Tap" },
        PowerId::DuplicationPower => PowerDefinition { id, name: "Duplication" },
        PowerId::FeelNoPain => PowerDefinition { id, name: "Feel No Pain" },
        PowerId::DarkEmbrace => PowerDefinition { id, name: "Dark Embrace" },
        PowerId::Minion => PowerDefinition { id, name: "Minion" },
        PowerId::Rupture => PowerDefinition { id, name: "Rupture" },
        PowerId::Combust => PowerDefinition { id, name: "Combust" },
        PowerId::Brutality => PowerDefinition { id, name: "Brutality" },
        PowerId::Barricade => PowerDefinition { id, name: "Barricade" },
        PowerId::Juggernaut => PowerDefinition { id, name: "Juggernaut" },
        PowerId::FlameBarrier => PowerDefinition { id, name: "Flame Barrier" },
        PowerId::Metallicize => PowerDefinition { id, name: "Metallicize" },
        PowerId::CurlUp => PowerDefinition { id, name: "Curl Up" },
        PowerId::SporeCloud => PowerDefinition { id, name: "Spore Cloud" },
        PowerId::Split => PowerDefinition { id, name: "Split" },
        PowerId::GenericStrengthUp => PowerDefinition { id, name: "Generic Strength Up" },
        PowerId::Entangle => PowerDefinition { id, name: "Entangled" },
        PowerId::Evolve => PowerDefinition { id, name: "Evolve" },
        PowerId::FireBreathing => PowerDefinition { id, name: "Fire Breathing" },
        PowerId::NoDraw => PowerDefinition { id, name: "No Draw" },
        PowerId::Regen => PowerDefinition { id, name: "Regen" },
        PowerId::Rage => PowerDefinition { id, name: "Rage" },
        PowerId::Berserk => PowerDefinition { id, name: "Berserk" },
        PowerId::Angry => PowerDefinition { id, name: "Angry" },
        PowerId::Anger => PowerDefinition { id, name: "Anger" },
        PowerId::LagavulinSleep => PowerDefinition { id, name: "LagavulinSleep" },
        PowerId::ModeShift => PowerDefinition { id, name: "Mode Shift" },
        PowerId::SharpHide => PowerDefinition { id, name: "Sharp Hide" },
        PowerId::Artifact => PowerDefinition { id, name: "Artifact" },
        PowerId::GuardianThreshold => PowerDefinition { id, name: "Guardian Threshold" },
        PowerId::Flight => PowerDefinition { id, name: "Flight" },
        PowerId::Hex => PowerDefinition { id, name: "Hex" },
        PowerId::Malleable => PowerDefinition { id, name: "Malleable" },
        PowerId::PlatedArmor => PowerDefinition { id, name: "Plated Armor" },
        PowerId::Confusion => PowerDefinition { id, name: "Confusion" },
        PowerId::Thievery => PowerDefinition { id, name: "Thievery" },
        PowerId::PainfulStabs => PowerDefinition { id, name: "Painful Stabs" },
        PowerId::Stasis => PowerDefinition { id, name: "Stasis" },
        PowerId::Explosive => PowerDefinition { id, name: "Explosive" },
        PowerId::Thorns => PowerDefinition { id, name: "Thorns" },
        PowerId::Regrow => PowerDefinition { id, name: "Regrow" },
        PowerId::Constricted => PowerDefinition { id, name: "Constricted" },
        PowerId::Fading => PowerDefinition { id, name: "Fading" },
        PowerId::Shifting => PowerDefinition { id, name: "Shifting" },
        PowerId::Reactive => PowerDefinition { id, name: "Reactive" },
        PowerId::Slow => PowerDefinition { id, name: "Slow" },
        PowerId::Intangible => PowerDefinition { id, name: "Intangible" },
        PowerId::Curiosity => PowerDefinition { id, name: "Curiosity" },
        PowerId::Unawakened => PowerDefinition { id, name: "Unawakened" },
        PowerId::Shackled => PowerDefinition { id, name: "Shackled" },
        PowerId::DrawReduction => PowerDefinition { id: PowerId::DrawReduction, name: "Draw Reduction" },
        PowerId::Surrounded => PowerDefinition { id: PowerId::Surrounded, name: "Surrounded" },
        PowerId::BackAttack => PowerDefinition { id: PowerId::BackAttack, name: "Back Attack" },
        PowerId::TimeWarp => PowerDefinition { id, name: "Time Warp" },
        PowerId::Invincible => PowerDefinition { id, name: "Invincible" },
        PowerId::BeatOfDeath => PowerDefinition { id, name: "Beat of Death" },
        PowerId::Vigor => PowerDefinition { id, name: "Vigor" },
        PowerId::Mantra => PowerDefinition { id, name: "Mantra" },
        PowerId::Focus => PowerDefinition { id, name: "Focus" },
        PowerId::DexterityDown => PowerDefinition { id, name: "Dexterity Down" },
        PowerId::PenNibPower => PowerDefinition { id, name: "Pen Nib" },
        PowerId::NextTurnBlock => PowerDefinition { id, name: "Next Turn Block" },
        PowerId::Energized => PowerDefinition { id, name: "Energized" },
        PowerId::MagnetismPower => PowerDefinition { id, name: "Magnetism" },
        PowerId::MayhemPower => PowerDefinition { id, name: "Mayhem" },
        PowerId::PanachePower => PowerDefinition { id, name: "Panache" },
        PowerId::SadisticPower => PowerDefinition { id, name: "Sadistic Nature" },
        PowerId::TheBombPower => PowerDefinition { id, name: "The Bomb" },
        PowerId::Buffer => PowerDefinition { id, name: "Buffer" },
        PowerId::Electro => PowerDefinition { id, name: "Electrodynamics" },
        PowerId::NoSkills => PowerDefinition { id, name: "No Skills" },
        PowerId::ThousandCuts => PowerDefinition { id, name: "A Thousand Cuts" },
    }
}

pub fn power_removes_at_zero(id: PowerId) -> bool {
    match id {
        PowerId::Strength | PowerId::Dexterity => false,
        _ => true,
    }
}

// Global Power Hook Routers
pub fn resolve_power_on_apply(id: PowerId, state: &mut CombatState, _target: crate::core::EntityId) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_apply(state),
        _ => {}
    }
}

pub fn resolve_power_on_card_draw(id: PowerId, state: &CombatState, card: &mut CombatCard) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_card_draw(state, card),
        _ => {} 
    }
}

pub fn resolve_power_on_use_card(id: PowerId, state: &mut CombatState, card: &CombatCard, exhaust_override: &mut bool, purge: bool, target: Option<usize>) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_use_card(state, card, exhaust_override),
        PowerId::DoubleTap => ironclad::double_tap::on_use_card(state, card, purge, target),
        PowerId::DuplicationPower => core::duplication_power::on_use_card(state, card, purge, target),
        PowerId::PenNibPower => {
            let actions = core::pen_nib::on_use_card(card);
            for action in actions {
                state.action_queue.push_back(action);
            }
        },
        PowerId::Vigor => {
            // Java: onUseCard — remove Vigor when an attack card is played
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Attack {
                state.action_queue.push_back(crate::action::Action::RemovePower {
                    target: 0, // Player
                    power_id: PowerId::Vigor,
                });
            }
        },
        _ => {}
    }
}

pub fn resolve_power_on_player_card_played(
    id: PowerId, owner: crate::core::EntityId, amount: i32, card: &CombatCard, state: &CombatState
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Curiosity => core::curiosity::on_player_card_played(owner, amount, card),
        PowerId::TimeWarp => core::time_warp::on_player_card_played(owner, amount, card, state),
        PowerId::BeatOfDeath => core::beat_of_death::on_player_card_played(owner, amount, card),
        _ => smallvec::smallvec![]
    }
}


pub fn resolve_power_on_exhaust(id: PowerId, _state: &CombatState, owner: crate::core::EntityId, amount: i32, _card_uuid: u32, _card_id: crate::content::cards::CardId) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::FeelNoPain => ironclad::feel_no_pain::on_exhaust(owner, amount),
        PowerId::DarkEmbrace => ironclad::dark_embrace::on_exhaust(amount),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_hp_lost(id: PowerId, state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Rupture => ironclad::rupture::on_hp_lost(amount),
        PowerId::LagavulinSleep => core::lagavulin_sleep::on_hp_lost(owner),
        PowerId::Split => core::split::on_hp_lost(state, owner, amount),
        PowerId::ModeShift => core::mode_shift::on_hp_lost(state, owner, amount),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_card_played(id: PowerId, state: &CombatState, owner: crate::core::EntityId, card: &CombatCard, power_amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Anger => core::anger::on_card_played(state, owner, card, power_amount),
        PowerId::BeatOfDeath => core::beat_of_death::on_player_card_played(owner, power_amount, card),
        PowerId::SharpHide => core::sharp_hide::on_card_played(state, owner, card, power_amount),
        PowerId::Hex => core::hex::on_card_played(state, owner, card, power_amount),
        PowerId::ThousandCuts => silent::thousand_cuts::on_after_card_played(owner, power_amount),
        PowerId::Curiosity => core::curiosity::on_player_card_played(owner, power_amount, card),
        PowerId::TimeWarp => core::time_warp::on_player_card_played(owner, power_amount, card, state),
        PowerId::PanachePower => {
            // Every 5th card played, deal damage to ALL enemies
            // amount = countdown (decremented), extra_data = damage value
            let mut acts = smallvec::SmallVec::new();
            // Decrement counter via ApplyPower(-1)
            acts.push(crate::action::Action::ApplyPower {
                source: owner, target: owner, power_id: PowerId::PanachePower, amount: -1,
            });
            // Check if counter will hit 0 (current amount - 1 == 0 means we're at 1 now)
            if power_amount == 1 {
                let damage = state.power_db.get(&owner)
                    .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::PanachePower))
                    .map(|p| p.extra_data)
                    .unwrap_or(10);
                acts.push(crate::action::Action::DamageAllEnemies {
                    source: owner,
                    damages: smallvec::smallvec![damage; 5],
                    damage_type: crate::action::DamageType::Thorns,
                    is_modified: false,
                });
                // Reset counter to 5
                acts.push(crate::action::Action::UpdatePowerExtraData {
                    target: owner, power_id: PowerId::PanachePower, value: 5,
                });
            }
            acts
        },
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_at_turn_start(id: PowerId, _state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::FlameBarrier | PowerId::Rage => smallvec::smallvec![crate::action::Action::RemovePower { target: owner, power_id: id }],
        PowerId::Berserk => smallvec::smallvec![crate::action::Action::GainEnergy { amount }],
        PowerId::NextTurnBlock => core::next_turn_block::at_turn_start(owner, amount),
        PowerId::Energized => core::energized::at_turn_start(owner, amount),
        PowerId::MagnetismPower => {
            // Add `amount` random colorless cards to hand
            let mut acts = smallvec::SmallVec::new();
            for _ in 0..amount {
                acts.push(crate::action::Action::MakeRandomColorlessCardInHand {
                    rarity: crate::content::cards::CardRarity::Uncommon,
                    cost_for_turn: None,
                });
            }
            acts
        },
        PowerId::MayhemPower => {
            // Play top card of draw pile `amount` times
            let mut acts = smallvec::SmallVec::new();
            for _ in 0..amount {
                acts.push(crate::action::Action::PlayTopCard { target: None, exhaust: false });
            }
            acts
        },
        PowerId::PanachePower => {
            // Reset counter to 5 at start of turn
            smallvec::smallvec![crate::action::Action::UpdatePowerExtraData {
                target: owner, power_id: PowerId::PanachePower, value: 5,
            }]
        },
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_post_draw(id: PowerId, _state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Brutality => ironclad::brutality::on_post_draw(owner, amount),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_at_end_of_turn(id: PowerId, _state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Combust => ironclad::combust::at_end_of_turn(_state, owner, amount),
        PowerId::Metallicize => ironclad::metallicize::at_end_of_turn(owner, amount),
        PowerId::Entangle => core::entangle::at_end_of_turn(owner),
        PowerId::Malleable => core::malleable::on_monster_turn_ended(_state, owner, amount),
        PowerId::PlatedArmor => core::plated_armor::on_monster_turn_ended(_state, owner, amount),
        PowerId::Thievery => core::thievery::on_monster_turn_ended(_state, owner, amount),
        PowerId::Explosive => core::explosive::at_end_of_turn(owner, amount),
        PowerId::Regrow => core::regrow::at_end_of_turn(owner, amount),
        PowerId::Constricted => core::constricted::at_end_of_turn(owner, amount),
        PowerId::Fading => core::fading::at_end_of_turn(owner, amount),
        PowerId::Shifting => core::shifting::at_end_of_turn(owner),
        PowerId::Slow => core::slow::at_end_of_turn(owner),
        PowerId::Intangible => core::intangible::at_end_of_turn(owner, amount),
        PowerId::TheBombPower => {
            // Countdown: reduce by 1, at amount==1 -> DamageAllEnemies then remove
            let mut acts = smallvec::SmallVec::new();
            if amount == 1 {
                // Explode! Deal damage (stored in extra_data) to all enemies
                // For simplicity, we use the card's base magic (40) stored as extra_data
                // Get extra_data for this power from power_db
                let extra = _state.power_db.get(&owner)
                    .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::TheBombPower))
                    .map(|p| p.extra_data)
                    .unwrap_or(40);
                acts.push(crate::action::Action::DamageAllEnemies {
                    source: owner,
                    damages: smallvec::smallvec![extra; 5],
                    damage_type: crate::action::DamageType::Thorns,
                    is_modified: false,
                });
                acts.push(crate::action::Action::RemovePower { target: owner, power_id: PowerId::TheBombPower });
            } else {
                acts.push(crate::action::Action::ApplyPower {
                    source: owner, target: owner, power_id: PowerId::TheBombPower, amount: -1,
                });
            }
            acts
        },
        PowerId::Regen => {
            // Java: RegenerateMonsterPower.atEndOfTurn() → HealAction(amount), ReducePower(1)
            let mut acts = smallvec::SmallVec::new();
            acts.push(crate::action::Action::Heal { target: owner, amount });
            acts.push(crate::action::Action::ApplyPower {
                source: owner, target: owner, power_id: PowerId::Regen, amount: -1,
            });
            acts
        },
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_at_end_of_round(id: PowerId, _state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Vulnerable => core::vulnerable::at_end_of_round(owner, amount),
        PowerId::Weak => core::weak::at_end_of_round(owner, amount),
        PowerId::Frail => core::frail::at_end_of_round(owner, amount),
        PowerId::Ritual => core::ritual::at_end_of_round(_state, owner, amount),
        // Other Powers that decay at end of round can be added here
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_card_drawn(id: PowerId, state: &CombatState, owner: crate::core::EntityId, amount: i32, card_uuid: u32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    if let Some(card) = state.hand.iter().find(|c| c.uuid == card_uuid) {
        match id {
            PowerId::Evolve => ironclad::evolve::on_card_drawn(card.id, amount),
            PowerId::FireBreathing => ironclad::fire_breathing::on_card_drawn(owner, card.id, amount),
            _ => smallvec::smallvec![]
        }
    } else {
        smallvec::smallvec![]
    }
}

// -------------------------------------------------------------
// On Attack Modifiers
// -------------------------------------------------------------

pub fn resolve_power_on_inflict_damage(
    id: PowerId,
    damage: i32,
    damage_type: crate::action::DamageType,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::PainfulStabs => core::painful_stabs::on_inflict_damage(damage, damage_type),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_block_gained(id: PowerId, _state: &CombatState, _owner: crate::core::EntityId, amount: i32, _block_amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Juggernaut => ironclad::juggernaut::on_block_gained(amount),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_attacked(id: PowerId, state: &CombatState, owner: crate::core::EntityId, damage: i32, source: crate::core::EntityId, power_amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::FlameBarrier => ironclad::flame_barrier::on_attacked(source, power_amount),
        PowerId::CurlUp => core::curl_up::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Angry => core::angry::on_attacked(state, owner, damage, source, power_amount),
        // SharpHide: moved from on_attacked to on_card_played (Java uses onUseCard, not onAttacked)
        PowerId::Flight => core::flight::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Malleable => core::malleable::on_attacked(state, owner, damage, power_amount),
        PowerId::PlatedArmor => core::plated_armor::on_attacked(state, owner, damage, power_amount),
        PowerId::Thorns => core::thorns::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Shifting => core::shifting::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Reactive => core::reactive::on_attacked(state, owner, damage, source, power_amount),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_death(id: PowerId, state: &CombatState, owner: crate::core::EntityId, amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::SporeCloud => core::spore_cloud::on_death(state, owner, amount),
        PowerId::Stasis => core::stasis::on_death(owner, amount),
        PowerId::Unawakened => core::unawakened::on_death(owner, amount),
        PowerId::Shackled => smallvec::smallvec![],
        PowerId::DrawReduction => smallvec::smallvec![],
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_remove(id: PowerId, state: &CombatState, owner: crate::core::EntityId) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::PlatedArmor => core::plated_armor::on_remove(state, owner),
        _ => smallvec::smallvec![]
    }
}

pub fn resolve_power_on_attack_to_change_damage(
    id: PowerId, _state: &CombatState, _info: &crate::action::DamageInfo, current_damage: i32, amount: i32
) -> i32 {
    match id {
        PowerId::Strength => core::strength::on_attack_to_change_damage(current_damage, amount),
        _ => current_damage
    }
}

pub fn resolve_power_on_attacked_to_change_damage(
    id: PowerId, _state: &CombatState, _info: &crate::action::DamageInfo, current_damage: i32, amount: i32
) -> i32 {
    match id {
        PowerId::Vulnerable => {
            let has_odd_mushroom = crate::content::relics::hooks::on_calculate_vulnerable_multiplier(_state);
            core::vulnerable::on_attacked_to_change_damage(current_damage, amount, has_odd_mushroom)
        },
        _ => current_damage
    }
}

pub fn resolve_power_on_calculate_damage_to_enemy(
    id: PowerId, _state: &CombatState, card: &CombatCard, damage: f32, amount: i32
) -> f32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Attack {
        match id {
            PowerId::Strength => core::strength::on_calculate_damage_to_enemy(card.id, card.base_magic_num_mut, damage, amount),
            PowerId::LoseStrength => core::lose_strength::on_calculate_damage_to_enemy(card.id, card.base_magic_num_mut, damage, amount),
            PowerId::PenNibPower => core::pen_nib::on_calculate_damage_to_enemy(damage),
            PowerId::Weak => damage * 0.75,
            // Java: atDamageGive — Vigor adds its amount to NORMAL attack damage
            PowerId::Vigor => damage + amount as f32,
            _ => damage
        }
    } else {
        damage
    }
}

pub fn resolve_power_on_calculate_block(
    id: PowerId, _state: &CombatState, card: &CombatCard, block: f32, amount: i32
) -> f32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.base_block > 0 || def.card_type == crate::content::cards::CardType::Skill {
        match id {
            PowerId::Dexterity => core::dexterity::on_calculate_block(block, amount),
            PowerId::Frail => core::frail::on_calculate_block(block, amount),
            _ => block
        }
    } else {
        block
    }
}

pub fn resolve_power_on_calculate_damage_from_player(
    id: PowerId, _state: &CombatState, card: &CombatCard, _target_id: crate::core::EntityId, damage: f32, amount: i32
) -> f32 {
    let mut updated_damage = damage;
    let def = crate::content::cards::get_card_definition(card.id);
    
    // Slow modifies damage dynamically regardless of attack type, but only on normal damage hits usually
    if def.card_type == crate::content::cards::CardType::Attack {
        updated_damage = match id {
            PowerId::Vulnerable => {
                let has_odd_mushroom = crate::content::relics::hooks::on_calculate_vulnerable_multiplier(_state);
                core::vulnerable::on_calculate_damage_from_player(updated_damage, amount, has_odd_mushroom)
            },
            PowerId::Slow => core::slow::on_calculate_damage_from_player(_state, card, _target_id, updated_damage, amount),
            PowerId::Flight => core::flight::on_calculate_damage_from_player(updated_damage, amount),
            _ => updated_damage
        };
    }
    
    updated_damage
}

/// Java: AbstractPower.canPlayCard(AbstractCard) — returns false to block card play.
/// Called in AbstractCard.hasEnoughEnergy() for each power on the player.
/// Currently only NoSkillsPower (Watcher) overrides this in Java.
pub fn resolve_power_can_play_card(
    id: PowerId, card: &CombatCard,
) -> bool {
    match id {
        PowerId::NoSkills => {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type != crate::content::cards::CardType::Skill
        },
        _ => true,
    }
}

/// Java: AbstractPower.onApplyPower(power, target, source)
/// Called on each power of the SOURCE entity when a power is applied.
/// Call site: ApplyPowerAction.update() — after the power is applied to target.
/// Only override: SadisticPower — deal THORNS damage when owner applies a debuff.
pub fn resolve_power_on_apply_power(
    id: PowerId,
    owner_amount: i32,
    applied_power_id: PowerId,
    applied_amount: i32,
    target: usize,
    source: usize,
    state: &CombatState,
) -> smallvec::SmallVec<[crate::action::ActionInfo; 2]> {
    let mut actions = smallvec::SmallVec::new();
    match id {
        PowerId::SadisticPower => {
            // Java SadisticPower.onApplyPower:
            //   if (power.type == DEBUFF && !power.ID.equals("Shackled")
            //       && source == this.owner && target != this.owner
            //       && !target.hasPower("Artifact"))
            //     → addToBot(DamageAction(target, amount, THORNS))
            if is_debuff(applied_power_id, applied_amount)
                // Java excludes "Shackled" (GainStrengthPower) — TODO: add PowerId::GainStrength
                && target != source
                && !state.power_db.get(&target).map_or(false, |powers| {
                    powers.iter().any(|p| p.power_type == PowerId::Artifact)
                })
            {
                actions.push(crate::action::ActionInfo {
                    action: crate::action::Action::Damage(crate::action::DamageInfo {
                        source,
                        target,
                        base: owner_amount,
                        output: owner_amount,
                        damage_type: crate::action::DamageType::Thorns,
                        is_modified: true,
                    }),
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            }
        },
        _ => {},
    }
    actions
}

/// Java: AbstractPower.atDamageFinalReceive(damage, damageType)
/// Used exclusively for Intangible and Flight.
pub fn resolve_power_at_damage_final_receive(
    id: PowerId,
    damage: i32,
    amount: i32,
    damage_type: crate::action::DamageType,
) -> i32 {
    match id {
        PowerId::Intangible => core::intangible::at_damage_final_receive(damage, amount, damage_type),
        PowerId::Flight => core::flight::at_damage_final_receive(damage, amount, damage_type),
        _ => damage,
    }
}
