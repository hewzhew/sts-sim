#!/usr/bin/env python3
"""
PPO Training Script for StS Simulator
======================================
Uses PyTorch + PPO with proper action masking, MLP policy,
GAE advantage estimation, and evaluation callbacks.
"""

import time
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.distributions import Categorical

import sts_sim
sts_sim.set_verbose(False)  # Suppress Rust game logs


# ==============================================================================
# Actor-Critic MLP Network
# ==============================================================================

class ActorCritic(nn.Module):
    """MLP Actor-Critic with separate policy and value heads."""
    
    def __init__(self, obs_dim: int, act_dim: int, hidden=(256, 128)):
        super().__init__()
        # Shared feature extractor
        layers = []
        in_dim = obs_dim
        for h in hidden:
            layers.append(nn.Linear(in_dim, h))
            layers.append(nn.ReLU())
            in_dim = h
        self.shared = nn.Sequential(*layers)
        
        # Policy head
        self.policy_head = nn.Linear(in_dim, act_dim)
        
        # Value head
        self.value_head = nn.Linear(in_dim, 1)
        
        # Orthogonal initialization
        for m in self.modules():
            if isinstance(m, nn.Linear):
                nn.init.orthogonal_(m.weight, gain=np.sqrt(2))
                nn.init.zeros_(m.bias)
        # Smaller init for policy head (encourage exploration)
        nn.init.orthogonal_(self.policy_head.weight, gain=0.01)
    
    def forward(self, obs):
        features = self.shared(obs)
        return self.policy_head(features), self.value_head(features).squeeze(-1)
    
    def get_action(self, obs, mask):
        """Sample action from masked policy."""
        logits, value = self.forward(obs)
        logits[~mask] = -1e8
        dist = Categorical(logits=logits)
        action = dist.sample()
        return action, dist.log_prob(action), value
    
    def evaluate(self, obs, actions, masks):
        """Evaluate actions for PPO loss computation."""
        logits, values = self.forward(obs)
        logits[~masks] = -1e8
        dist = Categorical(logits=logits)
        log_probs = dist.log_prob(actions)
        entropy = dist.entropy()
        return log_probs, values, entropy


# ==============================================================================
# Rollout Buffer
# ==============================================================================

class RolloutBuffer:
    """Stores rollout data for PPO updates."""
    
    def __init__(self):
        self.obs = []
        self.actions = []
        self.log_probs = []
        self.rewards = []
        self.values = []
        self.dones = []
        self.masks = []
    
    def add(self, obs, action, log_prob, reward, value, done, mask):
        self.obs.append(obs)
        self.actions.append(action)
        self.log_probs.append(log_prob)
        self.rewards.append(reward)
        self.values.append(value)
        self.dones.append(done)
        self.masks.append(mask)
    
    def compute_returns_and_advantages(self, last_value, gamma=0.99, gae_lambda=0.95):
        """Compute GAE advantages and discounted returns."""
        values = self.values + [last_value]
        T = len(self.rewards)
        advantages = np.zeros(T, dtype=np.float32)
        last_gae = 0.0
        
        for t in reversed(range(T)):
            next_non_terminal = 1.0 - float(self.dones[t])
            delta = self.rewards[t] + gamma * values[t + 1] * next_non_terminal - values[t]
            last_gae = delta + gamma * gae_lambda * next_non_terminal * last_gae
            advantages[t] = last_gae
        
        returns = advantages + np.array(self.values, dtype=np.float32)
        return advantages, returns
    
    def get_tensors(self, advantages, returns, device='cpu'):
        """Convert buffer to pytorch tensors."""
        return (
            torch.tensor(np.array(self.obs), dtype=torch.float32, device=device),
            torch.tensor(np.array(self.actions), dtype=torch.long, device=device),
            torch.tensor(np.array(self.log_probs), dtype=torch.float32, device=device),
            torch.tensor(advantages, dtype=torch.float32, device=device),
            torch.tensor(returns, dtype=torch.float32, device=device),
            torch.tensor(np.array(self.masks), dtype=torch.bool, device=device),
        )
    
    def clear(self):
        self.obs.clear()
        self.actions.clear()
        self.log_probs.clear()
        self.rewards.clear()
        self.values.clear()
        self.dones.clear()
        self.masks.clear()
    
    def __len__(self):
        return len(self.obs)


# ==============================================================================
# PPO Update
# ==============================================================================

def ppo_update(model, optimizer, buffer, advantages, returns,
               clip_eps=0.2, vf_coef=0.5, ent_coef=0.01,
               n_epochs=4, batch_size=64, device='cpu'):
    """Perform PPO clipped objective update."""
    obs, actions, old_log_probs, advs, rets, masks = buffer.get_tensors(
        advantages, returns, device
    )
    
    # Normalize advantages
    if advs.std() > 1e-8:
        advs = (advs - advs.mean()) / (advs.std() + 1e-8)
    
    total_loss = 0.0
    n_updates = 0
    
    for epoch in range(n_epochs):
        # Minibatch update
        indices = torch.randperm(len(obs))
        for start in range(0, len(obs), batch_size):
            end = min(start + batch_size, len(obs))
            mb_idx = indices[start:end]
            
            mb_obs = obs[mb_idx]
            mb_actions = actions[mb_idx]
            mb_old_log_probs = old_log_probs[mb_idx]
            mb_advs = advs[mb_idx]
            mb_rets = rets[mb_idx]
            mb_masks = masks[mb_idx]
            
            # Forward pass
            new_log_probs, new_values, entropy = model.evaluate(
                mb_obs, mb_actions, mb_masks
            )
            
            # Policy loss (clipped)
            ratio = torch.exp(new_log_probs - mb_old_log_probs)
            surr1 = ratio * mb_advs
            surr2 = torch.clamp(ratio, 1 - clip_eps, 1 + clip_eps) * mb_advs
            policy_loss = -torch.min(surr1, surr2).mean()
            
            # Value loss (clipped)
            value_loss = F.mse_loss(new_values, mb_rets)
            
            # Entropy bonus
            entropy_loss = -entropy.mean()
            
            # Total loss
            loss = policy_loss + vf_coef * value_loss + ent_coef * entropy_loss
            
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 0.5)
            optimizer.step()
            
            total_loss += loss.item()
            n_updates += 1
    
    return total_loss / max(n_updates, 1)


# ==============================================================================
# Episode Runner
# ==============================================================================

def collect_rollout(env, model, buffer, max_steps=500, device='cpu'):
    """Collect one episode of rollout data."""
    episode_reward = 0.0
    steps = 0
    
    for step in range(max_steps):
        screen = env.get_screen_type()
        if screen == "GAME_OVER":
            break
        
        obs = np.array(env.get_observation(), dtype=np.float32)
        mask = np.array(env.get_valid_actions_mask(), dtype=bool)
        
        if not mask.any():
            done, reward = env.step(99)
            episode_reward += reward
            if done:
                break
            continue
        
        obs_t = torch.tensor(obs, dtype=torch.float32, device=device).unsqueeze(0)
        mask_t = torch.tensor(mask, dtype=torch.bool, device=device).unsqueeze(0)
        
        with torch.no_grad():
            action, log_prob, value = model.get_action(obs_t, mask_t)
        
        action_int = action.item()
        log_prob_val = log_prob.item()
        value_val = value.item()
        
        done, reward = env.step(action_int)
        
        buffer.add(obs, action_int, log_prob_val, reward, value_val, done, mask)
        episode_reward += reward
        steps += 1
        
        if done:
            break
    
    return episode_reward, steps, env.get_hp()


# ==============================================================================
# Evaluation
# ==============================================================================

def evaluate(model, n_episodes=50, max_steps=500, device='cpu'):
    """Evaluate model without gradient computation."""
    rewards = []
    hps = []
    
    for i in range(n_episodes):
        env = sts_sim.PyStsSim(seed=10000 + i)
        env.reset(seed=10000 + i)
        
        total_reward = 0.0
        for step in range(max_steps):
            screen = env.get_screen_type()
            if screen == "GAME_OVER":
                break
            
            obs = np.array(env.get_observation(), dtype=np.float32)
            mask = np.array(env.get_valid_actions_mask(), dtype=bool)
            
            if not mask.any():
                done, reward = env.step(99)
                total_reward += reward
                if done:
                    break
                continue
            
            obs_t = torch.tensor(obs, dtype=torch.float32, device=device).unsqueeze(0)
            mask_t = torch.tensor(mask, dtype=torch.bool, device=device).unsqueeze(0)
            
            with torch.no_grad():
                logits, _ = model(obs_t)
                logits[~mask_t] = -1e8
                action = logits.argmax(dim=-1).item()
            
            done, reward = env.step(action)
            total_reward += reward
            if done:
                break
        
        rewards.append(total_reward)
        hps.append(env.get_hp())
    
    return np.mean(rewards), np.std(rewards), np.mean(hps)


def evaluate_random(n_episodes=50, max_steps=500):
    """Random baseline for comparison."""
    rewards = []
    for i in range(n_episodes):
        env = sts_sim.PyStsSim(seed=10000 + i)
        env.reset(seed=10000 + i)
        
        total_reward = 0.0
        for step in range(max_steps):
            if env.get_screen_type() == "GAME_OVER":
                break
            mask = np.array(env.get_valid_actions_mask(), dtype=bool)
            valid = np.where(mask)[0]
            action = np.random.choice(valid) if len(valid) > 0 else 99
            done, reward = env.step(int(action))
            total_reward += reward
            if done:
                break
        rewards.append(total_reward)
    return np.mean(rewards), np.std(rewards)


# ==============================================================================
# Main Training Loop
# ==============================================================================

def main():
    print("=" * 60)
    print("STS Simulator — PPO Training")
    print("=" * 60)
    
    # CPU is faster than GPU for this small network (3.9 vs 1.6 ep/s)
    # GPU would help with larger networks (1024+ hidden) or batch inference
    device = 'cpu'
    print(f"  Device: {device} (GPU available: {torch.cuda.is_available()})")
    
    # Get dimensions
    env = sts_sim.PyStsSim(seed=0)
    env.reset()
    obs_dim = len(env.get_observation())
    act_dim = len(env.get_valid_actions_mask())
    print(f"  Obs dim: {obs_dim}, Act dim: {act_dim}")
    
    # Random baseline
    print("\n[Baseline] Random agent (50 episodes)...")
    rand_r, rand_std = evaluate_random(50)
    print(f"  Random: {rand_r:+.2f} +/- {rand_std:.2f}")
    
    # Model
    model = ActorCritic(obs_dim, act_dim, hidden=(256, 128)).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=3e-4, eps=1e-5)
    
    param_count = sum(p.numel() for p in model.parameters())
    print(f"  Model params: {param_count:,}")
    
    # Training config
    TOTAL_EPISODES = 3000
    ROLLOUT_EPISODES = 16  # Collect 16 episodes before each PPO update
    EVAL_INTERVAL = 300    # Evaluate every N episodes
    PATIENCE = 3           # Early stop after N evals with no improvement
    
    print(f"\n[Training] {TOTAL_EPISODES} episodes, rollout={ROLLOUT_EPISODES}, "
          f"eval every {EVAL_INTERVAL}, patience={PATIENCE}")
    print("-" * 60)
    
    buffer = RolloutBuffer()
    episode_count = 0
    total_steps = 0
    t0 = time.time()
    
    recent_rewards = []
    best_eval_reward = -float('inf')
    no_improve_count = 0
    
    while episode_count < TOTAL_EPISODES:
        # Collect rollout
        buffer.clear()
        batch_rewards = []
        batch_steps = []
        
        for ep in range(ROLLOUT_EPISODES):
            seed = episode_count + ep + 2000  # Offset from eval seeds
            env = sts_sim.PyStsSim(seed=seed)
            env.reset(seed=seed)
            
            ep_reward, ep_steps, ep_hp = collect_rollout(
                env, model, buffer, max_steps=500, device=device
            )
            batch_rewards.append(ep_reward)
            batch_steps.append(ep_steps)
        
        episode_count += ROLLOUT_EPISODES
        
        # Compute advantages
        last_value = 0.0  # Terminal
        advantages, returns = buffer.compute_returns_and_advantages(last_value)
        
        if len(buffer) > 0:
            # PPO update
            loss = ppo_update(
                model, optimizer, buffer, advantages, returns,
                clip_eps=0.2, vf_coef=0.5, ent_coef=0.01,
                n_epochs=4, batch_size=64, device=device
            )
        else:
            loss = 0.0
        
        total_steps += sum(batch_steps)
        avg_r = np.mean(batch_rewards)
        recent_rewards.append(avg_r)
        
        elapsed = time.time() - t0
        eps_per_sec = episode_count / elapsed
        
        # Log every N episodes
        if episode_count % 80 == 0 or episode_count >= TOTAL_EPISODES:
            print(f"  Ep {episode_count:4d}/{TOTAL_EPISODES} | "
                  f"Avg R: {avg_r:+7.2f} | "
                  f"Loss: {loss:.4f} | "
                  f"Steps: {total_steps:6d} | "
                  f"{eps_per_sec:.1f} ep/s")
        
        # Evaluate periodically
        if episode_count % EVAL_INTERVAL == 0:
            eval_r, eval_std, eval_hp = evaluate(model, n_episodes=30, device=device)
            improved = eval_r > best_eval_reward
            tag = " ★ NEW BEST" if improved else ""
            if improved:
                best_eval_reward = eval_r
                no_improve_count = 0
                torch.save(model.state_dict(), 'scripts/best_ppo.pt')
            else:
                no_improve_count += 1
            print(f"  [EVAL] R: {eval_r:+.1f}±{eval_std:.1f} | "
                  f"HP: {eval_hp:.1f} | vs Rand: {eval_r - rand_r:+.1f} | "
                  f"patience: {PATIENCE - no_improve_count}{tag}")
            
            if no_improve_count >= PATIENCE:
                print(f"  *** Early stopping: {PATIENCE} evals without improvement ***")
                break
    
    elapsed = time.time() - t0
    
    # Final evaluation
    print("\n" + "=" * 60)
    print("[Final Evaluation] 100 episodes")
    eval_r, eval_std, eval_hp = evaluate(model, n_episodes=100, device=device)
    print(f"  Trained agent:  {eval_r:+.2f} +/- {eval_std:.2f} (HP: {eval_hp:.1f})")
    print(f"  Random agent:   {rand_r:+.2f} +/- {rand_std:.2f}")
    print(f"  vs Random:      {eval_r - rand_r:+.2f}")
    print(f"  Best eval:      {best_eval_reward:+.2f}")
    print(f"  Total time:     {elapsed:.1f}s ({TOTAL_EPISODES / elapsed:.1f} ep/s)")
    print(f"  Total steps:    {total_steps:,}")
    
    if eval_r > rand_r + 2.0:
        print("\n  >> PPO AGENT BEATS RANDOM BASELINE!")
    elif eval_r > rand_r:
        print("\n  >> PPO agent slightly better than random")
    else:
        print("\n  >> No improvement over random (needs more training or architecture changes)")
    
    print("\nDone.")


if __name__ == "__main__":
    main()
