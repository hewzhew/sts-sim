use crate::content::potions::get_potion_definition;
use crate::content::relics::RelicId;
use crate::runtime::combat::{CombatCard, CombatState, Intent, Power};
use crate::runtime::monster_move::MonsterMoveSpec;
use crate::state::core::{EngineState, PendingChoice, PileType};

use super::{card_line, debug_words, push_line};
use crate::eval::run_control::session::RunControlSession;
use crate::eval::run_control::view_model::{monster_name, reward_card_label};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatZonePanel {
    Draw,
    Discard,
    Exhaust,
}

impl CombatZonePanel {
    fn label(self) -> &'static str {
        match self {
            CombatZonePanel::Draw => "Draw",
            CombatZonePanel::Discard => "Discard",
            CombatZonePanel::Exhaust => "Exhaust",
        }
    }
}

pub(super) fn push_combat_screen(session: &RunControlSession, out: &mut String) {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return;
    };
    push_line(
        out,
        format!(
            "Player: HP {}/{} | Energy {}/{} | Block {}",
            combat.entities.player.current_hp,
            combat.entities.player.max_hp,
            combat.turn.energy,
            combat.entities.player.energy_master,
            combat.entities.player.block
        ),
    );
    push_power_line(out, "Player powers", combat_power_list(combat, 0));
    push_line(out, "Enemies:");
    for monster in &combat.entities.monsters {
        let intent = monster_intent_line(combat, monster.id);
        let powers = combat_power_list(combat, monster.id);
        let power_suffix = if powers.is_empty() {
            String::new()
        } else {
            format!(" | powers: {}", powers.join(", "))
        };
        push_line(
            out,
            format!(
                "  slot {} | {} {}/{} | block {} | intent: {}{}",
                monster.slot,
                monster_name(monster.monster_type),
                monster.current_hp,
                monster.max_hp,
                monster.block,
                intent,
                power_suffix,
            ),
        );
    }
    push_line(out, "");
    push_line(out, "Hand:");
    for (idx, card) in combat.zones.hand.iter().enumerate() {
        push_line(out, format!("  {idx} {}", card_line(card, true)));
    }
    push_pending_choice_screen(session, combat, out);
    let potion_line = combat_potion_short_line(session);
    if !potion_line.is_empty() {
        push_line(out, "");
        push_line(out, potion_line);
    }
}

fn push_pending_choice_screen(session: &RunControlSession, combat: &CombatState, out: &mut String) {
    let EngineState::PendingChoice(choice) = &session.engine_state else {
        return;
    };
    push_line(out, "");
    match choice {
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => {
            push_line(
                out,
                format!(
                    "Pending selection: {:?} from {:?} | choose {} to {} | cancel={}",
                    reason, source_pile, min_cards, max_cards, can_cancel
                ),
            );
            push_selection_cards(
                out,
                grid_source_cards(combat, *source_pile),
                candidate_uuids,
            );
            push_selection_command(out, *min_cards as usize, *max_cards as usize, *can_cancel);
        }
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => {
            push_line(
                out,
                format!(
                    "Pending hand selection: {:?} | choose {} to {} | cancel={}",
                    reason, min_cards, max_cards, can_cancel
                ),
            );
            push_selection_cards(out, &combat.zones.hand, candidate_uuids);
            push_selection_command(out, *min_cards as usize, *max_cards as usize, *can_cancel);
        }
        PendingChoice::DiscoverySelect(choice) => {
            push_line(out, "Pending discovery:");
            for (idx, card) in choice.cards.iter().enumerate() {
                push_line(out, format!("  {idx} {}", reward_card_label(*card, 0)));
            }
        }
        PendingChoice::CardRewardSelect { cards, .. } => {
            push_line(out, "Pending card reward:");
            for (idx, card) in cards.iter().enumerate() {
                push_line(out, format!("  {idx} {}", reward_card_label(*card, 0)));
            }
        }
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => {
            push_line(out, "Pending Foreign Influence choice:");
            for (idx, card) in cards.iter().enumerate() {
                push_line(
                    out,
                    format!("  {idx} {}", reward_card_label(*card, u8::from(*upgraded))),
                );
            }
        }
        PendingChoice::ChooseOneSelect { choices } => {
            push_line(out, "Pending choose-one:");
            for (idx, choice) in choices.iter().enumerate() {
                push_line(
                    out,
                    format!(
                        "  {idx} {}",
                        reward_card_label(choice.card_id, choice.upgrades)
                    ),
                );
            }
        }
        PendingChoice::ScrySelect { cards, .. } => {
            push_line(out, "Pending scry:");
            for (idx, card) in cards.iter().enumerate() {
                push_line(out, format!("  {idx} {}", reward_card_label(*card, 0)));
            }
            push_selection_command(out, 0, cards.len(), false);
        }
        PendingChoice::StanceChoice => {
            push_line(out, "Pending stance choice:");
            push_line(out, "  0 Wrath");
            push_line(out, "  1 Calm");
        }
    }
}

fn push_selection_cards(out: &mut String, cards: &[CombatCard], candidate_uuids: &[u32]) {
    push_line(out, "Selection cards:");
    for (idx, uuid) in candidate_uuids.iter().enumerate() {
        let label = cards
            .iter()
            .find(|card| card.uuid == *uuid)
            .map(|card| card_line(card, true))
            .unwrap_or_else(|| format!("card uuid {uuid}"));
        push_line(out, format!("  {idx} {label} | uuid {uuid}"));
    }
}

fn push_selection_command(
    out: &mut String,
    min_choices: usize,
    max_choices: usize,
    can_cancel: bool,
) {
    let mut parts = vec![format!(
        "select <idx...> = submit {min_choices} to {max_choices}"
    )];
    if min_choices == 0 {
        parts.push("select = choose nothing".to_string());
    }
    if can_cancel {
        parts.push("cancel = return without selecting".to_string());
    }
    push_line(out, format!("Selection command: {}", parts.join(" | ")));
}

fn grid_source_cards(combat: &CombatState, source_pile: PileType) -> &[CombatCard] {
    match source_pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &combat.meta.master_deck_snapshot,
    }
}

fn combat_power_list(combat: &CombatState, entity_id: usize) -> Vec<String> {
    combat
        .entities
        .power_db
        .get(&entity_id)
        .into_iter()
        .flatten()
        .map(power_label)
        .collect()
}

fn push_power_line(out: &mut String, label: &str, powers: Vec<String>) {
    if !powers.is_empty() {
        push_line(out, format!("{label}: {}", powers.join(", ")));
    }
}

fn power_label(power: &Power) -> String {
    if power.amount == -1 {
        debug_words(&format!("{:?}", power.power_type))
    } else {
        format!(
            "{} {}",
            debug_words(&format!("{:?}", power.power_type)),
            power.amount
        )
    }
}

pub fn render_combat_zone_panel(session: &RunControlSession, zone: CombatZonePanel) -> String {
    let mut out = String::new();
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return format!("{} is only available during combat.", zone.label());
    };
    let cards = match zone {
        CombatZonePanel::Draw => &combat.zones.draw_pile,
        CombatZonePanel::Discard => &combat.zones.discard_pile,
        CombatZonePanel::Exhaust => &combat.zones.exhaust_pile,
    };
    push_line(
        &mut out,
        format!("{} pile {} cards:", zone.label(), cards.len()),
    );
    if cards.is_empty() {
        push_line(&mut out, "  empty");
    }
    for (idx, card) in cards.iter().enumerate() {
        push_line(&mut out, format!("  {idx} {}", card_line(card, true)));
    }
    push_line(&mut out, "");
    push_line(&mut out, "Commands: main | raw | q");
    out
}

fn monster_intent_line(combat: &CombatState, monster_id: usize) -> String {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == monster_id)
    else {
        return "unknown".to_string();
    };
    if combat.entities.player.has_relic(RelicId::RunicDome) {
        return "hidden (Runic Dome)".to_string();
    }
    let observation = combat
        .runtime
        .monster_protocol
        .get(&monster.id)
        .map(|protocol| &protocol.observation);
    let turn_plan = monster.turn_plan();
    let damage_per_hit = observation
        .filter(|obs| obs.preview_damage_per_hit > 0)
        .map(|obs| obs.preview_damage_per_hit)
        .or_else(|| turn_plan.attack().map(|attack| attack.base_damage));
    if let Some(intent) = observation
        .filter(|obs| obs.visible_intent != Intent::Unknown)
        .map(|obs| &obs.visible_intent)
    {
        return format_visible_intent(intent, damage_per_hit);
    }
    format_monster_move_spec(&turn_plan.summary_spec(), damage_per_hit)
}

fn format_visible_intent(intent: &Intent, damage_per_hit: Option<i32>) -> String {
    match intent {
        Intent::Attack { damage, hits } => {
            format!(
                "attack {}",
                damage_phrase(damage_per_hit.unwrap_or(*damage), *hits)
            )
        }
        Intent::AttackBuff { damage, hits } => format!(
            "attack {}, buff",
            damage_phrase(damage_per_hit.unwrap_or(*damage), *hits)
        ),
        Intent::AttackDebuff { damage, hits } => format!(
            "attack {}, debuff",
            damage_phrase(damage_per_hit.unwrap_or(*damage), *hits)
        ),
        Intent::AttackDefend { damage, hits } => format!(
            "attack {}, block",
            damage_phrase(damage_per_hit.unwrap_or(*damage), *hits)
        ),
        Intent::Buff => "buff".to_string(),
        Intent::Debuff => "debuff".to_string(),
        Intent::StrongDebuff => "strong debuff".to_string(),
        Intent::Debug => "debug".to_string(),
        Intent::Defend => "block".to_string(),
        Intent::DefendDebuff => "block, debuff".to_string(),
        Intent::DefendBuff => "block, buff".to_string(),
        Intent::Escape => "escape".to_string(),
        Intent::Magic => "special".to_string(),
        Intent::None => "none".to_string(),
        Intent::Sleep => "sleep".to_string(),
        Intent::Stun => "stun".to_string(),
        Intent::Unknown => "unknown".to_string(),
    }
}

fn format_monster_move_spec(spec: &MonsterMoveSpec, damage_per_hit: Option<i32>) -> String {
    match spec {
        MonsterMoveSpec::Attack(attack) => format_attack_spec("attack", attack, damage_per_hit),
        MonsterMoveSpec::AttackAddCard(attack, add_card) => format!(
            "{}, {}",
            format_attack_spec("attack", attack, damage_per_hit),
            add_card_label(add_card)
        ),
        MonsterMoveSpec::AttackUpgradeCards(attack, upgrade) => format!(
            "{}, upgrade {}",
            format_attack_spec("attack", attack, damage_per_hit),
            reward_card_label(upgrade.card_id, 0)
        ),
        MonsterMoveSpec::AttackBuff(attack, buff) => format!(
            "{}, {}",
            format_attack_spec("attack", attack, damage_per_hit),
            buff_label(buff.power_id, buff.amount)
        ),
        MonsterMoveSpec::AttackSustain(attack) => {
            format!(
                "{}, sustain",
                format_attack_spec("attack", attack, damage_per_hit)
            )
        }
        MonsterMoveSpec::AttackDebuff(attack, debuff) => format!(
            "{}, {}",
            format_attack_spec("attack", attack, damage_per_hit),
            debuff_label(debuff.power_id, debuff.amount)
        ),
        MonsterMoveSpec::AttackDefend(attack, defend) => format!(
            "{}, block {}",
            format_attack_spec("attack", attack, damage_per_hit),
            defend.block
        ),
        MonsterMoveSpec::AddCard(add_card) => add_card_label(add_card),
        MonsterMoveSpec::Buff(buff) => buff_label(buff.power_id, buff.amount),
        MonsterMoveSpec::Debuff(debuff) => debuff_label(debuff.power_id, debuff.amount),
        MonsterMoveSpec::StrongDebuff(debuff) => {
            format!("strong {}", debuff_label(debuff.power_id, debuff.amount))
        }
        MonsterMoveSpec::Defend(defend) => format!("block {}", defend.block),
        MonsterMoveSpec::DefendDebuff(defend, debuff) => {
            format!(
                "block {}, {}",
                defend.block,
                debuff_label(debuff.power_id, debuff.amount)
            )
        }
        MonsterMoveSpec::DefendBuff(defend, buff) => {
            format!(
                "block {}, {}",
                defend.block,
                buff_label(buff.power_id, buff.amount)
            )
        }
        MonsterMoveSpec::Heal(heal) => format!("heal {}", heal.amount),
        MonsterMoveSpec::Escape => "escape".to_string(),
        MonsterMoveSpec::Magic => "special".to_string(),
        MonsterMoveSpec::Sleep => "sleep".to_string(),
        MonsterMoveSpec::Stun => "stun".to_string(),
        MonsterMoveSpec::Debug => "debug".to_string(),
        MonsterMoveSpec::None => "none".to_string(),
        MonsterMoveSpec::Unknown => "unknown".to_string(),
    }
}

fn format_attack_spec(
    label: &str,
    attack: &crate::runtime::monster_move::AttackSpec,
    damage_per_hit: Option<i32>,
) -> String {
    format!(
        "{label} {}",
        damage_phrase(damage_per_hit.unwrap_or(attack.base_damage), attack.hits)
    )
}

fn damage_phrase(damage_per_hit: i32, hits: u8) -> String {
    let hits = hits.max(1);
    if hits == 1 {
        damage_per_hit.to_string()
    } else {
        let total = damage_per_hit.saturating_mul(hits as i32);
        format!("{damage_per_hit}x{hits} ({total})")
    }
}

fn add_card_label(step: &crate::runtime::monster_move::AddCardStep) -> String {
    format!(
        "add {} {} to {}",
        step.amount,
        reward_card_label(step.card_id, u8::from(step.upgraded)),
        debug_words(&format!("{:?}", step.destination)).to_lowercase()
    )
}

fn buff_label(power_id: crate::content::powers::PowerId, amount: i32) -> String {
    format!(
        "buff {} {}",
        debug_words(&format!("{power_id:?}")),
        signed_amount(amount)
    )
}

fn debuff_label(power_id: crate::content::powers::PowerId, amount: i32) -> String {
    format!(
        "debuff {} {}",
        debug_words(&format!("{power_id:?}")),
        amount
    )
}

fn signed_amount(amount: i32) -> String {
    if amount >= 0 {
        format!("+{amount}")
    } else {
        amount.to_string()
    }
}

fn combat_potion_short_line(session: &RunControlSession) -> String {
    let potions = session
        .visible_potions()
        .iter()
        .enumerate()
        .filter_map(|(idx, potion)| {
            potion.as_ref().map(|potion| {
                let def = get_potion_definition(potion.id);
                format!("{idx} {}", def.name)
            })
        })
        .collect::<Vec<_>>();
    if potions.is_empty() {
        String::new()
    } else {
        format!("Potions: {}", potions.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::runtime::monster_move::{AttackSpec, BuffSpec, DamageKind};
    use crate::state::core::{
        ActiveCombat, CombatContext, EngineState, GridSelectReason, HandSelectReason,
        PendingChoice, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;

    #[test]
    fn combat_intent_label_does_not_leak_move_spec_debug() {
        let rendered = format_monster_move_spec(
            &MonsterMoveSpec::AttackBuff(
                AttackSpec {
                    base_damage: 7,
                    hits: 2,
                    damage_kind: DamageKind::Normal,
                },
                BuffSpec {
                    power_id: crate::content::powers::PowerId::Strength,
                    amount: 3,
                },
            ),
            Some(8),
        );

        assert_eq!(rendered, "attack 8x2 (16), buff Strength +3");
        assert!(!rendered.contains("AttackSpec"));
        assert!(!rendered.contains("BuffSpec"));
    }

    #[test]
    fn combat_screen_hides_enemy_intent_with_runic_dome() {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 42;
        monster.slot = 0;
        combat.entities.monsters.push(monster);
        combat.set_monster_protocol_visible_intent(42, Intent::Attack { damage: 7, hits: 1 });
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicDome));
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let rendered = crate::eval::run_control::render_run_control_state(&session);

        assert!(rendered.contains("intent: hidden (Runic Dome)"));
        assert!(!rendered.contains("intent: attack 7"));
    }

    #[test]
    fn combat_screen_renders_pending_grid_select_cards() {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 20),
        ];
        let choice = PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let rendered = crate::eval::run_control::render_run_control_state(&session);

        assert!(rendered.contains("Pending selection: MoveToDrawPile from Discard"));
        assert!(rendered.contains("0 Strike"));
        assert!(rendered.contains("1 Defend"));
        assert!(rendered.contains("Selection command: select <idx...> = submit 1 to 1"));
        assert!(rendered.contains("select | Submit selection with `select <idx...>`"));
    }

    #[test]
    fn combat_screen_uses_compact_surface_for_multi_hand_select() {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::ThunderClap, 10),
            CombatCard::new(CardId::RecklessCharge, 20),
            CombatCard::new(CardId::Whirlwind, 30),
        ];
        let choice = PendingChoice::HandSelect {
            candidate_uuids: vec![10, 20, 30],
            min_cards: 0,
            max_cards: 3,
            can_cancel: true,
            reason: HandSelectReason::Exhaust,
        };
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let rendered = crate::eval::run_control::render_run_control_state(&session);

        assert!(rendered.contains("Pending hand selection: Exhaust | choose 0 to 3"));
        assert!(rendered.contains("0 Thunderclap"));
        assert!(rendered.contains("1 Reckless Charge"));
        assert!(rendered.contains("2 Whirlwind"));
        assert!(rendered.contains("select = choose nothing"));
        assert!(rendered.contains("cancel = return without selecting"));
        assert!(rendered.contains("select | Submit selection with `select <idx...>`"));
        assert!(
            !rendered.contains("Exhaust: Thunderclap, Reckless Charge"),
            "multi-select UI must not enumerate card combinations:\n{rendered}"
        );
    }
}
