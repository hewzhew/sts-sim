// Mechanical split from cards/mod.rs for runtime card play/evaluation helpers.

use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardUseContext {
    /// Java `AbstractPlayer.useCard` calls `card.use(...)` before removing a
    /// normally played card from hand. Autoplay/direct paths can call the same
    /// card code while the card is already outside the hand.
    pub played_from_hand: bool,
}

/// Central dispatch table for resolving card play mechanics.
pub fn resolve_card_play(
    card_id: CardId,
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    resolve_card_play_with_context(card_id, state, card, target, CardUseContext::default())
}

pub fn resolve_card_play_with_context(
    card_id: CardId,
    _state: &CombatState,
    _card: &CombatCard,
    target: Option<EntityId>,
    context: CardUseContext,
) -> SmallVec<[ActionInfo; 4]> {
    let t = target;
    match card_id {
        CardId::Strike | CardId::StrikeG => ironclad::strike::strike_play(_state, _card, t),
        CardId::Bash => ironclad::bash::bash_play(_state, _card, t),
        CardId::Cleave => ironclad::cleave::cleave_play(_state, _card),
        CardId::IronWave => ironclad::iron_wave::iron_wave_play(_state, _card, t),
        CardId::PerfectedStrike => {
            ironclad::perfected_strike::perfected_strike_play(_state, _card, t)
        }
        CardId::TwinStrike => ironclad::twin_strike::twin_strike_play(_state, _card, t),
        CardId::ThunderClap => ironclad::thunderclap::thunderclap_play(_state, _card),
        CardId::Defend | CardId::DefendG => ironclad::defend::defend_play(_state, _card),
        CardId::Neutralize => silent::neutralize::neutralize_play(_state, _card, t),
        CardId::Survivor => silent::survivor::survivor_play(_state, _card),
        CardId::ShrugItOff => ironclad::shrug_it_off::shrug_it_off_play(_state, _card),
        CardId::Flex => ironclad::flex::flex_play(_state, _card),
        CardId::TrueGrit => ironclad::true_grit::true_grit_play(_state, _card),
        CardId::Inflame => ironclad::inflame::inflame_play(_state, _card),
        CardId::DemonForm => ironclad::demon_form::demon_form_play(_state, _card),
        CardId::Corruption => ironclad::corruption::corruption_play(_state, _card),
        CardId::HeavyBlade => ironclad::heavy_blade::heavy_blade_play(_state, _card, t),
        CardId::Whirlwind => ironclad::whirlwind::whirlwind_play(_state, _card),
        CardId::Bloodletting => ironclad::bloodletting::bloodletting_play(_state, _card),
        CardId::Offering => ironclad::offering::offering_play(_state, _card),
        CardId::SwordBoomerang => ironclad::sword_boomerang::sword_boomerang_play(_state, _card),
        CardId::Dropkick => ironclad::dropkick::dropkick_play(_state, _card, t),
        CardId::PommelStrike => ironclad::pommel_strike::pommel_strike_play(_state, _card, t),
        CardId::Headbutt => ironclad::headbutt::headbutt_play(_state, _card, t),
        CardId::Bludgeon => ironclad::bludgeon::bludgeon_play(_state, _card, t),
        CardId::DoubleTap => ironclad::double_tap::double_tap_play(_state, _card, t),
        CardId::FeelNoPain => ironclad::feel_no_pain::feel_no_pain_play(_state, _card),
        CardId::DarkEmbrace => ironclad::dark_embrace::dark_embrace_play(_state, _card),
        CardId::Sentinel => ironclad::sentinel::sentinel_play(_state, _card),
        CardId::FiendFire => ironclad::fiend_fire::fiend_fire_play(_state, _card, t),
        CardId::SeverSoul => ironclad::sever_soul::sever_soul_play(_state, _card, t),
        CardId::SecondWind => ironclad::second_wind::second_wind_play(_state, _card),
        CardId::Exhume => ironclad::exhume::exhume_play(_state, _card),
        CardId::BurningPact => ironclad::burning_pact::burning_pact_play(_state, _card),
        CardId::Reaper => ironclad::reaper::reaper_play(_state, _card),
        CardId::Feed => ironclad::feed::feed_play(_state, _card, t),
        CardId::BloodForBlood => ironclad::blood_for_blood::blood_for_blood_play(_state, _card, t),
        CardId::Rupture => ironclad::rupture::rupture_play(_state, _card),
        CardId::Hemokinesis => ironclad::hemokinesis::hemokinesis_play(_state, _card, t),
        CardId::Combust => ironclad::combust::combust_play(_state, _card),
        CardId::Brutality => ironclad::brutality::brutality_play(_state, _card),
        CardId::LimitBreak => ironclad::limit_break::limit_break_play(_state, _card),
        CardId::SpotWeakness => ironclad::spot_weakness::spot_weakness_play(_state, _card, t),
        CardId::Barricade => ironclad::barricade::barricade_play(_state, _card),
        CardId::Entrench => ironclad::entrench::entrench_play(_state, _card),
        CardId::Juggernaut => ironclad::juggernaut::juggernaut_play(_state, _card),
        CardId::FlameBarrier => ironclad::flame_barrier::flame_barrier_play(_state, _card),
        CardId::Metallicize => ironclad::metallicize::metallicize_play(_state, _card),
        CardId::GhostlyArmor => ironclad::ghostly_armor::ghostly_armor_play(_state, _card),
        CardId::Impervious => ironclad::impervious::impervious_play(_state, _card),
        CardId::PowerThrough => ironclad::power_through::power_through_play(_state, _card),
        CardId::Evolve => ironclad::evolve::evolve_play(_state, _card),
        CardId::FireBreathing => ironclad::fire_breathing::fire_breathing_play(_state, _card),
        CardId::Immolate => ironclad::immolate::immolate_play(_state, _card),
        CardId::WildStrike => ironclad::wild_strike::wild_strike_play(_state, _card, t),
        CardId::RecklessCharge => ironclad::reckless_charge::reckless_charge_play(_state, _card, t),
        CardId::Havoc => ironclad::havoc::havoc_play(_state, _card, t),
        CardId::Warcry => ironclad::warcry::warcry_play(_state, _card),
        CardId::BattleTrance => ironclad::battle_trance::battle_trance_play(_state, _card),
        CardId::Rampage => ironclad::rampage::rampage_play(_state, _card, t),
        CardId::SearingBlow => ironclad::searing_blow::searing_blow_play(_state, _card, t),
        CardId::Anger => ironclad::anger::anger_play(_state, _card, t),
        CardId::Armaments => ironclad::armaments::armaments_play(_state, _card),
        CardId::DualWield => ironclad::dual_wield::dual_wield_play(_state, _card),
        CardId::InfernalBlade => ironclad::infernal_blade::infernal_blade_play(_state, _card),
        CardId::SeeingRed => ironclad::seeing_red::seeing_red_play(_state, _card),
        CardId::Rage => ironclad::rage::rage_play(_state, _card),
        CardId::Berserk => ironclad::berserk::berserk_play(_state, _card),
        CardId::Shockwave => ironclad::shockwave::shockwave_play(_state, _card),
        CardId::Uppercut => ironclad::uppercut::uppercut_play(_state, _card, t),
        CardId::Clothesline => ironclad::clothesline::clothesline_play(_state, _card, t),
        CardId::Disarm => ironclad::disarm::disarm_play(_state, _card, t),
        CardId::Intimidate => ironclad::intimidate::intimidate_play(_state, _card),
        CardId::Carnage => ironclad::carnage::carnage_play(_state, _card, t),
        CardId::Clash => ironclad::clash::clash_play(_state, _card, t),
        CardId::BodySlam => ironclad::body_slam::body_slam_play(_state, _card, t),
        CardId::Pummel => ironclad::pummel::pummel_play(_state, _card, t),
        CardId::Miracle => {
            smallvec::smallvec![ActionInfo {
                action: crate::runtime::action::Action::GainEnergy { amount: 1 },
                insertion_mode: crate::runtime::action::AddTo::Bottom,
            }]
        }
        CardId::Shiv => {
            let mut actions = smallvec::SmallVec::new();
            if let Some(t) = t {
                let def = get_card_definition(CardId::Shiv);
                actions.push(ActionInfo {
                    action: crate::runtime::action::Action::Damage(
                        crate::runtime::action::DamageInfo {
                            source: _card.uuid as usize,
                            target: t,
                            base: def.base_damage,
                            output: _card.base_damage_mut,
                            damage_type: crate::runtime::action::DamageType::Normal,
                            is_modified: _card.base_damage_mut != def.base_damage,
                        },
                    ),
                    insertion_mode: crate::runtime::action::AddTo::Bottom,
                });
            }
            actions
        }
        CardId::Bite => colorless::play_colorless(_state, _card, t, context),
        CardId::Apparition => smallvec::smallvec![ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::IntangiblePlayer,
                amount: _card.base_magic_num_mut.max(1),
            },
            insertion_mode: crate::runtime::action::AddTo::Bottom,
        }],
        CardId::DeadlyPoison => silent::deadly_poison::deadly_poison_play(_state, _card, t),
        CardId::BouncingFlask => silent::bouncing_flask::bouncing_flask_play(_state, _card),
        CardId::Catalyst => silent::catalyst::catalyst_play(_state, _card, t),
        CardId::NoxiousFumes => silent::noxious_fumes::noxious_fumes_play(_state, _card),
        CardId::Footwork => silent::footwork::footwork_play(_state, _card),
        CardId::BladeDance => silent::blade_dance::blade_dance_play(_state, _card),
        CardId::CloakAndDagger => silent::cloak_and_dagger::cloak_and_dagger_play(_state, _card),
        CardId::Backflip => silent::backflip::backflip_play(_state, _card),
        CardId::Acrobatics => silent::acrobatics::acrobatics_play(_state, _card),
        CardId::Prepared => silent::prepared::prepared_play(_state, _card),
        CardId::DaggerThrow => silent::dagger_throw::dagger_throw_play(_state, _card, t),
        CardId::PoisonedStab => silent::poisoned_stab::poisoned_stab_play(_state, _card, t),
        CardId::DaggerSpray => silent::dagger_spray::dagger_spray_play(_state, _card),
        CardId::Deflect => silent::deflect::deflect_play(_state, _card),
        CardId::QuickSlash => silent::quick_slash::quick_slash_play(_state, _card, t),
        CardId::Slice => silent::slice::slice_play(_state, _card, t),
        CardId::FlyingKnee => silent::flying_knee::flying_knee_play(_state, _card, t),
        CardId::DodgeAndRoll => silent::dodge_and_roll::dodge_and_roll_play(_state, _card),
        CardId::SuckerPunch => silent::sucker_punch::sucker_punch_play(_state, _card, t),
        CardId::Outmaneuver => silent::outmaneuver::outmaneuver_play(_state, _card),
        CardId::SneakyStrike => silent::sneaky_strike::sneaky_strike_play(_state, _card, t),
        CardId::Dash => silent::dash::dash_play(_state, _card, t),
        CardId::Adrenaline => silent::adrenaline::adrenaline_play(_state, _card),
        CardId::AfterImage => silent::after_image::after_image_play(_state, _card),
        CardId::Burst => silent::burst::burst_play(_state, _card),
        CardId::Pride => smallvec::smallvec![], // Coast 1 but does nothing on play
        CardId::Finesse
        | CardId::BandageUp
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::DeepBreath
        | CardId::Discovery
        | CardId::DramaticEntrance
        | CardId::Enlightenment
        | CardId::FlashOfSteel
        | CardId::Forethought
        | CardId::GoodInstincts
        | CardId::Impatience
        | CardId::JackOfAllTrades
        | CardId::MindBlast
        | CardId::Panacea
        | CardId::PanicButton
        | CardId::Purity
        | CardId::SwiftStrike
        | CardId::Trip
        | CardId::Apotheosis
        | CardId::Chrysalis
        | CardId::HandOfGreed
        | CardId::Magnetism
        | CardId::MasterOfStrategy
        | CardId::Mayhem
        | CardId::Metamorphosis
        | CardId::Panache
        | CardId::SadisticNature
        | CardId::SecretTechnique
        | CardId::SecretWeapon
        | CardId::TheBomb
        | CardId::ThinkingAhead
        | CardId::Transmutation
        | CardId::Violence
        | CardId::JAX
        | CardId::RitualDagger => colorless::play_colorless(_state, _card, t, context),
        // Unplayable stubs — curses, status, and special cards
        CardId::Wound
        | CardId::Burn
        | CardId::Dazed
        | CardId::Slimed
        | CardId::Parasite
        | CardId::Void
        | CardId::Regret
        | CardId::AscendersBane
        | CardId::Clumsy
        | CardId::CurseOfTheBell
        | CardId::Decay
        | CardId::Doubt
        | CardId::Injury
        | CardId::Necronomicurse
        | CardId::Normality
        | CardId::Pain
        | CardId::Shame
        | CardId::Writhe
        | CardId::Reflex
        | CardId::Tactician => smallvec::smallvec![], // Unplayable / Stub
        CardId::Madness => smallvec::smallvec![ActionInfo {
            action: Action::Madness,
            insertion_mode: crate::runtime::action::AddTo::Bottom,
        }],
    }
}

/// Evaluates a card's damage, block, and magic number based on player powers, target powers, and specific card rules.
/// Maps directly to Java Spire's `applyPowers()` (when target is None) and `calculateCardDamage()` (when target is Some).
pub fn evaluate_card(card: &mut CombatCard, state: &CombatState, target: Option<EntityId>) {
    let def = get_card_definition(card.id);
    let u = if card.upgrades > 0 { 1 } else { 0 };
    let mut damage = card
        .base_damage_override
        .unwrap_or(def.base_damage + u * def.upgrade_damage) as f32;
    let mut block = (def.base_block + u * def.upgrade_block) as f32;
    card.base_magic_num_mut = def.base_magic + u * def.upgrade_magic;

    // 1. Card specific base overrides (Perfected Strike)
    if card.id == CardId::PerfectedStrike {
        let mut strike_count = 0;
        let is_strike = |id| get_card_definition(id).tags.contains(&CardTag::Strike);

        for c in &state.zones.hand {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        for c in &state.zones.draw_pile {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        for c in &state.zones.discard_pile {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        // Count the card itself definitively once
        if is_strike(card.id) {
            strike_count += 1;
        }

        damage += (card.base_magic_num_mut as f32) * (strike_count as f32);
    } else if card.id == CardId::BodySlam {
        damage = state.entities.player.block as f32;
    } else if card.id == CardId::SearingBlow {
        let u = card.upgrades as f32;
        damage = 12.0 + u * (u + 7.0) / 2.0;
    } else if card.id == CardId::RitualDagger {
        damage = if card.misc_value > 0 {
            card.misc_value as f32
        } else {
            def.base_damage as f32
        };
    }

    // 2. Relic atDamageModify hooks (Java: AbstractCard.applyPowers/calculateCardDamage)
    // run before player power atDamageGive hooks. This ordering matters for cases like
    // Strike Dummy under Weak: (base + 3) * 0.75, not base * 0.75 + 3.
    damage =
        crate::content::relics::hooks::modify_player_attack_damage_for_card(state, card, damage);

    // 3. Player Powers
    if let Some(powers) = crate::content::powers::store::powers_for(state, 0) {
        for power in powers {
            damage = crate::content::powers::resolve_power_on_calculate_damage_to_enemy(
                power.power_type,
                state,
                card,
                damage,
                power.amount,
            );
            block = crate::content::powers::resolve_power_on_calculate_block(
                power.power_type,
                state,
                card,
                block,
                power.amount,
            );
        }
    }

    // 4. Stance
    if def.card_type == crate::content::cards::CardType::Attack {
        match state.entities.player.stance {
            crate::runtime::combat::StanceId::Wrath => damage *= 2.0,
            crate::runtime::combat::StanceId::Divinity => damage *= 3.0,
            _ => {}
        }
    }
    // 5. Target Powers
    if def.is_multi_damage {
        card.multi_damage.clear();
        for m in &state.entities.monsters {
            let mut mdmg = damage;
            // Target specific powers (Vulnerable)
            if let Some(target_powers) = crate::content::powers::store::powers_for(state, m.id) {
                for power in target_powers {
                    mdmg = crate::content::powers::resolve_power_on_calculate_damage_from_player(
                        power.power_type,
                        state,
                        card,
                        m.id,
                        mdmg,
                        power.amount,
                    );
                }
                let mut final_receive = mdmg.max(0.0).floor() as i32;
                for power in target_powers {
                    final_receive = crate::content::powers::resolve_power_at_damage_final_receive(
                        power.power_type,
                        final_receive,
                        power.amount,
                        crate::runtime::action::DamageType::Normal,
                    );
                }
                mdmg = final_receive as f32;
            }
            if mdmg < 0.0 {
                mdmg = 0.0;
            }
            card.multi_damage.push(mdmg as i32);
        }
        if let Some(first) = card.multi_damage.first() {
            damage = *first as f32;
        }
    } else if let Some(target_id) = target {
        if let Some(target_powers) = crate::content::powers::store::powers_for(state, target_id) {
            for power in target_powers {
                damage = crate::content::powers::resolve_power_on_calculate_damage_from_player(
                    power.power_type,
                    state,
                    card,
                    target_id,
                    damage,
                    power.amount,
                );
            }
            let mut final_receive = damage.max(0.0).floor() as i32;
            for power in target_powers {
                final_receive = crate::content::powers::resolve_power_at_damage_final_receive(
                    power.power_type,
                    final_receive,
                    power.amount,
                    crate::runtime::action::DamageType::Normal,
                );
            }
            damage = final_receive as f32;
        }
    }

    if damage < 0.0 {
        damage = 0.0;
    }
    if block < 0.0 {
        block = 0.0;
    }

    card.base_damage_mut = damage as i32;
    card.base_block_mut = block as i32;
}

/// Produces a freshly evaluated combat card for actual play execution.
///
/// This avoids relying on potentially stale cached mutation fields on the card
/// object when generating execution-time actions.
pub fn evaluate_card_for_play(
    card: &CombatCard,
    state: &CombatState,
    target: Option<EntityId>,
) -> CombatCard {
    let mut evaluated = card.clone();
    evaluate_card(&mut evaluated, state, target);
    evaluated
}

/// Creates a fresh combat card using Java `makeCopy()` state hooks that depend
/// on the current combat.
///
/// Most cards return a plain new instance. `Blood for Blood` is an important
/// exception: its Java `makeCopy()` immediately applies the player's
/// `damagedThisCombat` cost reduction, which matters for random-card sources
/// such as Dead Branch, Nilry's Codex, Discovery, Infernal Blade, and
/// Metamorphosis.
pub fn make_fresh_card_copy_for_combat(
    card_id: CardId,
    uuid: u32,
    state: &CombatState,
) -> CombatCard {
    let mut card = CombatCard::new(card_id, uuid);
    if card_id == CardId::BloodForBlood {
        let damaged = state.turn.counters.times_damaged_this_combat as i32;
        card.update_cost_java(-damaged);
    }
    card
}

pub fn can_upgrade_card_once(card: &CombatCard) -> bool {
    if card.id == CardId::SearingBlow {
        return true;
    }
    let def = get_card_definition(card.id);
    card.upgrades == 0 && def.card_type != CardType::Status && def.card_type != CardType::Curse
}

/// Applies Java Master Reality upgrade semantics to generated cards.
///
/// Java routes generated cards through action constructors and card-manipulation
/// effects. Those are UI classes, but they also call `upgrade()`. Normal cards
/// ignore the second call after the first upgrade; `Searing Blow` does not, so
/// callers must pass the number of Java upgrade call sites for that path.
pub fn apply_master_reality_to_generated_card(
    card: &mut CombatCard,
    state: &CombatState,
    upgrade_call_sites: u8,
) {
    let def = get_card_definition(card.id);
    if def.card_type == CardType::Curse || def.card_type == CardType::Status {
        return;
    }
    if !crate::content::powers::store::has_power(
        state,
        0,
        crate::content::powers::PowerId::MasterRealityPower,
    ) {
        return;
    }
    for _ in 0..upgrade_call_sites {
        if can_upgrade_card_once(card) {
            card.upgrades += 1;
        }
    }
}

/// Returns the card's intrinsic exhaust-on-play behavior after applying
/// upgrade-sensitive card rules.
pub fn exhausts_when_played(card: &CombatCard) -> bool {
    match card.id {
        CardId::LimitBreak => card.upgrades == 0,
        CardId::Discovery => card.upgrades == 0,
        CardId::SecretTechnique => card.upgrades == 0,
        CardId::SecretWeapon => card.upgrades == 0,
        CardId::ThinkingAhead => card.upgrades == 0,
        _ => get_card_definition(card.id).exhaust,
    }
}

/// Returns the card's effective ethereal status after upgrade-sensitive overrides.
pub fn is_ethereal(card: &CombatCard) -> bool {
    match card.id {
        CardId::Apparition => card.upgrades == 0,
        _ => get_card_definition(card.id).ethereal,
    }
}

pub fn upgraded_base_cost_override(card: &CombatCard) -> Option<i8> {
    match card.id {
        CardId::Barricade if card.upgrades > 0 => Some(2),
        CardId::BloodForBlood if card.upgrades > 0 => Some(3),
        CardId::BodySlam if card.upgrades > 0 => Some(0),
        CardId::Corruption if card.upgrades > 0 => Some(2),
        CardId::DarkEmbrace if card.upgrades > 0 => Some(1),
        CardId::Entrench if card.upgrades > 0 => Some(1),
        CardId::Exhume if card.upgrades > 0 => Some(0),
        CardId::Havoc if card.upgrades > 0 => Some(0),
        CardId::InfernalBlade if card.upgrades > 0 => Some(0),
        CardId::Madness if card.upgrades > 0 => Some(0),
        CardId::SeeingRed if card.upgrades > 0 => Some(0),
        _ => None,
    }
}

pub fn effective_target(card: &CombatCard) -> CardTarget {
    match card.id {
        CardId::Blind if card.upgrades > 0 => CardTarget::AllEnemy,
        CardId::Trip if card.upgrades > 0 => CardTarget::AllEnemy,
        _ => get_card_definition(card.id).target,
    }
}

/// Validates whether a card can be played based on energy, status locks, and curses like Normality.
pub fn can_play_card(card: &CombatCard, state: &CombatState) -> Result<(), &'static str> {
    can_play_card_internal(card, state, false)
}

/// Java queued/autoplay cards still call `canUse`, but `isInAutoplay` only
/// bypasses the final energy-count check inside `hasEnoughEnergy`.
pub fn can_play_card_ignoring_energy(
    card: &CombatCard,
    state: &CombatState,
) -> Result<(), &'static str> {
    can_play_card_internal(card, state, true)
}

fn can_play_card_internal(
    card: &CombatCard,
    state: &CombatState,
    ignore_energy: bool,
) -> Result<(), &'static str> {
    // Curse: Normality Lock
    if state.zones.hand.iter().any(|c| c.id == CardId::Normality) {
        if state.turn.counters.cards_played_this_turn >= 3 {
            return Err("Normality: Cannot play more than 3 cards this turn.");
        }
    }

    let def = crate::content::cards::get_card_definition(card.id);
    let cost = card.get_cost();

    // In Slay the Spire, internally cost -2 means the card is Unplayable.
    if cost < -1 {
        if def.card_type == crate::content::cards::CardType::Curse
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::BlueCandle)
        {
            // Blue Candle override
        } else if def.card_type == crate::content::cards::CardType::Status
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::MedicalKit)
        {
            // Medical Kit override
        } else {
            return Err("Card is unplayable.");
        }
    }

    // Java: hasEnoughEnergy() — Power.canPlayCard() hook
    // Iterates all player powers; if any returns false, card cannot be played.
    if let Some(player_powers) = crate::content::powers::store::powers_for(state, 0) {
        for ps in player_powers {
            if !crate::content::powers::resolve_power_can_play_card(ps.power_type, card) {
                return Err("A power prevents playing this card.");
            }
        }
    }

    // Java: hasEnoughEnergy() — Entangled hardcode (L857-860)
    // This is separate from the canPlayCard hook; Java checks it explicitly.
    if let Some(player_powers) = crate::content::powers::store::powers_for(state, 0) {
        if player_powers
            .iter()
            .any(|p| p.power_type == crate::content::powers::PowerId::Entangle)
            && def.card_type == crate::content::cards::CardType::Attack
        {
            return Err("Entangled: Cannot play Attacks this turn.");
        }
    }

    // Card-specific overrides (Java: card.canUse overrides)
    match card.id {
        CardId::Clash => {
            let has_non_attack = state.zones.hand.iter().any(|c| {
                let d = crate::content::cards::get_card_definition(c.id);
                d.card_type != crate::content::cards::CardType::Attack
            });
            if has_non_attack {
                return Err("Can only play Clash if every card in your hand is an Attack.");
            }
        }
        CardId::SecretTechnique => {
            let has_skill = state
                .zones
                .draw_pile
                .iter()
                .any(|c| get_card_definition(c.id).card_type == CardType::Skill);
            if !has_skill {
                return Err("No Skill cards in draw pile.");
            }
        }
        CardId::SecretWeapon => {
            let has_attack = state
                .zones
                .draw_pile
                .iter()
                .any(|c| get_card_definition(c.id).card_type == CardType::Attack);
            if !has_attack {
                return Err("No Attack cards in draw pile.");
            }
        }
        // Future cards like Grand Finale (if draw pile not empty) go here
        _ => {}
    }

    // Default cost validation
    if !ignore_energy && cost >= 0 && state.turn.energy < (cost as u8) {
        return Err("Not enough energy.");
    }

    Ok(())
}

/// A global hook called immediately after a card is played to aggregate passive triggers from the state (e.g. Curses).
pub fn on_play_card(played_card: &CombatCard, state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut passive_actions = smallvec::SmallVec::new();

    // Curse: Pain (Lose 1 HP for every card played)
    for card in &state.zones.hand {
        if card.id == CardId::Pain && card.uuid != played_card.uuid {
            passive_actions.push(crate::content::cards::curses::pain::on_other_card_played());
        }
    }

    // Other hooks (Time Eater, Velvet Choker, etc.) can be placed here later.

    passive_actions
}

pub fn resolve_card_on_manual_discard(
    card: &CombatCard,
    _state: &CombatState,
) -> SmallVec<[ActionInfo; 4]> {
    let def = get_card_definition(card.id);
    let upgraded = if card.upgrades > 0 { 1 } else { 0 };
    let magic = def.base_magic + upgraded * def.upgrade_magic;

    match card.id {
        CardId::Reflex => smallvec::smallvec![ActionInfo {
            action: Action::DrawCards(magic.max(0) as u32),
            insertion_mode: crate::runtime::action::AddTo::Bottom,
        }],
        CardId::Tactician => smallvec::smallvec![ActionInfo {
            action: Action::GainEnergy {
                amount: magic.max(0),
            },
            insertion_mode: crate::runtime::action::AddTo::Top,
        }],
        _ => smallvec::smallvec![],
    }
}
