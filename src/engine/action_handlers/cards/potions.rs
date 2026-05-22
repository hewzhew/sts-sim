use crate::engine::action_handlers::damage;
use crate::engine::targeting;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::CombatState;

pub fn handle_use_potion(slot: usize, target: Option<usize>, state: &mut CombatState) {
    if let Some(Some(potion)) = state.entities.potions.get(slot).cloned() {
        if !crate::content::potions::potion_can_use_in_combat_like_java(&potion, state) {
            return;
        }
        let def = crate::content::potions::get_potion_definition(potion.id);
        let mut potency = def.base_potency;
        if state
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::SacredBark)
        {
            potency *= 2;
        }
        if potion.id == crate::content::potions::PotionId::EntropicBrew {
            let potion_class = potion_class_for_combat(state);
            let potion_slots = state.entities.potions.len();
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..potion_slots {
                let potion_id = crate::content::potions::random_potion(
                    &mut state.rng.potion_rng,
                    potion_class,
                    true,
                );
                actions.push(ActionInfo {
                    action: Action::ObtainSpecificPotion(potion_id),
                    insertion_mode: AddTo::Bottom,
                });
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::FruitJuice {
            damage::increase_player_max_hp_like_java(potency, state);
            let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
            state.queue_actions(relic_actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::DistilledChaosPotion {
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..potency.max(0) {
                actions.push(ActionInfo {
                    action: Action::PlayTopCard {
                        target: targeting::pick_random_target(
                            state,
                            crate::state::TargetValidation::AnyEnemy,
                        ),
                        exhaust: false,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::EssenceOfDarkness {
            let orb_slots =
                (state.entities.player.max_orbs as usize).max(state.entities.player.orbs.len());
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..orb_slots {
                for _ in 0..potency.max(0) {
                    actions.push(ActionInfo {
                        action: Action::ChannelOrb(crate::runtime::combat::OrbId::Dark),
                        insertion_mode: AddTo::Bottom,
                    });
                }
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::LiquidMemories
            && state.zones.discard_pile.len() <= potency.max(0) as usize
        {
            let uuids: Vec<u32> = state.zones.discard_pile.iter().map(|c| c.uuid).collect();
            for uuid in uuids {
                if state.zones.hand.len() >= 10 {
                    break;
                }
                if let Some(pos) = state.zones.discard_pile.iter().position(|c| c.uuid == uuid) {
                    let mut card = state.zones.discard_pile.remove(pos);
                    card.set_cost_for_turn_java(0);
                    state.zones.hand.push(card);
                }
            }
            let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
            state.queue_actions(relic_actions);
            state.entities.potions[slot] = None;
            return;
        }
        let resolved_target = match targeting::resolve_target_request(
            state,
            targeting::validation_for_potion_target(def.target_required),
            target,
        ) {
            Ok(target) => target,
            Err(_) => return,
        };
        if potion.id == crate::content::potions::PotionId::FirePotion {
            let Some(target_id) = resolved_target else {
                return;
            };
            let mut output = potency.max(0);
            for power in crate::content::powers::store::powers_snapshot_for(state, target_id) {
                output = crate::content::powers::resolve_power_at_damage_final_receive(
                    power.power_type,
                    output,
                    power.amount,
                    DamageType::Thorns,
                );
            }
            let mut actions = smallvec::smallvec![ActionInfo {
                action: Action::Damage(DamageInfo {
                    source: 0,
                    target: target_id,
                    base: potency,
                    output,
                    damage_type: DamageType::Thorns,
                    is_modified: output != potency,
                }),
                insertion_mode: AddTo::Bottom,
            }];
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        let actions = crate::content::potions::potion_effects::get_potion_actions(
            state,
            potion.id,
            resolved_target,
            potency,
        );
        let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
        let mut combined = actions;
        combined.extend(relic_actions);
        state.queue_actions(combined);
        state.entities.potions[slot] = None;
    }
}

fn potion_class_for_combat(state: &CombatState) -> crate::content::potions::PotionClass {
    match state.meta.player_class.as_str() {
        "Silent" => crate::content::potions::PotionClass::Silent,
        "Defect" => crate::content::potions::PotionClass::Defect,
        "Watcher" => crate::content::potions::PotionClass::Watcher,
        _ => crate::content::potions::PotionClass::Ironclad,
    }
}

pub fn handle_obtain_potion(state: &mut CombatState) {
    let potion_class = match state.meta.player_class.as_str() {
        "Silent" => crate::content::potions::PotionClass::Silent,
        "Defect" => crate::content::potions::PotionClass::Defect,
        "Watcher" => crate::content::potions::PotionClass::Watcher,
        _ => crate::content::potions::PotionClass::Ironclad,
    };
    let potion_id =
        crate::content::potions::random_potion(&mut state.rng.potion_rng, potion_class, true);

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Sozu)
    {
        return;
    }
    if let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) {
        state.entities.potions[slot] = Some(crate::content::potions::Potion::new(
            potion_id,
            40000 + slot as u32,
        ));
    }
}

/// Java source evidence:
/// `actions/common/ObtainPotionAction.java` stores one concrete `AbstractPotion`
/// and on first update performs:
///   if Sozu: flash only
///   else: AbstractDungeon.player.obtainPotion(this.potion)
/// `AbstractPlayer.obtainPotion` places into the first empty potion slot and
/// does nothing if all slots are full. Rust models only the mechanical state
/// transition; sound/flash/UI effects are intentionally excluded.
pub fn obtain_specific_potion_if_allowed(
    state: &mut CombatState,
    potion_id: crate::content::potions::PotionId,
) -> bool {
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Sozu)
    {
        return false;
    }
    let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) else {
        return false;
    };
    state.entities.potions[slot] = Some(crate::content::potions::Potion::new(
        potion_id,
        40000 + slot as u32,
    ));
    true
}
