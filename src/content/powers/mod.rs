pub mod core;
pub mod ironclad;
pub mod silent;
pub mod store;

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
    NoBlock,
    Regen,
    Regeneration,
    Rage,
    Berserk,
    Angry,
    Anger,
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
    IntangiblePlayer,
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
    NoxiousFumes,
    AfterImage,
    Burst,
}

use crate::combat::{CombatCard, CombatState};

pub struct PowerDefinition {
    pub id: PowerId,
    pub name: &'static str,
}

/// Java powers whose runtime amount is a sentinel `-1` rather than a stack count.
pub fn uses_sentinel_amount(id: PowerId) -> bool {
    matches!(
        id,
        PowerId::NoDraw
            | PowerId::Minion
            | PowerId::Corruption
            | PowerId::Split
            | PowerId::Barricade
            | PowerId::Stasis
            | PowerId::BackAttack
            | PowerId::PainfulStabs
            | PowerId::Regrow
            | PowerId::Surrounded
            | PowerId::Unawakened
            | PowerId::Confusion
    )
}

pub fn uses_distinct_instances(id: PowerId) -> bool {
    matches!(id, PowerId::TheBombPower)
}

pub fn canonicalize_applied_amount(id: PowerId, amount: i32) -> i32 {
    if uses_sentinel_amount(id) {
        -1
    } else {
        amount
    }
}

pub fn allows_negative_amount(id: PowerId) -> bool {
    matches!(id, PowerId::Strength | PowerId::Dexterity | PowerId::Focus)
        || uses_sentinel_amount(id)
}

pub fn should_keep_power_instance(id: PowerId, amount: i32) -> bool {
    if id == PowerId::Slow {
        return amount >= 0;
    }
    if id == PowerId::TimeWarp {
        return true;
    }
    if amount > 0 {
        return true;
    }
    if amount < 0 {
        return allows_negative_amount(id);
    }
    false
}

/// Java: AbstractPower.PowerType — BUFF or DEBUFF.
/// Some powers are dynamic: Strength/Focus/Dexterity are DEBUFF when amount < 0.
/// Used by: SadisticPower.onApplyPower, RemoveDebuffsAction, Artifact interception.
pub fn is_debuff(id: PowerId, amount: i32) -> bool {
    match id {
        // Dynamic: DEBUFF when amount < 0
        PowerId::Strength | PowerId::Focus | PowerId::Dexterity => amount < 0,
        // Fixed DEBUFFs (from Java constructor: this.type = PowerType.DEBUFF)
        PowerId::Vulnerable
        | PowerId::Weak
        | PowerId::Frail
        | PowerId::Poison
        | PowerId::Entangle
        | PowerId::NoDraw
        | PowerId::NoBlock
        | PowerId::Constricted
        | PowerId::Confusion
        | PowerId::Hex
        | PowerId::Slow
        | PowerId::LoseStrength
        | PowerId::DexterityDown
        | PowerId::NoSkills
        | PowerId::Fading
        | PowerId::Shackled
        | PowerId::DrawReduction => true,
        // Everything else is BUFF
        _ => false,
    }
}

/// Returns true only when this ApplyPower is making the target meaningfully more debuffed.
///
/// This is narrower than `is_debuff(...)`:
/// - fixed debuffs like Weak/Vulnerable are only debuff applications when `amount > 0`
/// - dynamic debuffs like negative Strength are only debuff applications when `amount < 0`
///
/// End-of-round cleanup like `Weak -1` should not be blocked by Artifact or trigger
/// "on apply debuff" hooks.
pub fn is_debuff_application(id: PowerId, amount: i32) -> bool {
    match id {
        PowerId::Strength | PowerId::Focus | PowerId::Dexterity => amount < 0,
        // Some Java debuffs use sentinel amounts instead of stacks. Artifact still
        // blocks the application even when the constructor amount is -1.
        PowerId::NoDraw | PowerId::Confusion => amount != 0,
        PowerId::NoBlock => amount > 0,
        PowerId::Vulnerable
        | PowerId::Weak
        | PowerId::Frail
        | PowerId::Poison
        | PowerId::Entangle
        | PowerId::Constricted
        | PowerId::Hex
        | PowerId::Slow
        | PowerId::LoseStrength
        | PowerId::DexterityDown
        | PowerId::NoSkills
        | PowerId::Fading
        | PowerId::Shackled
        | PowerId::DrawReduction => amount > 0,
        _ => false,
    }
}

pub fn get_power_definition(id: PowerId) -> PowerDefinition {
    match id {
        PowerId::Strength => PowerDefinition {
            id,
            name: "Strength",
        },
        PowerId::Vulnerable => PowerDefinition {
            id,
            name: "Vulnerable",
        },
        PowerId::Weak => PowerDefinition { id, name: "Weak" },
        PowerId::Frail => PowerDefinition { id, name: "Frail" },
        PowerId::Dexterity => PowerDefinition {
            id,
            name: "Dexterity",
        },
        PowerId::Ritual => PowerDefinition { id, name: "Ritual" },
        PowerId::Poison => PowerDefinition { id, name: "Poison" },
        PowerId::LoseStrength => PowerDefinition {
            id,
            name: "Lose Strength",
        },
        PowerId::DemonForm => PowerDefinition {
            id,
            name: "Demon Form",
        },
        PowerId::Corruption => PowerDefinition {
            id,
            name: "Corruption",
        },
        PowerId::DoubleTap => PowerDefinition {
            id,
            name: "Double Tap",
        },
        PowerId::DuplicationPower => PowerDefinition {
            id,
            name: "Duplication",
        },
        PowerId::FeelNoPain => PowerDefinition {
            id,
            name: "Feel No Pain",
        },
        PowerId::DarkEmbrace => PowerDefinition {
            id,
            name: "Dark Embrace",
        },
        PowerId::Minion => PowerDefinition { id, name: "Minion" },
        PowerId::Rupture => PowerDefinition {
            id,
            name: "Rupture",
        },
        PowerId::Combust => PowerDefinition {
            id,
            name: "Combust",
        },
        PowerId::Brutality => PowerDefinition {
            id,
            name: "Brutality",
        },
        PowerId::Barricade => PowerDefinition {
            id,
            name: "Barricade",
        },
        PowerId::Juggernaut => PowerDefinition {
            id,
            name: "Juggernaut",
        },
        PowerId::FlameBarrier => PowerDefinition {
            id,
            name: "Flame Barrier",
        },
        PowerId::Metallicize => PowerDefinition {
            id,
            name: "Metallicize",
        },
        PowerId::CurlUp => PowerDefinition {
            id,
            name: "Curl Up",
        },
        PowerId::SporeCloud => PowerDefinition {
            id,
            name: "Spore Cloud",
        },
        PowerId::Split => PowerDefinition { id, name: "Split" },
        PowerId::GenericStrengthUp => PowerDefinition {
            id,
            name: "Generic Strength Up",
        },
        PowerId::Entangle => PowerDefinition {
            id,
            name: "Entangled",
        },
        PowerId::Evolve => PowerDefinition { id, name: "Evolve" },
        PowerId::FireBreathing => PowerDefinition {
            id,
            name: "Fire Breathing",
        },
        PowerId::NoDraw => PowerDefinition {
            id,
            name: "No Draw",
        },
        PowerId::Regen => PowerDefinition {
            id,
            name: "Regenerate",
        },
        PowerId::Regeneration => PowerDefinition {
            id,
            name: "Regeneration",
        },
        PowerId::Rage => PowerDefinition { id, name: "Rage" },
        PowerId::Berserk => PowerDefinition {
            id,
            name: "Berserk",
        },
        PowerId::Angry => PowerDefinition { id, name: "Angry" },
        PowerId::Anger => PowerDefinition { id, name: "Anger" },
        // Java: Mode Shift is a display/state power. Guardian-specific behavior lives on the monster.
        PowerId::ModeShift => PowerDefinition {
            id,
            name: "Mode Shift",
        },
        PowerId::SharpHide => PowerDefinition {
            id,
            name: "Sharp Hide",
        },
        PowerId::Artifact => PowerDefinition {
            id,
            name: "Artifact",
        },
        // Internal Rust-only tracker for The Guardian's next threshold; not exported by Java snapshots.
        PowerId::GuardianThreshold => PowerDefinition {
            id,
            name: "Guardian Threshold",
        },
        PowerId::Flight => PowerDefinition { id, name: "Flight" },
        PowerId::Hex => PowerDefinition { id, name: "Hex" },
        PowerId::Malleable => PowerDefinition {
            id,
            name: "Malleable",
        },
        PowerId::PlatedArmor => PowerDefinition {
            id,
            name: "Plated Armor",
        },
        PowerId::Confusion => PowerDefinition {
            id,
            name: "Confusion",
        },
        PowerId::Thievery => PowerDefinition {
            id,
            name: "Thievery",
        },
        PowerId::PainfulStabs => PowerDefinition {
            id,
            name: "Painful Stabs",
        },
        PowerId::Stasis => PowerDefinition { id, name: "Stasis" },
        PowerId::Explosive => PowerDefinition {
            id,
            name: "Explosive",
        },
        PowerId::Thorns => PowerDefinition { id, name: "Thorns" },
        PowerId::Regrow => PowerDefinition {
            id,
            name: "Life Link",
        },
        PowerId::Constricted => PowerDefinition {
            id,
            name: "Constricted",
        },
        PowerId::Fading => PowerDefinition { id, name: "Fading" },
        PowerId::Shifting => PowerDefinition {
            id,
            name: "Shifting",
        },
        PowerId::Reactive => PowerDefinition {
            id,
            name: "Reactive",
        },
        PowerId::Slow => PowerDefinition { id, name: "Slow" },
        PowerId::Intangible => PowerDefinition {
            id,
            name: "Intangible",
        },
        PowerId::IntangiblePlayer => PowerDefinition {
            id,
            name: "Intangible",
        },
        PowerId::Curiosity => PowerDefinition {
            id,
            name: "Curiosity",
        },
        PowerId::Unawakened => PowerDefinition {
            id,
            name: "Unawakened",
        },
        PowerId::NoBlock => PowerDefinition {
            id,
            name: "No Block",
        },
        PowerId::Shackled => PowerDefinition {
            id,
            name: "Shackled",
        },
        PowerId::DrawReduction => PowerDefinition {
            id: PowerId::DrawReduction,
            name: "Draw Reduction",
        },
        PowerId::Surrounded => PowerDefinition {
            id: PowerId::Surrounded,
            name: "Surrounded",
        },
        PowerId::BackAttack => PowerDefinition {
            id: PowerId::BackAttack,
            name: "Back Attack",
        },
        PowerId::TimeWarp => PowerDefinition {
            id,
            name: "Time Warp",
        },
        PowerId::Invincible => PowerDefinition {
            id,
            name: "Invincible",
        },
        PowerId::BeatOfDeath => PowerDefinition {
            id,
            name: "Beat of Death",
        },
        PowerId::Vigor => PowerDefinition { id, name: "Vigor" },
        PowerId::Mantra => PowerDefinition { id, name: "Mantra" },
        PowerId::Focus => PowerDefinition { id, name: "Focus" },
        PowerId::DexterityDown => PowerDefinition {
            id,
            name: "Dexterity Down",
        },
        PowerId::PenNibPower => PowerDefinition {
            id,
            name: "Pen Nib",
        },
        PowerId::NextTurnBlock => PowerDefinition {
            id,
            name: "Next Turn Block",
        },
        PowerId::Energized => PowerDefinition {
            id,
            name: "Energized",
        },
        PowerId::MagnetismPower => PowerDefinition {
            id,
            name: "Magnetism",
        },
        PowerId::MayhemPower => PowerDefinition { id, name: "Mayhem" },
        PowerId::PanachePower => PowerDefinition {
            id,
            name: "Panache",
        },
        PowerId::SadisticPower => PowerDefinition {
            id,
            name: "Sadistic Nature",
        },
        PowerId::TheBombPower => PowerDefinition {
            id,
            name: "The Bomb",
        },
        PowerId::Buffer => PowerDefinition { id, name: "Buffer" },
        PowerId::Electro => PowerDefinition {
            id,
            name: "Electrodynamics",
        },
        PowerId::NoSkills => PowerDefinition {
            id,
            name: "No Skills",
        },
        PowerId::ThousandCuts => PowerDefinition {
            id,
            name: "A Thousand Cuts",
        },
        PowerId::NoxiousFumes => PowerDefinition {
            id,
            name: "Noxious Fumes",
        },
        PowerId::AfterImage => PowerDefinition {
            id,
            name: "After Image",
        },
        PowerId::Burst => PowerDefinition { id, name: "Burst" },
    }
}

pub fn power_removes_at_zero(id: PowerId) -> bool {
    match id {
        PowerId::Strength | PowerId::Dexterity => false,
        _ => true,
    }
}

// Global Power Hook Routers
pub fn resolve_power_on_apply(
    id: PowerId,
    state: &mut CombatState,
    _target: crate::core::EntityId,
) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_apply(state),
        _ => {}
    }
}

pub fn resolve_power_on_card_draw(id: PowerId, state: &mut CombatState, card: &mut CombatCard) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_card_draw(state, card),
        PowerId::Confusion => core::confusion::on_card_draw(state, card),
        _ => {}
    }
}

pub fn resolve_power_on_use_card(
    id: PowerId,
    state: &mut CombatState,
    card: &CombatCard,
    exhaust_override: &mut bool,
    purge: bool,
    target: Option<usize>,
) {
    match id {
        PowerId::Corruption => ironclad::corruption::on_use_card(state, card, exhaust_override),
        PowerId::DoubleTap => ironclad::double_tap::on_use_card(state, card, purge, target),
        PowerId::DuplicationPower => {
            core::duplication_power::on_use_card(state, card, purge, target)
        }
        PowerId::Burst => silent::burst::on_use_card(state, card, purge, target),
        PowerId::PenNibPower => {
            let actions = core::pen_nib::on_use_card(card);
            for action in actions {
                state.engine.action_queue.push_back(action);
            }
        }
        PowerId::Vigor => {
            // Java: onUseCard — remove Vigor when an attack card is played
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Attack {
                state
                    .engine
                    .action_queue
                    .push_back(crate::action::Action::RemovePower {
                        target: 0, // Player
                        power_id: PowerId::Vigor,
                    });
            }
        }
        _ => {}
    }
}

pub fn resolve_power_on_player_card_played(
    id: PowerId,
    owner: crate::core::EntityId,
    amount: i32,
    card: &CombatCard,
    state: &CombatState,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Curiosity => core::curiosity::on_player_card_played(owner, amount, card),
        PowerId::TimeWarp => core::time_warp::on_player_card_played(owner, amount, card, state),
        PowerId::BeatOfDeath => core::beat_of_death::on_player_card_played(owner, amount, card),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_exhaust(
    id: PowerId,
    _state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
    _card_uuid: u32,
    _card_id: crate::content::cards::CardId,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::FeelNoPain => ironclad::feel_no_pain::on_exhaust(owner, amount),
        PowerId::DarkEmbrace => ironclad::dark_embrace::on_exhaust(amount),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_hp_lost(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
    source: Option<crate::core::EntityId>,
    damage_type: crate::action::DamageType,
    triggers_rupture: bool,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Rupture if triggers_rupture => ironclad::rupture::on_hp_lost(amount),
        PowerId::Split => core::split::on_hp_lost(state, owner, amount),
        PowerId::PlatedArmor => {
            core::plated_armor::on_hp_lost(state, owner, amount, source, damage_type)
        }
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_card_played(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
    card: &CombatCard,
    power_amount: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        // Java Gremlin Nob Enrage = AngerPower.onUseCard(SKILL).
        PowerId::Anger => core::anger::on_card_played(state, owner, card, power_amount),
        PowerId::BeatOfDeath => {
            core::beat_of_death::on_player_card_played(owner, power_amount, card)
        }
        PowerId::Rage => core::rage::on_card_played(owner, card, power_amount),
        PowerId::SharpHide => core::sharp_hide::on_card_played(state, owner, card, power_amount),
        PowerId::Hex => core::hex::on_card_played(state, owner, card, power_amount),
        PowerId::ThousandCuts => {
            silent::thousand_cuts::on_after_card_played(state, owner, power_amount)
        }
        PowerId::AfterImage => silent::after_image::on_card_played(owner, power_amount, card),
        PowerId::Curiosity => core::curiosity::on_player_card_played(owner, power_amount, card),
        PowerId::TimeWarp => {
            core::time_warp::on_player_card_played(owner, power_amount, card, state)
        }
        PowerId::Slow => core::slow::on_card_played(owner),
        PowerId::PanachePower => {
            let mut acts = smallvec::SmallVec::new();
            if power_amount == 1 {
                let damage = crate::content::powers::store::powers_for(state, owner)
                    .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::PanachePower))
                    .map(|p| p.extra_data)
                    .unwrap_or(10);
                acts.push(crate::action::Action::DamageAllEnemies {
                    source: owner,
                    damages: crate::action::repeated_damage_matrix(
                        state.entities.monsters.len(),
                        damage,
                    ),
                    damage_type: crate::action::DamageType::Thorns,
                    is_modified: false,
                });
                acts.push(crate::action::Action::ApplyPower {
                    source: owner,
                    target: owner,
                    power_id: PowerId::PanachePower,
                    amount: 4,
                });
            } else {
                acts.push(crate::action::Action::ApplyPower {
                    source: owner,
                    target: owner,
                    power_id: PowerId::PanachePower,
                    amount: -1,
                });
            }
            acts
        }
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_at_turn_start(
    id: PowerId,
    _state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::FlameBarrier | PowerId::Rage => {
            smallvec::smallvec![crate::action::Action::RemovePower {
                target: owner,
                power_id: id
            }]
        }
        PowerId::Berserk => smallvec::smallvec![crate::action::Action::GainEnergy { amount }],
        PowerId::NextTurnBlock => core::next_turn_block::at_turn_start(owner, amount),
        PowerId::Energized => core::energized::at_turn_start(owner, amount),
        PowerId::MagnetismPower => {
            // Add `amount` random colorless cards to hand
            let mut acts = smallvec::SmallVec::new();
            for _ in 0..amount {
                acts.push(crate::action::Action::MakeRandomColorlessCardInHand {
                    cost_for_turn: None,
                    upgraded: false,
                });
            }
            acts
        }
        PowerId::MayhemPower => {
            // Play top card of draw pile `amount` times
            let mut acts = smallvec::SmallVec::new();
            for _ in 0..amount {
                acts.push(crate::action::Action::PlayTopCard {
                    target: None,
                    exhaust: false,
                });
            }
            acts
        }
        PowerId::PanachePower => {
            if amount == 5 {
                smallvec::smallvec![]
            } else {
                smallvec::smallvec![crate::action::Action::ApplyPower {
                    source: owner,
                    target: owner,
                    power_id: PowerId::PanachePower,
                    amount: 5 - amount,
                }]
            }
        }
        PowerId::Poison => core::poison::at_turn_start(owner, amount),
        PowerId::Flight => core::flight::at_turn_start(_state, owner, amount),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_post_draw(
    id: PowerId,
    _state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Brutality => ironclad::brutality::on_post_draw(owner, amount),
        PowerId::DemonForm => ironclad::demon_form::on_post_draw(owner, amount),
        PowerId::NoxiousFumes => silent::noxious_fumes::on_post_draw(_state, owner, amount),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_at_end_of_turn(
    power: &crate::combat::Power,
    _state: &CombatState,
    owner: crate::core::EntityId,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    let id = power.power_type;
    let amount = power.amount;
    match id {
        PowerId::DoubleTap => smallvec::smallvec![crate::action::Action::RemovePower {
            target: owner,
            power_id: PowerId::DoubleTap,
        }],
        PowerId::Burst => smallvec::smallvec![crate::action::Action::RemovePower {
            target: owner,
            power_id: PowerId::Burst,
        }],
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
        PowerId::LoseStrength => core::lose_strength::at_end_of_turn(owner, amount),
        PowerId::DexterityDown => core::dexterity_down::at_end_of_turn(owner, amount),
        PowerId::NoDraw => core::no_draw::at_end_of_turn(owner),
        PowerId::Ritual => core::ritual::at_end_of_turn(owner, amount, power.extra_data),
        PowerId::Shackled => core::shackled::at_end_of_turn(owner, amount),
        PowerId::Shifting => core::shifting::at_end_of_turn(owner),
        PowerId::Intangible => core::intangible::at_end_of_turn(owner, amount),
        PowerId::TheBombPower => {
            let mut acts = smallvec::SmallVec::new();
            if amount == 1 {
                if let Some(instance_id) = power.instance_id {
                    acts.push(crate::action::Action::ReducePowerInstance {
                        target: owner,
                        power_id: PowerId::TheBombPower,
                        instance_id,
                        amount: 1,
                    });
                } else {
                    acts.push(crate::action::Action::ReducePower {
                        target: owner,
                        power_id: PowerId::TheBombPower,
                        amount: 1,
                    });
                }
                acts.push(crate::action::Action::DamageAllEnemies {
                    source: owner,
                    damages: crate::action::repeated_damage_matrix(
                        _state.entities.monsters.len(),
                        power.extra_data.max(0),
                    ),
                    damage_type: crate::action::DamageType::Thorns,
                    is_modified: false,
                });
            } else {
                if let Some(instance_id) = power.instance_id {
                    acts.push(crate::action::Action::ReducePowerInstance {
                        target: owner,
                        power_id: PowerId::TheBombPower,
                        instance_id,
                        amount: 1,
                    });
                } else {
                    acts.push(crate::action::Action::ReducePower {
                        target: owner,
                        power_id: PowerId::TheBombPower,
                        amount: 1,
                    });
                }
            }
            acts
        }
        PowerId::Regen => {
            // Java: RegenerateMonsterPower.atEndOfTurn() only heals; it does not tick down.
            let mut acts = smallvec::SmallVec::new();
            acts.push(crate::action::Action::Heal {
                target: owner,
                amount,
            });
            acts
        }
        PowerId::Regeneration => {
            // Java: RegenPower.atEndOfTurn() -> RegenAction(heal), then decrement/remove Regeneration.
            let mut acts = smallvec::SmallVec::new();
            acts.push(crate::action::Action::Heal {
                target: owner,
                amount,
            });
            acts.push(crate::action::Action::ApplyPower {
                source: owner,
                target: owner,
                power_id: PowerId::Regeneration,
                amount: -1,
            });
            acts
        }
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_at_end_of_round(
    id: PowerId,
    _state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
    just_applied: bool,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::DuplicationPower => {
            if amount <= 1 {
                smallvec::smallvec![crate::action::Action::RemovePower {
                    target: owner,
                    power_id: PowerId::DuplicationPower,
                }]
            } else {
                smallvec::smallvec![crate::action::Action::ApplyPower {
                    source: owner,
                    target: owner,
                    power_id: PowerId::DuplicationPower,
                    amount: -1,
                }]
            }
        }
        PowerId::Vulnerable => core::vulnerable::at_end_of_round(owner, amount, just_applied),
        PowerId::Weak => core::weak::at_end_of_round(owner, amount, just_applied),
        PowerId::Frail => core::frail::at_end_of_round(owner, amount, just_applied),
        PowerId::NoBlock => core::no_block::at_end_of_round(owner, amount, just_applied),
        PowerId::DrawReduction => {
            core::draw_reduction::at_end_of_round(owner, amount, just_applied)
        }
        PowerId::Slow => core::slow::at_end_of_round(owner, amount),
        PowerId::IntangiblePlayer => core::intangible::at_end_of_round(owner, amount),
        PowerId::Ritual => core::ritual::at_end_of_round(_state, owner, amount),
        PowerId::GenericStrengthUp => core::generic_strength_up::at_end_of_round(owner, amount),
        // Other Powers that decay at end of round can be added here
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_card_drawn(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
    card_uuid: u32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    if let Some(card) = state.zones.hand.iter().find(|c| c.uuid == card_uuid) {
        match id {
            PowerId::Evolve => ironclad::evolve::on_card_drawn(card.id, amount),
            PowerId::FireBreathing => {
                ironclad::fire_breathing::on_card_drawn(state, owner, card.id, amount)
            }
            _ => smallvec::smallvec![],
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

pub fn resolve_power_on_block_gained(
    id: PowerId,
    _state: &CombatState,
    _owner: crate::core::EntityId,
    amount: i32,
    _block_amount: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Juggernaut => ironclad::juggernaut::on_block_gained(amount),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_attacked(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
    damage: i32,
    source: crate::core::EntityId,
    power_amount: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Angry => core::angry::on_attacked(state, owner, damage, source, power_amount),
        PowerId::FlameBarrier => ironclad::flame_barrier::on_attacked(source, power_amount),
        PowerId::CurlUp => core::curl_up::on_attacked(state, owner, damage, source, power_amount),
        // SharpHide: moved from on_attacked to on_card_played (Java uses onUseCard, not onAttacked)
        PowerId::Flight => core::flight::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Malleable => core::malleable::on_attacked(state, owner, damage, power_amount),
        PowerId::Thorns => core::thorns::on_attacked(state, owner, damage, source, power_amount),
        PowerId::Shifting => {
            core::shifting::on_attacked(state, owner, damage, source, power_amount)
        }
        PowerId::Reactive => {
            core::reactive::on_attacked(state, owner, damage, source, power_amount)
        }
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_death(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
    extra_data: i32,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::SporeCloud => core::spore_cloud::on_death(state, owner, amount),
        PowerId::Stasis => core::stasis::on_death(state, owner, extra_data),
        PowerId::Unawakened => core::unawakened::on_death(owner, amount),
        PowerId::Shackled => smallvec::smallvec![],
        PowerId::DrawReduction => smallvec::smallvec![],
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_remove(
    id: PowerId,
    state: &CombatState,
    owner: crate::core::EntityId,
) -> smallvec::SmallVec<[crate::action::Action; 2]> {
    match id {
        PowerId::Flight => core::flight::on_remove(state, owner),
        PowerId::PlatedArmor => core::plated_armor::on_remove(state, owner),
        _ => smallvec::smallvec![],
    }
}

pub fn resolve_power_on_attack_to_change_damage(
    id: PowerId,
    _state: &CombatState,
    _info: &crate::action::DamageInfo,
    current_damage: i32,
    amount: i32,
) -> i32 {
    match id {
        PowerId::Strength => core::strength::on_attack_to_change_damage(current_damage, amount),
        _ => current_damage,
    }
}

pub fn resolve_power_on_attacked_to_change_damage(
    id: PowerId,
    state: &mut CombatState,
    info: &crate::action::DamageInfo,
    current_damage: i32,
    _amount: i32,
) -> i32 {
    match id {
        PowerId::Buffer => {
            if current_damage > 0 {
                let target = info.target;
                let Some(remaining) =
                    store::with_power_mut(state, target, PowerId::Buffer, |power| {
                        power.amount -= 1;
                        power.amount
                    })
                else {
                    return current_damage;
                };
                if !should_keep_power_instance(PowerId::Buffer, remaining) {
                    store::remove_power_type(state, target, PowerId::Buffer);
                }
                0
            } else {
                current_damage
            }
        }
        _ => current_damage,
    }
}

pub fn resolve_power_on_calculate_damage_to_enemy(
    id: PowerId,
    _state: &CombatState,
    card: &CombatCard,
    damage: f32,
    amount: i32,
) -> f32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Attack {
        match id {
            PowerId::Strength => core::strength::on_calculate_damage_to_enemy(
                card.id,
                card.base_magic_num_mut,
                damage,
                amount,
            ),
            PowerId::PenNibPower => core::pen_nib::on_calculate_damage_to_enemy(damage),
            PowerId::Weak => damage * 0.75,
            // Java: atDamageGive — Vigor adds its amount to NORMAL attack damage
            PowerId::Vigor => damage + amount as f32,
            _ => damage,
        }
    } else {
        damage
    }
}

pub fn resolve_power_on_calculate_block(
    id: PowerId,
    _state: &CombatState,
    card: &CombatCard,
    block: f32,
    amount: i32,
) -> f32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.base_block > 0 || def.card_type == crate::content::cards::CardType::Skill {
        match id {
            PowerId::Dexterity => core::dexterity::on_calculate_block(block, amount),
            PowerId::Frail => core::frail::on_calculate_block(block, amount),
            PowerId::NoBlock => core::no_block::on_calculate_block(block, amount),
            _ => block,
        }
    } else {
        block
    }
}

pub fn resolve_power_on_calculate_damage_from_player(
    id: PowerId,
    _state: &CombatState,
    card: &CombatCard,
    _target_id: crate::core::EntityId,
    damage: f32,
    amount: i32,
) -> f32 {
    let mut updated_damage = damage;
    let def = crate::content::cards::get_card_definition(card.id);

    // Slow modifies damage dynamically regardless of attack type, but only on normal damage hits usually
    if def.card_type == crate::content::cards::CardType::Attack {
        updated_damage = match id {
            PowerId::Vulnerable => {
                let multiplier = crate::content::relics::hooks::on_calculate_vulnerable_multiplier(
                    _state, false,
                );
                core::vulnerable::on_calculate_damage_from_player(
                    updated_damage,
                    amount,
                    multiplier,
                )
            }
            PowerId::Slow => core::slow::on_calculate_damage_from_player(
                _state,
                card,
                _target_id,
                updated_damage,
                amount,
            ),
            _ => updated_damage,
        };
    }

    updated_damage
}

/// Java: AbstractMonster.calculateDamage() pipeline
/// Evaluates the damage a monster will deal based on its base intent damage
/// after adjusting for Strength, Weak, Vulnerable, Intangible, etc.
pub fn calculate_monster_damage(
    base: i32,
    source_id: usize,
    target_id: usize,
    state: &CombatState,
) -> i32 {
    let mut tmp = base as f32;

    // 1. Monster powers (Strength, Weak, etc.) atDamageGive
    if let Some(owner_powers) = crate::content::powers::store::powers_for(state, source_id) {
        for p in owner_powers {
            tmp = match p.power_type {
                PowerId::Strength => {
                    core::strength::on_attack_to_change_damage(tmp as i32, p.amount) as f32
                } // Strength is linear addition
                PowerId::Weak => tmp * 0.75,
                _ => tmp,
            };
        }
    }

    // 2. Target powers (Vulnerable, Intangible, etc.) atDamageReceive
    if let Some(target_powers) = crate::content::powers::store::powers_for(state, target_id) {
        for p in target_powers {
            tmp = match p.power_type {
                PowerId::Vulnerable => {
                    let multiplier = if target_id == 0 {
                        crate::content::relics::hooks::on_calculate_vulnerable_multiplier(
                            state, true,
                        )
                    } else {
                        1.5
                    };
                    core::vulnerable::on_calculate_damage_from_player(tmp, p.amount, multiplier)
                    // Same logic for monster vs player vulnerability
                }
                _ => tmp,
            };
        }
    }

    // 3. Player stance atDamageReceive
    if target_id == 0 {
        if state.entities.player.stance == crate::combat::StanceId::Wrath {
            tmp *= 2.0;
        }
    }

    // 4. Target powers atDamageFinalReceive (Intangible)
    if let Some(target_powers) = crate::content::powers::store::powers_for(state, target_id) {
        for p in target_powers {
            tmp = match p.power_type {
                PowerId::Intangible | PowerId::IntangiblePlayer => {
                    core::intangible::at_damage_final_receive(
                        tmp as i32,
                        p.amount,
                        crate::action::DamageType::Normal,
                    ) as f32
                }
                PowerId::Flight => core::flight::at_damage_final_receive(
                    tmp as i32,
                    p.amount,
                    crate::action::DamageType::Normal,
                ) as f32,
                _ => tmp,
            };
        }
    }

    let dmg = tmp.floor() as i32;
    dmg.max(0)
}

/// Java: AbstractPower.canPlayCard(AbstractCard) — returns false to block card play.
/// Called in AbstractCard.hasEnoughEnergy() for each power on the player.
/// Currently only NoSkillsPower (Watcher) overrides this in Java.
pub fn resolve_power_can_play_card(id: PowerId, card: &CombatCard) -> bool {
    match id {
        PowerId::NoSkills => {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type != crate::content::cards::CardType::Skill
        }
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
            if is_debuff_application(applied_power_id, applied_amount)
                // Java excludes "Shackled" (GainStrengthPower) — TODO: add PowerId::GainStrength
                && target != source
                && !crate::content::powers::store::has_power(state, target, PowerId::Artifact)
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
        }
        _ => {}
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
        PowerId::Intangible | PowerId::IntangiblePlayer => {
            core::intangible::at_damage_final_receive(damage, amount, damage_type)
        }
        PowerId::Flight => core::flight::at_damage_final_receive(damage, amount, damage_type),
        _ => damage,
    }
}
