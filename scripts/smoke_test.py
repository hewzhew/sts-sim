#!/usr/bin/env python3
"""
PPO Smoke Test v2 — Using proper action masking
================================================
Tests whether the sts_sim Rust env produces a learning signal
by using get_valid_actions_mask() for proper action selection.
"""

import time
import numpy as np

try:
    import sts_sim
    sts_sim.set_verbose(False)  # Suppress Rust game logs for training speed
except ImportError as e:
    print(f"Cannot import sts_sim: {e}")
    print("   Run: maturin develop --release")
    exit(1)


# ==============================================================================
# Minimal Linear Policy (numpy only)
# ==============================================================================

class LinearPolicy:
    """Simple linear policy: softmax(W @ obs + b) with action masking."""
    
    def __init__(self, obs_dim: int, act_dim: int):
        scale = np.sqrt(2.0 / (obs_dim + act_dim))
        self.W = np.random.randn(act_dim, obs_dim).astype(np.float32) * scale
        self.b = np.zeros(act_dim, dtype=np.float32)
        self.Wv = np.random.randn(obs_dim).astype(np.float32) * 0.01
        self.bv = np.float32(0.0)
    
    def forward(self, obs: np.ndarray, mask: np.ndarray):
        """Returns (action, log_prob, value)."""
        logits = self.W @ obs + self.b
        logits[~mask] = -1e9
        logits -= logits.max()
        probs = np.exp(logits)
        probs /= probs.sum() + 1e-8
        action = np.random.choice(len(probs), p=probs)
        log_prob = np.log(probs[action] + 1e-8)
        value = float(self.Wv @ obs + self.bv)
        return action, log_prob, value
    
    def update(self, trajectories, lr=1e-3):
        """REINFORCE with baseline update."""
        if not trajectories:
            return 0.0
        
        total_loss = 0.0
        n_steps = 0
        for traj in trajectories:
            obs_list, actions, log_probs, rewards, values = traj
            T = len(rewards)
            if T == 0:
                continue
            
            gamma = 0.99
            returns = np.zeros(T, dtype=np.float32)
            G = 0.0
            for t in reversed(range(T)):
                G = rewards[t] + gamma * G
                returns[t] = G
            
            if returns.std() > 1e-6:
                returns = (returns - returns.mean()) / (returns.std() + 1e-8)
            
            for t in range(T):
                advantage = returns[t] - values[t]
                obs = obs_list[t]
                action = actions[t]
                
                logits = self.W @ obs + self.b
                logits -= logits.max()
                probs = np.exp(logits)
                probs /= probs.sum() + 1e-8
                
                grad_logits = -probs.copy()
                grad_logits[action] += 1.0
                
                self.W += lr * advantage * np.outer(grad_logits, obs)
                self.b += lr * advantage * grad_logits
                
                v_pred = float(self.Wv @ obs + self.bv)
                v_error = returns[t] - v_pred
                self.Wv += lr * 0.5 * v_error * obs
                self.bv += lr * 0.5 * v_error
                
                total_loss += advantage ** 2
                n_steps += 1
        
        return total_loss / max(n_steps, 1)


# ==============================================================================
# Episode runner using PyStsSim with proper action masking
# ==============================================================================

def run_episode(env, policy, max_steps=500):
    """Run one episode using PyStsSim with get_valid_actions_mask()."""
    obs_list, actions, log_probs, rewards, values = [], [], [], [], []
    
    for step in range(max_steps):
        screen = env.get_screen_type()
        if screen == "GAME_OVER":
            break
        
        obs = np.array(env.get_observation(), dtype=np.float32)
        mask = np.array(env.get_valid_actions_mask(), dtype=bool)
        
        # Check if any action is valid
        if not mask.any():
            # No valid actions — try proceed
            done, reward = env.step(99)
            obs_list.append(obs)
            actions.append(99)
            log_probs.append(0.0)
            rewards.append(reward)
            values.append(0.0)
            if done:
                break
            continue
        
        action, log_prob, value = policy.forward(obs, mask)
        
        obs_list.append(obs)
        actions.append(action)
        log_probs.append(log_prob)
        values.append(value)
        
        done, reward = env.step(int(action))
        rewards.append(reward)
        
        if done:
            break
    
    total_reward = sum(rewards)
    hp = env.get_hp()
    return (obs_list, actions, log_probs, rewards, values), total_reward, hp


# ==============================================================================
# Random baseline
# ==============================================================================

def run_random_episode(env, max_steps=500):
    """Run one episode with random valid actions (baseline)."""
    total_reward = 0.0
    for step in range(max_steps):
        screen = env.get_screen_type()
        if screen == "GAME_OVER":
            break
        
        mask = np.array(env.get_valid_actions_mask(), dtype=bool)
        valid_actions = np.where(mask)[0]
        
        if len(valid_actions) == 0:
            done, reward = env.step(99)
        else:
            action = np.random.choice(valid_actions)
            done, reward = env.step(int(action))
        
        total_reward += reward
        if done:
            break
    
    return total_reward, env.get_hp()


# ==============================================================================
# Main
# ==============================================================================

def main():
    print("=" * 60)
    print("STS Simulator -- RL Smoke Test v2")
    print("=" * 60)
    
    # Phase 1: Basic interface check
    print("\n[Phase 1] Interface check...")
    env = sts_sim.PyStsSim(seed=42)
    env.reset()
    print(f"  Screen: {env.get_screen_type()}")
    print(f"  HP: {env.get_hp()}/{env.get_max_hp()}")
    obs = env.get_observation()
    print(f"  Obs dim: {len(obs)}")
    mask = env.get_valid_actions_mask()
    print(f"  Mask dim: {len(mask)}")
    valid_count = sum(1 for x in mask if x)
    print(f"  Valid actions: {valid_count}")
    print("  OK")
    
    obs_dim = len(obs)
    act_dim = len(mask)
    
    # Phase 2: Random baseline
    print(f"\n[Phase 2] Random baseline (50 episodes)...")
    random_rewards = []
    random_hps = []
    t0 = time.time()
    for i in range(50):
        env = sts_sim.PyStsSim(seed=i)
        env.reset(seed=i)
        r, hp = run_random_episode(env)
        random_rewards.append(r)
        random_hps.append(hp)
    t_rand = time.time() - t0
    print(f"  Avg reward: {np.mean(random_rewards):+.2f} +/- {np.std(random_rewards):.2f}")
    print(f"  Avg final HP: {np.mean(random_hps):.1f}")
    print(f"  Speed: {50 / t_rand:.1f} ep/s")
    
    # Phase 3: Training
    NUM_EPISODES = 400
    BATCH_SIZE = 8
    NUM_EPOCHS = NUM_EPISODES // BATCH_SIZE
    
    print(f"\n[Phase 3] Training ({NUM_EPISODES} episodes, batch={BATCH_SIZE})...")
    policy = LinearPolicy(obs_dim, act_dim)
    
    win_rates = []
    avg_rewards = []
    avg_hps = []
    
    t0 = time.time()
    
    for epoch in range(NUM_EPOCHS):
        trajectories = []
        ep_rewards = []
        ep_hps = []
        
        for ep in range(BATCH_SIZE):
            seed = epoch * BATCH_SIZE + ep + 1000  # Offset from random baseline seeds
            env = sts_sim.PyStsSim(seed=seed)
            env.reset(seed=seed)
            
            traj, total_r, hp = run_episode(env, policy)
            trajectories.append(traj)
            ep_rewards.append(total_r)
            ep_hps.append(hp)
        
        # Update policy
        loss = policy.update(trajectories, lr=1e-3)
        
        avg_r = np.mean(ep_rewards)
        avg_hp = np.mean(ep_hps)
        avg_rewards.append(avg_r)
        avg_hps.append(avg_hp)
        
        elapsed = time.time() - t0
        eps_per_sec = (epoch + 1) * BATCH_SIZE / elapsed
        
        if epoch % 10 == 0 or epoch == NUM_EPOCHS - 1:
            print(f"  Epoch {epoch:3d}/{NUM_EPOCHS} | Avg R: {avg_r:+.2f} | "
                  f"Avg HP: {avg_hp:.1f} | Loss: {loss:.4f} | {eps_per_sec:.1f} ep/s")
    
    elapsed = time.time() - t0
    
    # Phase 4: Analysis
    print(f"\n[Phase 4] Analysis")
    print(f"  Total time: {elapsed:.1f}s ({NUM_EPISODES / elapsed:.1f} ep/s)")
    
    # Compare first quarter vs last quarter
    q = max(1, len(avg_rewards) // 4)
    first_r = np.mean(avg_rewards[:q])
    last_r = np.mean(avg_rewards[-q:])
    first_hp = np.mean(avg_hps[:q])
    last_hp = np.mean(avg_hps[-q:])
    rand_r = np.mean(random_rewards)
    
    print(f"\n  Random baseline  -- Avg R: {rand_r:+.2f}")
    print(f"  Trained (first)  -- Avg R: {first_r:+.2f}, HP: {first_hp:.1f}")
    print(f"  Trained (last)   -- Avg R: {last_r:+.2f}, HP: {last_hp:.1f}")
    print(f"  Delta reward:      {last_r - first_r:+.2f}")
    print(f"  Delta HP:          {last_hp - first_hp:+.1f}")
    print(f"  vs Random:         {last_r - rand_r:+.2f}")
    
    if last_r > first_r + 0.5:
        print("\n  >> LEARNING SIGNAL DETECTED (reward improving)")
    elif last_hp > first_hp + 2:
        print("\n  >> LEARNING SIGNAL DETECTED (HP improving)")
    elif last_r > rand_r + 0.5:
        print("\n  >> BEATS RANDOM BASELINE")
    else:
        print("\n  >> No clear learning signal yet")
        print("     (may need more episodes, deeper network, or framework fixes)")
    
    print("\nDone.")


if __name__ == "__main__":
    main()
