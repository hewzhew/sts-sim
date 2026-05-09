use serde::{Deserialize, Serialize};

pub const OBS_LEN: usize = 96;
pub const ACTION_LEN: usize = 11;
pub const END_TURN_ACTION: usize = 10;

const PLAYER_MAX_HP: i32 = 80;
const JAW_WORM_HP: i32 = 40;
const MAX_STEPS: u32 = 80;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroCard {
    Strike,
    Defend,
    Bash,
}

impl MicroCard {
    fn cost(self) -> i32 {
        match self {
            MicroCard::Strike | MicroCard::Defend => 1,
            MicroCard::Bash => 2,
        }
    }

    fn id(self) -> &'static str {
        match self {
            MicroCard::Strike => "strike",
            MicroCard::Defend => "defend",
            MicroCard::Bash => "bash",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JawIntent {
    Attack,
    Buff,
    AttackBlock,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum MicroRequest {
    Reset { seed: Option<u64> },
    Step { action: usize },
    Close,
}

#[derive(Debug, Serialize)]
pub struct MicroResponse {
    pub obs: Vec<f32>,
    pub action_mask: Vec<bool>,
    pub reward: f32,
    pub done: bool,
    pub truncated: bool,
    pub info: MicroInfo,
}

#[derive(Debug, Serialize)]
pub struct MicroInfo {
    pub obs_len: usize,
    pub action_len: usize,
    pub turn: u32,
    pub steps: u32,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub enemy_hp: i32,
    pub enemy_block: i32,
    pub enemy_strength: i32,
    pub enemy_vulnerable: i32,
    pub enemy_intent: JawIntent,
    pub intent_damage: i32,
    pub legal_actions: Vec<usize>,
    pub hand: Vec<&'static str>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub illegal_action: bool,
    pub killed_enemy: bool,
    pub player_dead: bool,
}

#[derive(Clone, Debug)]
pub struct MicroJawWormEnv {
    rng: Lcg,
    turn: u32,
    steps: u32,
    player_hp: i32,
    player_block: i32,
    energy: i32,
    enemy_hp: i32,
    enemy_block: i32,
    enemy_strength: i32,
    enemy_vulnerable: i32,
    enemy_turns_taken: u32,
    draw: Vec<MicroCard>,
    discard: Vec<MicroCard>,
    hand: Vec<MicroCard>,
    done: bool,
    truncated: bool,
}

impl MicroJawWormEnv {
    pub fn new(seed: u64) -> Self {
        let mut env = Self {
            rng: Lcg::new(seed),
            turn: 0,
            steps: 0,
            player_hp: PLAYER_MAX_HP,
            player_block: 0,
            energy: 3,
            enemy_hp: JAW_WORM_HP,
            enemy_block: 0,
            enemy_strength: 0,
            enemy_vulnerable: 0,
            enemy_turns_taken: 0,
            draw: Vec::new(),
            discard: Vec::new(),
            hand: Vec::new(),
            done: false,
            truncated: false,
        };
        env.reset(seed);
        env
    }

    pub fn reset(&mut self, seed: u64) -> MicroResponse {
        self.rng = Lcg::new(seed);
        self.turn = 1;
        self.steps = 0;
        self.player_hp = PLAYER_MAX_HP;
        self.player_block = 0;
        self.energy = 3;
        self.enemy_hp = JAW_WORM_HP;
        self.enemy_block = 0;
        self.enemy_strength = 0;
        self.enemy_vulnerable = 0;
        self.enemy_turns_taken = 0;
        self.done = false;
        self.truncated = false;
        self.discard.clear();
        self.hand.clear();
        self.draw = starter_deck();
        self.rng.shuffle(&mut self.draw);
        self.draw_cards(5);
        self.response(0.0, false)
    }

    pub fn step(&mut self, action: usize) -> MicroResponse {
        if self.done {
            return self.response(0.0, false);
        }

        self.steps += 1;
        let mut reward = -0.01;
        let mut illegal = false;

        if action == END_TURN_ACTION {
            reward += self.end_turn();
        } else if self.is_legal_card_action(action) {
            reward += self.play_card(action);
        } else {
            illegal = true;
            reward -= 10.0;
            reward += self.end_turn();
        }

        if self.enemy_hp <= 0 {
            self.done = true;
            reward += 50.0;
        }

        if self.player_hp <= 0 {
            self.done = true;
            reward -= 50.0;
        }

        if self.steps >= MAX_STEPS && !self.done {
            self.done = true;
            self.truncated = true;
            reward -= 10.0;
        }

        self.response(reward, illegal)
    }

    fn play_card(&mut self, hand_index: usize) -> f32 {
        let card = self.hand.remove(hand_index);
        self.energy -= card.cost();
        let mut reward = 0.0;

        match card {
            MicroCard::Strike => {
                reward += self.deal_damage(6) as f32;
            }
            MicroCard::Defend => {
                self.player_block += 5;
            }
            MicroCard::Bash => {
                reward += self.deal_damage(8) as f32;
                self.enemy_vulnerable += 2;
            }
        }

        self.discard.push(card);
        reward
    }

    fn end_turn(&mut self) -> f32 {
        let mut reward = 0.0;
        self.discard.extend(self.hand.drain(..));
        self.enemy_block = 0;

        match self.current_intent() {
            JawIntent::Attack => {
                reward -= self.take_damage(11 + self.enemy_strength) as f32;
            }
            JawIntent::Buff => {
                self.enemy_strength += 3;
                self.enemy_block += 6;
            }
            JawIntent::AttackBlock => {
                reward -= self.take_damage(7 + self.enemy_strength) as f32;
                self.enemy_block += 5;
            }
        }

        self.enemy_turns_taken += 1;
        if self.enemy_vulnerable > 0 {
            self.enemy_vulnerable -= 1;
        }
        self.turn += 1;
        self.player_block = 0;
        self.energy = 3;
        self.draw_cards(5);
        reward
    }

    fn deal_damage(&mut self, base_damage: i32) -> i32 {
        let damage = if self.enemy_vulnerable > 0 {
            base_damage * 3 / 2
        } else {
            base_damage
        };
        let block_damage = self.enemy_block.min(damage);
        self.enemy_block -= block_damage;
        let hp_damage = (damage - block_damage).min(self.enemy_hp).max(0);
        self.enemy_hp -= hp_damage;
        hp_damage
    }

    fn take_damage(&mut self, damage: i32) -> i32 {
        let blocked = self.player_block.min(damage);
        self.player_block -= blocked;
        let hp_damage = damage - blocked;
        self.player_hp -= hp_damage;
        hp_damage
    }

    fn draw_cards(&mut self, count: usize) {
        for _ in 0..count {
            if self.draw.is_empty() {
                if self.discard.is_empty() {
                    break;
                }
                self.draw.append(&mut self.discard);
                self.rng.shuffle(&mut self.draw);
            }
            if let Some(card) = self.draw.pop() {
                self.hand.push(card);
            }
        }
    }

    fn current_intent(&self) -> JawIntent {
        if self.enemy_turns_taken == 0 {
            JawIntent::Attack
        } else {
            match (self.enemy_turns_taken - 1) % 3 {
                0 => JawIntent::Buff,
                1 => JawIntent::AttackBlock,
                _ => JawIntent::Attack,
            }
        }
    }

    fn intent_damage(&self) -> i32 {
        match self.current_intent() {
            JawIntent::Attack => 11 + self.enemy_strength,
            JawIntent::Buff => 0,
            JawIntent::AttackBlock => 7 + self.enemy_strength,
        }
    }

    fn is_legal_card_action(&self, action: usize) -> bool {
        self.hand
            .get(action)
            .is_some_and(|card| card.cost() <= self.energy)
    }

    fn action_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; ACTION_LEN];
        if self.done {
            return mask;
        }

        for (idx, card) in self.hand.iter().take(END_TURN_ACTION).enumerate() {
            mask[idx] = card.cost() <= self.energy;
        }
        mask[END_TURN_ACTION] = true;
        mask
    }

    fn response(&self, reward: f32, illegal_action: bool) -> MicroResponse {
        let action_mask = self.action_mask();
        let legal_actions = action_mask
            .iter()
            .enumerate()
            .filter_map(|(idx, legal)| legal.then_some(idx))
            .collect();
        MicroResponse {
            obs: self.obs(),
            action_mask,
            reward,
            done: self.done,
            truncated: self.truncated,
            info: MicroInfo {
                obs_len: OBS_LEN,
                action_len: ACTION_LEN,
                turn: self.turn,
                steps: self.steps,
                player_hp: self.player_hp,
                player_block: self.player_block,
                energy: self.energy,
                enemy_hp: self.enemy_hp,
                enemy_block: self.enemy_block,
                enemy_strength: self.enemy_strength,
                enemy_vulnerable: self.enemy_vulnerable,
                enemy_intent: self.current_intent(),
                intent_damage: self.intent_damage(),
                legal_actions,
                hand: self.hand.iter().map(|card| card.id()).collect(),
                draw_count: self.draw.len(),
                discard_count: self.discard.len(),
                illegal_action,
                killed_enemy: self.enemy_hp <= 0,
                player_dead: self.player_hp <= 0,
            },
        }
    }

    fn obs(&self) -> Vec<f32> {
        let mut obs = vec![0.0; OBS_LEN];
        obs[0] = self.player_hp as f32 / PLAYER_MAX_HP as f32;
        obs[1] = self.player_block as f32 / 50.0;
        obs[2] = self.energy as f32 / 3.0;
        obs[3] = self.enemy_hp as f32 / JAW_WORM_HP as f32;
        obs[4] = self.enemy_block as f32 / 50.0;
        obs[5] = self.enemy_strength as f32 / 20.0;
        match self.current_intent() {
            JawIntent::Attack => obs[6] = 1.0,
            JawIntent::Buff => obs[7] = 1.0,
            JawIntent::AttackBlock => obs[8] = 1.0,
        }
        obs[9] = self.intent_damage() as f32 / 30.0;
        obs[10] = self.enemy_vulnerable as f32 / 5.0;
        obs[11] = self.draw.len() as f32 / 10.0;
        obs[12] = self.discard.len() as f32 / 10.0;
        obs[13] = self.hand.len() as f32 / 10.0;
        obs[14] = self.turn as f32 / 20.0;

        let draw_counts = card_counts(&self.draw);
        let discard_counts = card_counts(&self.discard);
        obs[15] = draw_counts.0 as f32 / 5.0;
        obs[16] = draw_counts.1 as f32 / 4.0;
        obs[17] = draw_counts.2 as f32;
        obs[18] = discard_counts.0 as f32 / 5.0;
        obs[19] = discard_counts.1 as f32 / 4.0;
        obs[20] = discard_counts.2 as f32;

        for slot in 0..END_TURN_ACTION {
            let base = 24 + slot * 6;
            if let Some(card) = self.hand.get(slot).copied() {
                obs[base] = 1.0;
                match card {
                    MicroCard::Strike => obs[base + 1] = 1.0,
                    MicroCard::Defend => obs[base + 2] = 1.0,
                    MicroCard::Bash => obs[base + 3] = 1.0,
                }
                obs[base + 4] = card.cost() as f32 / 3.0;
                obs[base + 5] = (card.cost() <= self.energy) as u8 as f32;
            }
        }

        obs
    }
}

fn starter_deck() -> Vec<MicroCard> {
    let mut deck = Vec::with_capacity(10);
    deck.extend(std::iter::repeat(MicroCard::Strike).take(5));
    deck.extend(std::iter::repeat(MicroCard::Defend).take(4));
    deck.push(MicroCard::Bash);
    deck
}

fn card_counts(cards: &[MicroCard]) -> (usize, usize, usize) {
    cards.iter().fold((0, 0, 0), |mut counts, card| {
        match card {
            MicroCard::Strike => counts.0 += 1,
            MicroCard::Defend => counts.1 += 1,
            MicroCard::Bash => counts.2 += 1,
        }
        counts
    })
}

#[derive(Clone, Debug)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = (self.next_u64() as usize) % (i + 1);
            values.swap(i, j);
        }
    }
}
