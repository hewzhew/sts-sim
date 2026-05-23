use super::*;
use crate::content::cards::{self, CardTarget, CardType};
use std::cmp::Ordering;

// These ranks only decide child-generation order inside the same legal action set.
// They never merge, prune, or claim that two actions are equivalent.
const ROLE_LETHAL_CARD: i32 = 130;
const ROLE_PREVENT_VISIBLE_LETHAL: i32 = 120;
const ROLE_TACTICAL_POTION_BASE: i32 = 60;
const ROLE_PREVENT_HP_LOSS: i32 = 85;
const ROLE_DEFERRED_SETUP: i32 = 75;
const ROLE_DAMAGE_PROGRESS: i32 = 60;
const ROLE_BLOCK: i32 = 45;
const ROLE_UTILITY_PLAY: i32 = 35;
const ROLE_END_TURN: i32 = 0;
const ROLE_DISCARD_POTION: i32 = -20;

#[derive(Clone, Debug)]
pub(super) struct OrderedActionChoice {
    pub(super) original_action_id: usize,
    pub(super) choice: CombatActionChoice,
}

#[derive(Clone, Debug)]
struct ActionOrderingEntry {
    original_action_id: usize,
    choice: CombatActionChoice,
    priority: ActionOrderingPriority,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActionOrderingPriority {
    role_rank: i32,
    potion_tactical_rank: i32,
    target_progress: i32,
    block: i32,
    damage: i32,
    cheaper_cost: i32,
}

pub(super) fn order_action_choices(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<CombatActionChoice>,
) -> Vec<OrderedActionChoice> {
    let mut entries = choices
        .into_iter()
        .enumerate()
        .map(|(original_action_id, choice)| ActionOrderingEntry {
            original_action_id,
            priority: priority_for_input(engine, combat, &choice.input),
            choice,
        })
        .collect::<Vec<_>>();

    if matches!(engine, EngineState::CombatPlayerTurn) {
        entries.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.original_action_id.cmp(&right.original_action_id))
        });
    }

    entries
        .into_iter()
        .map(|entry| OrderedActionChoice {
            original_action_id: entry.original_action_id,
            choice: entry.choice,
        })
        .collect()
}

fn priority_for_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> ActionOrderingPriority {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return ActionOrderingPriority::neutral();
    }

    match input {
        ClientInput::PlayCard { card_index, target } => {
            priority_for_play_card(combat, *card_index, *target)
        }
        ClientInput::UsePotion { .. } => {
            let potion_rank =
                potions::semantic_potion_tactical_priority(combat, input).unwrap_or_default();
            ActionOrderingPriority {
                role_rank: ROLE_TACTICAL_POTION_BASE + potion_rank,
                potion_tactical_rank: potion_rank,
                ..ActionOrderingPriority::neutral()
            }
        }
        ClientInput::DiscardPotion(_) => ActionOrderingPriority {
            role_rank: ROLE_DISCARD_POTION,
            ..ActionOrderingPriority::neutral()
        },
        ClientInput::EndTurn => ActionOrderingPriority {
            role_rank: ROLE_END_TURN,
            ..ActionOrderingPriority::neutral()
        },
        _ => ActionOrderingPriority {
            role_rank: ROLE_UTILITY_PLAY,
            ..ActionOrderingPriority::neutral()
        },
    }
}

fn priority_for_play_card(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
) -> ActionOrderingPriority {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return ActionOrderingPriority::neutral();
    };

    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let def = cards::get_card_definition(card.id);
    let target_kind = cards::effective_target(card);
    let damage = evaluated.base_damage_mut.max(0);
    let block = evaluated.base_block_mut.max(0);
    let target_progress = target_progress_hint(combat, target_kind, target, damage);
    let visible_damage = visible_incoming_damage(combat);
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block = (visible_damage - current_block - block).max(0);
    let prevents_visible_lethal =
        visible_loss_now >= current_hp && visible_loss_after_block < current_hp;
    let prevents_hp_loss = visible_loss_after_block < visible_loss_now;
    let role_rank = if target_progress_kills(combat, target_kind, target, damage) {
        ROLE_LETHAL_CARD
    } else if prevents_visible_lethal {
        ROLE_PREVENT_VISIBLE_LETHAL
    } else if def.card_type == CardType::Power {
        ROLE_DEFERRED_SETUP
    } else if prevents_hp_loss {
        ROLE_PREVENT_HP_LOSS
    } else if target_progress > 0 {
        ROLE_DAMAGE_PROGRESS
    } else if block > 0 {
        ROLE_BLOCK
    } else {
        ROLE_UTILITY_PLAY
    };

    ActionOrderingPriority {
        role_rank,
        target_progress,
        block,
        damage,
        cheaper_cost: -card.cost_for_turn_java().max(0),
        ..ActionOrderingPriority::neutral()
    }
}

fn target_progress_hint(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> i32 {
    if damage <= 0 {
        return 0;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
            .sum(),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .map(|hp| damage.min(hp).max(0))
            .unwrap_or_default(),
        _ => 0,
    }
}

fn target_progress_kills(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> bool {
    if damage <= 0 {
        return false;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .any(|monster| damage >= monster.current_hp + monster.block),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .is_some_and(|hp| damage >= hp),
        _ => false,
    }
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

impl ActionOrderingPriority {
    fn neutral() -> Self {
        Self {
            role_rank: ROLE_END_TURN,
            potion_tactical_rank: 0,
            target_progress: 0,
            block: 0,
            damage: 0,
            cheaper_cost: 0,
        }
    }
}

impl Ord for ActionOrderingPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.role_rank
            .cmp(&other.role_rank)
            .then_with(|| self.potion_tactical_rank.cmp(&other.potion_tactical_rank))
            .then_with(|| self.target_progress.cmp(&other.target_progress))
            .then_with(|| self.block.cmp(&other.block))
            .then_with(|| self.damage.cmp(&other.damage))
            .then_with(|| self.cheaper_cost.cmp(&other.cheaper_cost))
    }
}

impl PartialOrd for ActionOrderingPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn combat_ordering_keeps_original_action_ids_after_reordering() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.current_hp = 6;
        monster.max_hp = 6;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered[0].original_action_id, 1);
        assert!(matches!(
            ordered[0].choice.input,
            ClientInput::PlayCard { .. }
        ));
        assert_eq!(ordered[1].original_action_id, 0);
    }

    #[test]
    fn lethal_card_is_ordered_before_nonlethal_damage() {
        let mut combat = blank_test_combat();
        let mut low_hp = test_monster(EnemyId::LouseNormal);
        low_hp.current_hp = 6;
        low_hp.max_hp = 6;
        low_hp.id = 1;
        let mut high_hp = test_monster(EnemyId::JawWorm);
        high_hp.current_hp = 30;
        high_hp.max_hp = 30;
        high_hp.id = 2;
        combat.entities.monsters = vec![low_hp, high_hp];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(2),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered[0].original_action_id, 1);
    }

    #[test]
    fn block_that_prevents_visible_lethal_is_ordered_before_damage() {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = 5;
        combat.entities.player.block = 0;
        let mut monster = test_monster(EnemyId::Cultist);
        monster.set_planned_move_id(1);
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: None,
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered[0].original_action_id, 1);
    }

    #[test]
    fn non_player_turn_choices_keep_existing_order() {
        let combat = blank_test_combat();
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::Proceed),
            CombatActionChoice::from_input(&combat, ClientInput::Cancel),
        ];

        let ordered = order_action_choices(&EngineState::CombatProcessing, &combat, choices);

        assert_eq!(ordered[0].original_action_id, 0);
        assert_eq!(ordered[1].original_action_id, 1);
    }
}
