//! Monte Carlo Tree Search (MCTS) 示意实现
//!
//! 这是一个简化的MCTS框架，展示如何利用模拟器的确定性和快速复制特性。
//! 用于面试展示架构设计能力。

use std::collections::HashMap;

use crate::state::GameState;
use crate::ai::features::{get_action_mask, ACTION_DIM};

/// MCTS 节点
#[derive(Clone)]
pub struct MctsNode {
    /// 访问次数
    pub visits: u32,
    /// 累计价值
    pub total_value: f64,
    /// 子节点: action -> node
    pub children: HashMap<usize, MctsNode>,
    /// 是否已完全展开
    pub expanded: bool,
}

impl MctsNode {
    pub fn new() -> Self {
        Self {
            visits: 0,
            total_value: 0.0,
            children: HashMap::new(),
            expanded: false,
        }
    }
    
    /// 计算UCB1分数
    pub fn ucb1(&self, child_visits: u32, child_value: f64, exploration: f64) -> f64 {
        if child_visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = child_value / child_visits as f64;
        let exploration_term = exploration * ((self.visits as f64).ln() / child_visits as f64).sqrt();
        exploitation + exploration_term
    }
    
    /// 选择最佳子节点 (UCB1)
    pub fn select_child(&self, valid_actions: &[bool; ACTION_DIM], exploration: f64) -> usize {
        let mut best_action = 10; // 默认结束回合
        let mut best_ucb = f64::NEG_INFINITY;
        
        for action in 0..ACTION_DIM {
            if !valid_actions[action] {
                continue;
            }
            
            let ucb = if let Some(child) = self.children.get(&action) {
                self.ucb1(child.visits, child.total_value, exploration)
            } else {
                f64::INFINITY // 未探索的节点优先
            };
            
            if ucb > best_ucb {
                best_ucb = ucb;
                best_action = action;
            }
        }
        
        best_action
    }
}

impl Default for MctsNode {
    fn default() -> Self {
        Self::new()
    }
}

/// MCTS 搜索配置
#[derive(Clone)]
pub struct MctsConfig {
    /// 探索常数 (UCB1)
    pub exploration: f64,
    /// 每次搜索的模拟次数
    pub num_simulations: u32,
    /// 最大rollout深度
    pub max_rollout_depth: u32,
    /// 折扣因子
    pub discount: f64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            exploration: 1.414, // sqrt(2)
            num_simulations: 100,
            max_rollout_depth: 50,
            discount: 0.99,
        }
    }
}

/// MCTS 搜索器
pub struct MctsSearcher {
    pub config: MctsConfig,
    pub root: MctsNode,
}

impl MctsSearcher {
    pub fn new(config: MctsConfig) -> Self {
        Self {
            config,
            root: MctsNode::new(),
        }
    }
    
    /// 执行MCTS搜索，返回最佳动作
    /// 
    /// 利用Rust的Clone特性实现状态复制，
    /// 这比Python快100倍以上。
    pub fn search(&mut self, state: &GameState) -> usize {
        let config = self.config.clone();
        let num_sims = config.num_simulations;
        
        for _ in 0..num_sims {
            // 1. 复制状态 (Rust的Clone非常快)
            let sim_state = state.clone();
            
            // 2. Selection + Expansion + Simulation + Backpropagation
            Self::simulate_from_root(
                sim_state,
                &mut self.root,
                &config,
            );
        }
        
        // 选择访问次数最多的动作
        self.best_action()
    }
    
    /// 从根节点开始模拟 (消耗 state 所有权)
    fn simulate_from_root(
        mut state: GameState,
        root: &mut MctsNode,
        config: &MctsConfig,
    ) {
        let value = Self::simulate_recursive(&mut state, root, 0, config);
        let _ = value;
    }
    
    /// 模拟单次rollout (静态方法避免借用冲突)
    fn simulate_recursive(
        state: &mut GameState,
        node: &mut MctsNode,
        depth: u32,
        config: &MctsConfig,
    ) -> f64 {
        // 终止条件
        if state.combat_won() {
            return 1.0;
        }
        if state.combat_lost() {
            return -1.0;
        }
        if depth >= config.max_rollout_depth {
            return 0.0; // 超时视为平局
        }
        
        let valid_actions = get_action_mask(state);
        
        // Selection: 选择动作
        let action = node.select_child(&valid_actions, config.exploration);
        
        // Expansion: 如果是新节点，添加到树中
        if !node.children.contains_key(&action) {
            node.children.insert(action, MctsNode::new());
        }
        
        // 执行动作 (简化版)
        Self::apply_action_static(state, action);
        
        // Simulation: 递归或rollout
        let child = node.children.get_mut(&action).unwrap();
        let value = if child.visits < 5 {
            // 少于5次访问，使用随机rollout
            Self::random_rollout_static(state, depth, config)
        } else {
            // 否则继续树搜索
            Self::simulate_recursive(state, child, depth + 1, config)
        };
        
        // Backpropagation: 更新统计
        child.visits += 1;
        child.total_value += value;
        
        value * config.discount
    }
    
    /// 随机rollout策略 (静态方法)
    fn random_rollout_static(state: &mut GameState, start_depth: u32, config: &MctsConfig) -> f64 {
        let mut depth = start_depth;
        
        while depth < config.max_rollout_depth {
            if state.combat_won() {
                return 1.0;
            }
            if state.combat_lost() {
                return -1.0;
            }
            
            let valid_actions = get_action_mask(state);
            
            // 简单贪心策略：优先打牌
            let action = (0..10)
                .find(|&i| valid_actions[i])
                .unwrap_or(10);
            
            Self::apply_action_static(state, action);
            depth += 1;
        }
        
        0.0
    }
    
    /// 应用动作到状态 (简化版，静态方法)
    fn apply_action_static(state: &mut GameState, action: usize) {
        if action == 10 {
            // 结束回合
            state.end_turn();
            // 简化的敌人回合
            let damage = 10i32.saturating_sub(state.player.block);
            state.player.block = 0;
            state.player.current_hp -= damage.max(0);
            if state.player.current_hp > 0 {
                state.start_turn();
            }
        } else if action < state.hand.len() {
            // 打出手牌 (简化：不实际执行效果)
            let card = state.hand.remove(action);
            state.player.energy -= card.current_cost;
            
            // 简化效果
            match card.definition_id.as_str() {
                "Strike_Ironclad" => {
                    if let Some(e) = state.enemies.first_mut() {
                        e.hp -= 6;
                    }
                }
                "Defend_Ironclad" => {
                    state.player.block += 5;
                }
                "Bash" => {
                    if let Some(e) = state.enemies.first_mut() {
                        e.hp -= 8;
                        e.apply_status("Vulnerable", 2);
                    }
                }
                _ => {}
            }
            
            state.discard_pile.push(card);
        }
    }
    
    /// 返回访问次数最多的动作
    fn best_action(&self) -> usize {
        self.root.children
            .iter()
            .max_by_key(|(_, node)| node.visits)
            .map(|(&action, _)| action)
            .unwrap_or(10)
    }
    /// 获取动作概率分布 (用于训练)
    pub fn get_policy(&self) -> [f32; ACTION_DIM] {
        let mut policy = [0.0f32; ACTION_DIM];
        let total_visits: u32 = self.root.children.values().map(|n| n.visits).sum();
        
        if total_visits > 0 {
            for (&action, node) in &self.root.children {
                policy[action] = node.visits as f32 / total_visits as f32;
            }
        } else {
            policy[10] = 1.0; // 默认结束回合
        }
        
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Enemy;
    use crate::schema::CardInstance;
    
    #[test]
    fn test_mcts_search() {
        let mut state = GameState::new(42);
        state.enemies.push(Enemy::new_simple("Test Enemy", 30));
        
        for _ in 0..5 {
            state.add_to_deck(CardInstance::new("Strike_Ironclad".to_string(), 1));
        }
        for _ in 0..4 {
            state.add_to_deck(CardInstance::new("Defend_Ironclad".to_string(), 1));
        }
        state.add_to_deck(CardInstance::new("Bash".to_string(), 2));
        state.shuffle_draw_pile();
        state.start_turn();
        
        let config = MctsConfig {
            num_simulations: 50,
            ..Default::default()
        };
        let mut searcher = MctsSearcher::new(config);
        
        let action = searcher.search(&state);
        println!("MCTS selected action: {}", action);
        println!("Policy: {:?}", searcher.get_policy());
        
        // 应该选择一个有效动作
        assert!(action <= 10);
    }
}
