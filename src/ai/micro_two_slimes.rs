use serde::{Deserialize, Serialize};

pub const OBS_LEN: usize = 128;
pub const HAND_SLOTS: usize = 10;
pub const TARGET_SLOTS: usize = 2;
pub const CARD_ACTION_COUNT: usize = HAND_SLOTS * TARGET_SLOTS;
pub const END_TURN_ACTION: usize = CARD_ACTION_COUNT;
pub const ACTION_LEN: usize = CARD_ACTION_COUNT + 1;

const PLAYER_MAX_HP: i32 = 80;
const MAX_STEPS: u32 = 90;

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

    fn needs_target(self) -> bool {
        matches!(self, MicroCard::Strike | MicroCard::Bash)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlimeKind {
    Green,
    Blue,
}

impl SlimeKind {
    fn attack_damage(self) -> i32 {
        match self {
            SlimeKind::Green => 8,
            SlimeKind::Blue => 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlimeIntent {
    Attack,
    Idle,
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
    pub enemies: Vec<EnemyInfo>,
    pub legal_actions: Vec<usize>,
    pub hand: Vec<&'static str>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub illegal_action: bool,
    pub killed_all: bool,
    pub player_dead: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnemyInfo {
    pub slot: usize,
    pub kind: SlimeKind,
    pub hp: i32,
    pub max_hp: i32,
    pub vulnerable: i32,
    pub intent: SlimeIntent,
    pub intent_damage: i32,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug)]
struct Enemy {
    kind: SlimeKind,
    hp: i32,
    max_hp: i32,
    vulnerable: i32,
    phase: u32,
}

impl Enemy {
    fn alive(self) -> bool {
        self.hp > 0
    }

    fn intent(self, turn: u32) -> SlimeIntent {
        if !self.alive() {
            return SlimeIntent::Idle;
        }
        if (turn + self.phase).is_multiple_of(2) {
            SlimeIntent::Attack
        } else {
            SlimeIntent::Idle
        }
    }

    fn intent_damage(self, turn: u32) -> i32 {
        match self.intent(turn) {
            SlimeIntent::Attack => self.kind.attack_damage(),
            SlimeIntent::Idle => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MicroTwoSlimesEnv {
    rng: Lcg,
    turn: u32,
    steps: u32,
    player_hp: i32,
    player_block: i32,
    energy: i32,
    enemies: [Enemy; TARGET_SLOTS],
    draw: Vec<MicroCard>,
    discard: Vec<MicroCard>,
    hand: Vec<MicroCard>,
    done: bool,
    truncated: bool,
}

impl MicroTwoSlimesEnv {
    pub fn new(seed: u64) -> Self {
        let mut env = Self {
            rng: Lcg::new(seed),
            turn: 0,
            steps: 0,
            player_hp: PLAYER_MAX_HP,
            player_block: 0,
            energy: 3,
            enemies: default_enemies(),
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
        self.enemies = self.roll_enemies();
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
        } else if let Some((hand_index, target_index)) = decode_card_action(action) {
            if self.is_legal_card_action(hand_index, target_index) {
                reward += self.play_card(hand_index, target_index);
            } else {
                illegal = true;
                reward -= 10.0;
                reward += self.end_turn();
            }
        } else {
            illegal = true;
            reward -= 10.0;
            reward += self.end_turn();
        }

        if self.killed_all() {
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

    fn roll_enemies(&mut self) -> [Enemy; TARGET_SLOTS] {
        let green_hp = 18 + (self.rng.next_u64() % 7) as i32;
        let blue_hp = 20 + (self.rng.next_u64() % 7) as i32;
        let phase = (self.rng.next_u64() % 2) as u32;
        let mut enemies = [
            Enemy {
                kind: SlimeKind::Green,
                hp: green_hp,
                max_hp: green_hp,
                vulnerable: 0,
                phase,
            },
            Enemy {
                kind: SlimeKind::Blue,
                hp: blue_hp,
                max_hp: blue_hp,
                vulnerable: 0,
                phase: 1 - phase,
            },
        ];
        if self.rng.next_u64().is_multiple_of(2) {
            enemies.swap(0, 1);
        }
        enemies
    }

    fn play_card(&mut self, hand_index: usize, target_index: usize) -> f32 {
        let card = self.hand.remove(hand_index);
        self.energy -= card.cost();
        let mut reward = 0.0;

        match card {
            MicroCard::Strike => {
                reward += self.deal_damage(target_index, 6) as f32;
            }
            MicroCard::Defend => {
                self.player_block += 5;
            }
            MicroCard::Bash => {
                reward += self.deal_damage(target_index, 8) as f32;
                if let Some(enemy) = self.enemies.get_mut(target_index) {
                    if enemy.alive() {
                        enemy.vulnerable += 2;
                    }
                }
            }
        }

        self.discard.push(card);
        reward
    }

    fn end_turn(&mut self) -> f32 {
        let mut reward = 0.0;
        self.discard.extend(self.hand.drain(..));

        for enemy in self.enemies {
            if enemy.intent(self.turn) == SlimeIntent::Attack {
                reward -= self.take_damage(enemy.kind.attack_damage()) as f32;
            }
        }

        for enemy in &mut self.enemies {
            if enemy.vulnerable > 0 {
                enemy.vulnerable -= 1;
            }
        }
        self.turn += 1;
        self.player_block = 0;
        self.energy = 3;
        self.draw_cards(5);
        reward
    }

    fn deal_damage(&mut self, target_index: usize, base_damage: i32) -> i32 {
        let Some(enemy) = self.enemies.get_mut(target_index) else {
            return 0;
        };
        if !enemy.alive() {
            return 0;
        }
        let damage = if enemy.vulnerable > 0 {
            base_damage * 3 / 2
        } else {
            base_damage
        };
        let hp_damage = damage.min(enemy.hp).max(0);
        enemy.hp -= hp_damage;
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

    fn is_legal_card_action(&self, hand_index: usize, target_index: usize) -> bool {
        let Some(card) = self.hand.get(hand_index).copied() else {
            return false;
        };
        if card.cost() > self.energy {
            return false;
        }
        if !card.needs_target() {
            return target_index == 0;
        }
        self.enemies
            .get(target_index)
            .is_some_and(|enemy| enemy.alive())
    }

    fn action_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; ACTION_LEN];
        if self.done {
            return mask;
        }

        for hand_index in 0..HAND_SLOTS {
            for target_index in 0..TARGET_SLOTS {
                if self.is_legal_card_action(hand_index, target_index) {
                    mask[encode_card_action(hand_index, target_index)] = true;
                }
            }
        }
        mask[END_TURN_ACTION] = true;
        mask
    }

    fn killed_all(&self) -> bool {
        self.enemies.iter().all(|enemy| !enemy.alive())
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
                enemies: self.enemy_info(),
                legal_actions,
                hand: self.hand.iter().map(|card| card.id()).collect(),
                draw_count: self.draw.len(),
                discard_count: self.discard.len(),
                illegal_action,
                killed_all: self.killed_all(),
                player_dead: self.player_hp <= 0,
            },
        }
    }

    fn enemy_info(&self) -> Vec<EnemyInfo> {
        self.enemies
            .iter()
            .enumerate()
            .map(|(slot, enemy)| EnemyInfo {
                slot,
                kind: enemy.kind,
                hp: enemy.hp,
                max_hp: enemy.max_hp,
                vulnerable: enemy.vulnerable,
                intent: enemy.intent(self.turn),
                intent_damage: enemy.intent_damage(self.turn),
                alive: enemy.alive(),
            })
            .collect()
    }

    fn obs(&self) -> Vec<f32> {
        let mut obs = vec![0.0; OBS_LEN];
        obs[0] = self.player_hp as f32 / PLAYER_MAX_HP as f32;
        obs[1] = self.player_block as f32 / 50.0;
        obs[2] = self.energy as f32 / 3.0;
        obs[3] = self.turn as f32 / 20.0;
        obs[4] = self.draw.len() as f32 / 10.0;
        obs[5] = self.discard.len() as f32 / 10.0;
        obs[6] = self.hand.len() as f32 / 10.0;

        let draw_counts = card_counts(&self.draw);
        let discard_counts = card_counts(&self.discard);
        obs[8] = draw_counts.0 as f32 / 5.0;
        obs[9] = draw_counts.1 as f32 / 4.0;
        obs[10] = draw_counts.2 as f32;
        obs[11] = discard_counts.0 as f32 / 5.0;
        obs[12] = discard_counts.1 as f32 / 4.0;
        obs[13] = discard_counts.2 as f32;

        for slot in 0..TARGET_SLOTS {
            let enemy = self.enemies[slot];
            let base = 16 + slot * 12;
            obs[base] = 1.0;
            obs[base + 1] = enemy.alive() as u8 as f32;
            obs[base + 2] = enemy.hp.max(0) as f32 / 30.0;
            obs[base + 3] = enemy.max_hp as f32 / 30.0;
            obs[base + 4] = enemy.vulnerable as f32 / 5.0;
            match enemy.intent(self.turn) {
                SlimeIntent::Attack => obs[base + 5] = 1.0,
                SlimeIntent::Idle => obs[base + 6] = 1.0,
            }
            obs[base + 7] = enemy.intent_damage(self.turn) as f32 / 20.0;
            match enemy.kind {
                SlimeKind::Green => obs[base + 8] = 1.0,
                SlimeKind::Blue => obs[base + 9] = 1.0,
            }
        }

        for slot in 0..HAND_SLOTS {
            let base = 48 + slot * 6;
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

fn encode_card_action(hand_index: usize, target_index: usize) -> usize {
    hand_index * TARGET_SLOTS + target_index
}

fn decode_card_action(action: usize) -> Option<(usize, usize)> {
    if action >= CARD_ACTION_COUNT {
        return None;
    }
    Some((action / TARGET_SLOTS, action % TARGET_SLOTS))
}

fn default_enemies() -> [Enemy; TARGET_SLOTS] {
    [
        Enemy {
            kind: SlimeKind::Green,
            hp: 20,
            max_hp: 20,
            vulnerable: 0,
            phase: 0,
        },
        Enemy {
            kind: SlimeKind::Blue,
            hp: 22,
            max_hp: 22,
            vulnerable: 0,
            phase: 1,
        },
    ]
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
            state: seed ^ 0x517c_c1b7_2722_0a95,
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
