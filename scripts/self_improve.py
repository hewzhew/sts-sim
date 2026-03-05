"""
Self-Improvement Loop: AlphaZero-style NN ↔ MCTS Cyclic Training

Architecture:
  1. Train NN on MCTS data (ExpertNet style)
  2. Export NN weights to binary file
  3. Load weights into Rust MCTS → NN-guided search
  4. Collect better data with NN-MCTS → train NN again
  5. Repeat — each generation gets better

Usage:
  python scripts/self_improve.py --generations 5 --episodes 15 --n-sims 30
"""

import os, sys, time, struct
import numpy as np

# ── PyTorch ──
import torch
import torch.nn as nn
import torch.optim as optim

# ── STS Sim ──
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import sts_sim

sts_sim.set_verbose(False)

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")
ACTION_DIM = 100
WEIGHTS_PATH = "models/nn_weights.bin"
MAX_EP_STEPS = 500
os.makedirs("models", exist_ok=True)


# ══════════════════════════════════════════════════════════════════
# Smart non-combat action (from train_expert_v2)
# ══════════════════════════════════════════════════════════════════

def pick_noncombat(valid, screen):
    for a in [99, 39, 34, 35, 36]:
        if a in valid: return a
    for a in range(30, 34):
        if a in valid: return a
    for a in range(20, 30):
        if a in valid: return a
    for a in [60, 61, 62, 63, 64, 65]:
        if a in valid: return a
    for a in range(90, 100):
        if a in valid: return a
    return valid[np.random.randint(len(valid))]


# ══════════════════════════════════════════════════════════════════
# Network: shared backbone + policy/value heads
# ══════════════════════════════════════════════════════════════════

class ExpertNetV3(nn.Module):
    """MLP with shared backbone + policy/value heads."""
    
    def __init__(self, obs_dim):
        super().__init__()
        self.shared = nn.Sequential(
            nn.Linear(obs_dim, 256),
            nn.LayerNorm(256),
            nn.ReLU(),
            nn.Linear(256, 256),
            nn.LayerNorm(256),
            nn.ReLU(),
            nn.Linear(256, 128),
            nn.LayerNorm(128),
            nn.ReLU(),
        )
        self.policy_head = nn.Linear(128, ACTION_DIM)
        self.value_head = nn.Linear(128, 1)
    
    def forward(self, x, mask=None):
        h = self.shared(x)
        logits = self.policy_head(h)
        if mask is not None:
            logits = logits.masked_fill(~mask, -1e8)
        return logits, self.value_head(h).squeeze(-1)


# ══════════════════════════════════════════════════════════════════
# Weight Export: PyTorch → binary format for Rust
# ══════════════════════════════════════════════════════════════════

def export_weights(model: ExpertNetV3, path: str):
    """Export model weights to binary file readable by Rust SimpleMLP.
    
    Format:
      u32: num_shared_layers
      For each shared Linear layer: u32 rows, u32 cols, f32[] weights, f32[] bias
      Policy head: same format
      Value head: same format
    
    LayerNorm is approximately fused into the Linear weights.
    """
    
    # Collect (Linear, optional LayerNorm) pairs from shared
    shared_pairs = []
    modules = list(model.shared.children())
    i = 0
    while i < len(modules):
        if isinstance(modules[i], nn.Linear):
            linear = modules[i]
            ln = modules[i+1] if i+1 < len(modules) and isinstance(modules[i+1], nn.LayerNorm) else None
            shared_pairs.append((linear, ln))
            i += 2 if ln else 1
            if i < len(modules) and isinstance(modules[i], nn.ReLU):
                i += 1
        else:
            i += 1
    
    def get_weights(linear, ln=None):
        W = linear.weight.data.cpu().numpy()  # [out, in]
        b = linear.bias.data.cpu().numpy()    # [out]
        if ln is not None:
            gamma = ln.weight.data.cpu().numpy()
            beta = ln.bias.data.cpu().numpy()
            W = W * gamma[:, None]
            b = b * gamma + beta
        return W, b
    
    with open(path, 'wb') as f:
        f.write(struct.pack('<I', len(shared_pairs)))
        for linear, ln in shared_pairs:
            W, b = get_weights(linear, ln)
            rows, cols = W.shape
            f.write(struct.pack('<II', rows, cols))
            f.write(W.astype(np.float32).tobytes())
            f.write(b.astype(np.float32).tobytes())
        
        # Policy head
        W = model.policy_head.weight.data.cpu().numpy()
        b = model.policy_head.bias.data.cpu().numpy()
        f.write(struct.pack('<II', *W.shape))
        f.write(W.astype(np.float32).tobytes())
        f.write(b.astype(np.float32).tobytes())
        
        # Value head
        W = model.value_head.weight.data.cpu().numpy()
        b = model.value_head.bias.data.cpu().numpy()
        f.write(struct.pack('<II', *W.shape))
        f.write(W.astype(np.float32).tobytes())
        f.write(b.astype(np.float32).tobytes())
    
    print(f"    Exported: {path} ({os.path.getsize(path):,} bytes)")


# ══════════════════════════════════════════════════════════════════
# Data Collection
# ══════════════════════════════════════════════════════════════════

def collect_mcts_data(n_episodes, n_sims=30, seed_offset=0, nn_guided=False):
    """Collect training data using MCTS to find best actions."""
    data = []
    total_reward = 0.0
    
    for ep in range(n_episodes):
        seed = seed_offset + ep * 7 + 1
        env = sts_sim.PyStsSim(seed=seed)
        env.reset(seed=seed)
        
        if nn_guided and os.path.exists(WEIGHTS_PATH):
            env.load_nn_weights(WEIGHTS_PATH)
        
        ep_reward = 0.0
        ep_obs_actions = []
        steps = 0
        
        while steps < MAX_EP_STEPS:
            screen = env.get_screen_type()
            if screen == "GAME_OVER":
                break
            
            mask_list = env.get_valid_actions_mask()
            valid = [i for i, v in enumerate(mask_list) if v]
            
            if not valid:
                done, r = env.step(99)
                ep_reward += r; steps += 1
                if done: break
                continue
            
            if screen == "COMBAT":
                obs = np.array(env.get_observation(), dtype=np.float32)
                mask_arr = np.array(mask_list, dtype=np.float32)
                
                result = env.mcts_evaluate(n_sims=n_sims)
                best = result['best_action']
                
                ep_obs_actions.append((obs, best, mask_arr))
                
                done, r = env.step(best)
                ep_reward += r; steps += 1
                if done: break
            else:
                action = pick_noncombat(valid, screen)
                done, r = env.step(action)
                ep_reward += r; steps += 1
                if done: break
        
        # Assign value targets
        n_steps = len(ep_obs_actions)
        for i, (obs, action, mask) in enumerate(ep_obs_actions):
            discount = 0.99 ** (n_steps - i)
            value_target = ep_reward * discount
            data.append((obs, action, mask, value_target))
        
        total_reward += ep_reward
    
    avg_r = total_reward / max(n_episodes, 1)
    return data, avg_r


# ══════════════════════════════════════════════════════════════════
# Training
# ══════════════════════════════════════════════════════════════════

def train_model(model, data, epochs=10, lr=1e-3):
    if not data:
        return 0.0, 0.0
    
    optimizer = optim.Adam(model.parameters(), lr=lr)
    
    obs_batch = torch.tensor(np.array([d[0] for d in data]), dtype=torch.float32).to(DEVICE)
    act_batch = torch.tensor([d[1] for d in data], dtype=torch.long).to(DEVICE)
    mask_batch = torch.tensor(np.array([d[2] for d in data]), dtype=torch.float32).to(DEVICE)
    val_batch = torch.tensor([d[3] for d in data], dtype=torch.float32).to(DEVICE)
    
    mask_bool = mask_batch.bool()
    val_mean = val_batch.mean()
    val_std = val_batch.std() + 1e-8
    val_norm = (val_batch - val_mean) / val_std
    
    total_loss = 0.0
    total = len(data)
    n_batches = 0
    
    for epoch in range(epochs):
        perm = torch.randperm(total, device=DEVICE)
        batch_size = min(256, total)
        
        for start in range(0, total, batch_size):
            end = min(start + batch_size, total)
            idx = perm[start:end]
            
            policy_logits, values = model(obs_batch[idx], mask=mask_bool[idx])
            
            policy_loss = nn.functional.cross_entropy(policy_logits, act_batch[idx])
            value_loss = nn.functional.mse_loss(values, val_norm[idx])
            loss = policy_loss + 0.5 * value_loss
            
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            
            total_loss += loss.item()
            n_batches += 1
    
    with torch.no_grad():
        policy_logits, _ = model(obs_batch, mask=mask_bool)
        preds = policy_logits.argmax(dim=1)
        correct = (preds == act_batch).sum().item()
    
    acc = correct / max(total, 1)
    avg_loss = total_loss / max(n_batches, 1)
    return avg_loss, acc


# ══════════════════════════════════════════════════════════════════
# Evaluation
# ══════════════════════════════════════════════════════════════════

def eval_with_mcts(n_episodes=10, seed_base=9000, nn_guided=False, n_sims=30):
    """Evaluate using MCTS for combat decisions."""
    rewards = []
    for ep in range(n_episodes):
        seed = seed_base + ep
        env = sts_sim.PyStsSim(seed=seed)
        env.reset(seed=seed)
        
        if nn_guided and os.path.exists(WEIGHTS_PATH):
            env.load_nn_weights(WEIGHTS_PATH)
        
        total_r = 0.0; steps = 0
        while steps < MAX_EP_STEPS:
            screen = env.get_screen_type()
            if screen == "GAME_OVER": break
            
            mask = env.get_valid_actions_mask()
            valid = [j for j, v in enumerate(mask) if v]
            if not valid:
                done, r = env.step(99); total_r += r; steps += 1
                if done: break
                continue
            
            if screen == "COMBAT":
                mcts = env.mcts_evaluate(n_sims=n_sims)
                done, r = env.step(mcts["best_action"])
            else:
                action = pick_noncombat(valid, screen)
                done, r = env.step(action)
            total_r += r; steps += 1
            if done: break
        rewards.append(total_r)
    return np.mean(rewards), np.std(rewards)


# ══════════════════════════════════════════════════════════════════
# Self-Improvement Loop
# ══════════════════════════════════════════════════════════════════

def self_improvement_loop(
    generations=5,
    episodes_per_gen=15,
    n_sims=30,
    train_epochs=10,
    lr=1e-3,
):
    print("=" * 60)
    print("  Self-Improvement Loop (AlphaZero-style)")
    print(f"  Generations: {generations} | Episodes/gen: {episodes_per_gen}")
    print(f"  MCTS sims: {n_sims} | Train epochs: {train_epochs}")
    print(f"  Device: {DEVICE}")
    print("=" * 60)
    
    # Detect obs_dim
    test_env = sts_sim.PyStsSim(1)
    test_env.reset()
    obs_dim = len(test_env.get_observation())
    del test_env
    print(f"  Obs dim: {obs_dim}")
    
    model = ExpertNetV3(obs_dim).to(DEVICE)
    params = sum(p.numel() for p in model.parameters())
    print(f"  Model: {params:,} params")
    
    # Baseline: random MCTS
    print("\n  [Baseline] Random MCTS...")
    rand_r, rand_s = eval_with_mcts(10, nn_guided=False, n_sims=n_sims)
    print(f"  Random-MCTS: {rand_r:.1f} ± {rand_s:.1f}")
    print()
    
    best_reward = -999.0
    all_data = []
    MAX_BUFFER = 30000
    
    for gen in range(1, generations + 1):
        t0 = time.time()
        nn_guided = gen > 1
        
        # ── Collect ──
        seed_offset = gen * 1000
        data, avg_reward = collect_mcts_data(
            episodes_per_gen, n_sims=n_sims, 
            seed_offset=seed_offset, nn_guided=nn_guided
        )
        
        all_data.extend(data)
        if len(all_data) > MAX_BUFFER:
            all_data = all_data[-MAX_BUFFER:]
        
        # ── Train ──
        loss, acc = train_model(model, all_data, epochs=train_epochs, lr=lr)
        
        # ── Export ──
        export_weights(model, WEIGHTS_PATH)
        
        # ── Track ──
        is_best = avg_reward > best_reward
        if is_best:
            best_reward = avg_reward
            torch.save(model.state_dict(), "models/best_model.pt")
        
        dt = time.time() - t0
        mode = "NN-MCTS" if nn_guided else "Rand-MCTS"
        star = " ★" if is_best else ""
        print(f"  Gen {gen}/{generations} | {mode:9s} | "
              f"R: {avg_reward:+6.1f} | Loss: {loss:.3f} | Acc: {acc:.0%} | "
              f"Buf: {len(all_data):,} | {dt:.1f}s{star}")
    
    # ── Final comparison ──
    print(f"\n{'='*60}")
    print("  Final: NN-MCTS vs Random-MCTS (10 games each)...")
    nn_r, nn_s = eval_with_mcts(10, nn_guided=True, n_sims=n_sims)
    rand_r2, rand_s2 = eval_with_mcts(10, nn_guided=False, n_sims=n_sims)
    print(f"  NN-MCTS:     {nn_r:.1f} ± {nn_s:.1f}")
    print(f"  Random-MCTS: {rand_r2:.1f} ± {rand_s2:.1f}")
    print(f"  Δ:           {nn_r - rand_r2:+.1f}")
    print(f"  Weights:     {WEIGHTS_PATH}")
    print("=" * 60)
    print("Done.")


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--generations", type=int, default=5)
    parser.add_argument("--episodes", type=int, default=15)
    parser.add_argument("--n-sims", type=int, default=30)
    parser.add_argument("--train-epochs", type=int, default=10)
    parser.add_argument("--lr", type=float, default=1e-3)
    args = parser.parse_args()
    
    self_improvement_loop(
        generations=args.generations,
        episodes_per_gen=args.episodes,
        n_sims=args.n_sims,
        train_epochs=args.train_epochs,
        lr=args.lr,
    )
