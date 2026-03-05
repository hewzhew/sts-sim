//! Feature engineering for RL - 观测向量化
//!
//! 将游戏状态编码为神经网络可用的固定长度浮点向量。
//! 这是PPO/MuZero等算法的关键输入层。

use crate::state::GameState;

/// 观测向量维度
pub const OBS_DIM: usize = 128;

/// 动作空间大小 (10张手牌 + 结束回合)
pub const ACTION_DIM: usize = 11;

/// 将游戏状态编码为固定长度的观测向量
/// 
/// ## 特征设计
/// - [0-9]: 手牌 one-hot 编码 (是否有牌)
/// - [10-19]: 手牌费用 (归一化)
/// - [20-29]: 手牌类型 (攻击=1, 技能=0.5, 能力=0)
/// - [30]: 当前能量 / 最大能量
/// - [31]: 当前生命 / 最大生命
/// - [32]: 当前格挡 / 50 (归一化)
/// - [33]: 力量 / 10
/// - [34]: 敏捷 / 10
/// - [35]: 虚弱状态 (0/1)
/// - [36]: 脆弱状态 (0/1)
/// - [40-49]: 敌人1生命/格挡/状态
/// - [50-59]: 敌人2生命/格挡/状态
/// - [60-69]: 敌人3生命/格挡/状态
/// - [70-79]: 敌人4生命/格挡/状态
/// - [80-99]: 抽牌堆组成 (卡牌类型分布)
/// - [100-119]: 弃牌堆组成
/// - [120-127]: 战斗统计 (回合数、打出的牌数等)
pub fn encode_observation(state: &GameState) -> [f32; OBS_DIM] {
    let mut obs = [0.0f32; OBS_DIM];
    
    // === 手牌特征 [0-29] ===
    for (i, card) in state.hand.iter().enumerate().take(10) {
        // 是否有牌
        obs[i] = 1.0;
        // 费用归一化
        obs[10 + i] = card.current_cost as f32 / 5.0;
        // 卡牌类型 (简化：按费用猜测)
        obs[20 + i] = if card.current_cost >= 2 { 1.0 } else { 0.5 };
    }
    
    // === 玩家状态 [30-39] ===
    obs[30] = state.player.energy as f32 / state.player.max_energy as f32;
    obs[31] = state.player.current_hp as f32 / state.player.max_hp as f32;
    obs[32] = (state.player.block as f32 / 50.0).min(1.0);
    obs[33] = (state.player.strength() as f32 / 10.0).clamp(-1.0, 1.0);
    obs[34] = (state.player.dexterity() as f32 / 10.0).clamp(-1.0, 1.0);
    obs[35] = if state.player.is_weak() { 1.0 } else { 0.0 };
    obs[36] = if state.player.has_status("Frail") { 1.0 } else { 0.0 };
    
    // === 敌人状态 [40-79] ===
    for (i, enemy) in state.enemies.iter().enumerate().take(4) {
        let base = 40 + i * 10;
        obs[base] = if enemy.is_dead() { 0.0 } else { 1.0 };
        obs[base + 1] = enemy.hp as f32 / enemy.max_hp.max(1) as f32;
        obs[base + 2] = (enemy.block as f32 / 30.0).min(1.0);
        obs[base + 3] = (enemy.get_buff("Vulnerable") as f32 / 5.0).min(1.0);
        obs[base + 4] = (enemy.get_buff("Weak") as f32 / 5.0).min(1.0);
        obs[base + 5] = (enemy.get_buff("Poison") as f32 / 20.0).min(1.0);
        // [base+6..base+9] 预留给意图编码
    }
    
    // === 牌堆统计 [80-99] ===
    let draw_size = state.draw_pile.len() as f32;
    let discard_size = state.discard_pile.len() as f32;
    obs[80] = (draw_size / 30.0).min(1.0);
    obs[90] = (discard_size / 30.0).min(1.0);
    
    // === 战斗统计 [120-127] ===
    obs[120] = (state.turn as f32 / 20.0).min(1.0);
    obs[121] = (state.cards_played_this_turn as f32 / 10.0).min(1.0);
    
    obs
}

/// 生成有效动作掩码
pub fn get_action_mask(state: &GameState) -> [bool; ACTION_DIM] {
    let mut mask = [false; ACTION_DIM];
    
    for (i, card) in state.hand.iter().enumerate().take(10) {
        mask[i] = card.current_cost <= state.player.energy;
    }
    
    // 结束回合总是有效
    mask[10] = true;
    
    mask
}

/// 奖励塑形 (Reward Shaping)
/// 
/// 除了胜负奖励外，添加中间奖励引导学习：
/// - 造成伤害: +0.01 per damage
/// - 获得格挡: +0.005 per block
/// - 施加debuff: +0.02 per stack
/// - 受到伤害: -0.02 per damage
/// - 死亡: -10.0
/// - 胜利: +10.0
pub struct RewardShaper {
    last_enemy_hp: i32,
    last_player_hp: i32,
    last_player_block: i32,
}

impl RewardShaper {
    pub fn new(state: &GameState) -> Self {
        Self {
            last_enemy_hp: state.enemies.iter().map(|e| e.hp).sum(),
            last_player_hp: state.player.current_hp,
            last_player_block: state.player.block,
        }
    }
    
    /// 计算塑形奖励
    pub fn compute_reward(&mut self, state: &GameState, done: bool, won: bool) -> f32 {
        let mut reward = 0.0;
        
        // 终局奖励
        if done {
            reward += if won { 10.0 } else { -10.0 };
            return reward;
        }
        
        // 伤害奖励
        let current_enemy_hp: i32 = state.enemies.iter().map(|e| e.hp).sum();
        let damage_dealt = self.last_enemy_hp - current_enemy_hp;
        reward += damage_dealt as f32 * 0.01;
        self.last_enemy_hp = current_enemy_hp;
        
        // 受伤惩罚
        let hp_lost = self.last_player_hp - state.player.current_hp;
        reward -= hp_lost as f32 * 0.02;
        self.last_player_hp = state.player.current_hp;
        
        // 格挡奖励
        let block_gained = state.player.block - self.last_player_block;
        if block_gained > 0 {
            reward += block_gained as f32 * 0.005;
        }
        self.last_player_block = state.player.block;
        
        reward
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_observation() {
        let state = GameState::new(42);
        let obs = encode_observation(&state);
        
        assert_eq!(obs.len(), OBS_DIM);
        // 初始状态能量应该满
        assert!((obs[30] - 1.0).abs() < 0.01);
    }
}
