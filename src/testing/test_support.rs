use crate::combat::{
    CombatCard, CombatMeta, CombatPhase, CombatRng, CombatState, EngineRuntime, EntityState,
    Intent, MonsterEntity, PlayerEntity, Power, RelicBuses, StanceId, TurnRuntime,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::core::EntityId;
use std::collections::{HashMap, VecDeque};

pub fn basic_combat() -> CombatState {
    CombatState {
        meta: CombatMeta {
            ascension_level: 0,
            player_class: "Ironclad",
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: crate::combat::CardZones {
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 10,
        },
        entities: EntityState {
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: EnemyId::JawWorm as usize,
                current_hp: 40,
                max_hp: 40,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Unknown,
                move_history: VecDeque::new(),
                intent_dmg: 0,
                logical_position: 0,
                protocol_identity: Default::default(),
                hexaghost: Default::default(),
                chosen: Default::default(),
                darkling: Default::default(),
                lagavulin: Default::default(),
            }],
            potions: vec![None, None, None],
            power_db: HashMap::new(),
        },
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(crate::rng::RngPool::new(123)),
        runtime: Default::default(),
    }
}

pub fn single_monster_combat(enemy_id: EnemyId) -> CombatState {
    basic_combat().with_monster_type(1, enemy_id)
}

pub fn combat_with_hand(card_ids: &[CardId]) -> CombatState {
    basic_combat().with_hand_ids(card_ids)
}

pub fn combat_with_hand_and_intent(hand: Vec<CombatCard>, intent: Intent) -> CombatState {
    basic_combat()
        .with_rng_seed(11)
        .with_card_uuid_counter(100)
        .with_player_hp(50)
        .with_monster_max_hp(1, 40)
        .with_monster_hp(1, 40)
        .with_monster_intent(1, intent)
        .with_hand_cards(hand)
        .with_player_block(0)
        .with_energy(3)
}

pub fn combat_with_monsters(monsters: Vec<MonsterEntity>) -> CombatState {
    basic_combat().with_monsters(monsters)
}

pub fn combat_with_hand_and_monsters(hand: &[CardId], monsters: Vec<MonsterEntity>) -> CombatState {
    basic_combat()
        .with_card_uuid_counter(100)
        .with_hand_ids(hand)
        .with_monsters(monsters)
}

pub fn combat_with_attacking_monster(
    enemy_id: EnemyId,
    monster_hp: i32,
    damage: i32,
) -> CombatState {
    basic_combat()
        .with_card_uuid_counter(100)
        .with_monster_type(1, enemy_id)
        .with_monster_max_hp(1, monster_hp)
        .with_monster_hp(1, monster_hp)
        .with_monster_intent(1, Intent::Attack { damage, hits: 1 })
}

pub trait CombatTestExt: Sized {
    fn with_player_hp(self, hp: i32) -> Self;
    fn with_player_max_hp(self, max_hp: i32) -> Self;
    fn with_player_block(self, block: i32) -> Self;
    fn with_player_gold(self, gold: i32) -> Self;
    fn with_player_stance(self, stance: StanceId) -> Self;
    fn with_energy(self, energy: u8) -> Self;
    fn with_turn_count(self, turn_count: i32) -> Self;
    fn with_rng_seed(self, seed: u64) -> Self;
    fn with_card_uuid_counter(self, next_uuid: u32) -> Self;
    fn with_boss_fight(self, is_boss_fight: bool) -> Self;
    fn with_elite_fight(self, is_elite_fight: bool) -> Self;
    fn with_monster_hp(self, monster_id: EntityId, hp: i32) -> Self;
    fn with_monster_max_hp(self, monster_id: EntityId, max_hp: i32) -> Self;
    fn with_monster_type(self, monster_id: EntityId, enemy_id: EnemyId) -> Self;
    fn with_monster_intent(self, monster_id: EntityId, intent: Intent) -> Self;
    fn with_player_power(self, power_id: PowerId, amount: i32) -> Self;
    fn with_monster_power(self, monster_id: EntityId, power_id: PowerId, amount: i32) -> Self;
    fn with_hand_ids(self, ids: &[CardId]) -> Self;
    fn with_draw_ids(self, ids: &[CardId]) -> Self;
    fn with_hand_cards(self, cards: Vec<CombatCard>) -> Self;
    fn with_draw_cards(self, cards: Vec<CombatCard>) -> Self;
    fn with_discard_cards(self, cards: Vec<CombatCard>) -> Self;
    fn with_monsters(self, monsters: Vec<MonsterEntity>) -> Self;
}

impl CombatTestExt for CombatState {
    fn with_player_hp(mut self, hp: i32) -> Self {
        self.entities.player.current_hp = hp.max(0).min(self.entities.player.max_hp);
        self
    }

    fn with_player_max_hp(mut self, max_hp: i32) -> Self {
        self.entities.player.max_hp = max_hp.max(1);
        self.entities.player.current_hp = self
            .entities
            .player
            .current_hp
            .min(self.entities.player.max_hp);
        self
    }

    fn with_player_block(mut self, block: i32) -> Self {
        self.entities.player.block = block.max(0);
        self
    }

    fn with_player_gold(mut self, gold: i32) -> Self {
        self.entities.player.gold = gold.max(0);
        self
    }

    fn with_player_stance(mut self, stance: StanceId) -> Self {
        self.entities.player.stance = stance;
        self
    }

    fn with_energy(mut self, energy: u8) -> Self {
        self.turn.energy = energy;
        self
    }

    fn with_turn_count(mut self, turn_count: i32) -> Self {
        self.turn.turn_count = turn_count.max(0) as u32;
        self
    }

    fn with_rng_seed(mut self, seed: u64) -> Self {
        self.rng = CombatRng::new(crate::rng::RngPool::new(seed));
        self
    }

    fn with_card_uuid_counter(mut self, next_uuid: u32) -> Self {
        self.zones.card_uuid_counter = next_uuid;
        self
    }

    fn with_boss_fight(mut self, is_boss_fight: bool) -> Self {
        self.meta.is_boss_fight = is_boss_fight;
        self
    }

    fn with_elite_fight(mut self, is_elite_fight: bool) -> Self {
        self.meta.is_elite_fight = is_elite_fight;
        self
    }

    fn with_monster_hp(mut self, monster_id: EntityId, hp: i32) -> Self {
        let monster = self
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("monster id missing in test combat");
        monster.current_hp = hp.max(0).min(monster.max_hp);
        self
    }

    fn with_monster_max_hp(mut self, monster_id: EntityId, max_hp: i32) -> Self {
        let monster = self
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("monster id missing in test combat");
        monster.max_hp = max_hp.max(1);
        monster.current_hp = monster.current_hp.min(monster.max_hp);
        self
    }

    fn with_monster_type(mut self, monster_id: EntityId, enemy_id: EnemyId) -> Self {
        let monster = self
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("monster id missing in test combat");
        monster.monster_type = enemy_id as usize;
        self
    }

    fn with_monster_intent(mut self, monster_id: EntityId, intent: Intent) -> Self {
        let monster = self
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("monster id missing in test combat");
        monster.intent_dmg = intent_damage(&intent);
        monster.current_intent = intent;
        self
    }

    fn with_player_power(mut self, power_id: PowerId, amount: i32) -> Self {
        self.entities
            .power_db
            .entry(0)
            .or_default()
            .push(test_power(power_id, amount));
        self.recompute_turn_start_draw_modifier();
        self
    }

    fn with_monster_power(mut self, monster_id: EntityId, power_id: PowerId, amount: i32) -> Self {
        self.entities
            .power_db
            .entry(monster_id)
            .or_default()
            .push(test_power(power_id, amount));
        self
    }

    fn with_hand_ids(mut self, ids: &[CardId]) -> Self {
        let start = self.zones.card_uuid_counter;
        self.zones.hand = ids
            .iter()
            .enumerate()
            .map(|(index, id)| CombatCard::new(*id, start + index as u32))
            .collect();
        self.zones.card_uuid_counter = start + ids.len() as u32;
        self
    }

    fn with_draw_ids(mut self, ids: &[CardId]) -> Self {
        let start = self.zones.card_uuid_counter;
        self.zones.draw_pile = ids
            .iter()
            .enumerate()
            .map(|(index, id)| CombatCard::new(*id, start + index as u32))
            .collect();
        self.zones.card_uuid_counter = start + ids.len() as u32;
        self
    }

    fn with_hand_cards(mut self, cards: Vec<CombatCard>) -> Self {
        self.zones.hand = cards;
        self
    }

    fn with_draw_cards(mut self, cards: Vec<CombatCard>) -> Self {
        self.zones.draw_pile = cards;
        self
    }

    fn with_discard_cards(mut self, cards: Vec<CombatCard>) -> Self {
        self.zones.discard_pile = cards;
        self
    }

    fn with_monsters(mut self, monsters: Vec<MonsterEntity>) -> Self {
        self.entities.monsters = monsters;
        self
    }
}

fn test_power(power_id: PowerId, amount: i32) -> Power {
    Power {
        power_type: power_id,
        instance_id: None,
        amount,
        extra_data: 0,
        just_applied: false,
    }
}

fn intent_damage(intent: &Intent) -> i32 {
    match intent {
        Intent::Attack { damage, .. }
        | Intent::AttackBuff { damage, .. }
        | Intent::AttackDebuff { damage, .. }
        | Intent::AttackDefend { damage, .. } => *damage,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{basic_combat, CombatTestExt};
    use crate::combat::Intent;
    use crate::content::cards::CardId;

    #[test]
    fn basic_combat_has_stable_default_shape() {
        let combat = basic_combat();
        assert_eq!(combat.turn.energy, 3);
        assert_eq!(combat.entities.player.current_hp, 80);
        assert_eq!(combat.entities.monsters.len(), 1);
        assert_eq!(combat.entities.monsters[0].id, 1);
    }

    #[test]
    fn helpers_patch_only_requested_fields() {
        let combat = basic_combat()
            .with_player_hp(50)
            .with_monster_hp(1, 30)
            .with_monster_intent(
                1,
                Intent::Attack {
                    damage: 12,
                    hits: 1,
                },
            )
            .with_hand_ids(&[CardId::Strike, CardId::Defend]);

        assert_eq!(combat.entities.player.current_hp, 50);
        assert_eq!(combat.entities.player.max_hp, 80);
        assert_eq!(combat.entities.monsters[0].current_hp, 30);
        assert_eq!(combat.entities.monsters[0].intent_dmg, 12);
        assert_eq!(combat.zones.hand.len(), 2);
        assert_eq!(combat.zones.draw_pile.len(), 0);
    }
}
