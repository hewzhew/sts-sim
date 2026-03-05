"""
Expert Iteration v2: Reward Shaping + Experience Replay + Multi-Head

Improvements over v1:
  - Experience replay buffer (accumulates MCTS data across iterations)
  - Reward shaping (damage, block, kill, power sub-signals from game state)
  - Additional auxiliary heads (enemy_killed, block_gained predictions)
  - Longer training with better data efficiency
"""
import time, collections, numpy as np
import torch, torch.nn as nn, torch.nn.functional as F
import sts_sim

sts_sim.set_verbose(False)
MAX_EP_STEPS = 500

# =============================================================================
# Smart non-combat action
# =============================================================================
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

# =============================================================================
# Network v2: Policy + Value + HP + Kill + Block
# =============================================================================
class ExpertNetV2(nn.Module):
    def __init__(self, obs_dim, act_dim, hidden=256):
        super().__init__()
        self.shared = nn.Sequential(
            nn.Linear(obs_dim, hidden), nn.LayerNorm(hidden), nn.ReLU(),
            nn.Linear(hidden, hidden), nn.LayerNorm(hidden), nn.ReLU(),
            nn.Linear(hidden, hidden // 2), nn.ReLU(),
        )
        self.policy = nn.Linear(hidden // 2, act_dim)
        self.value = nn.Linear(hidden // 2, 1)
        self.hp_delta = nn.Linear(hidden // 2, 1)     # predict HP change
        self.kill_pred = nn.Linear(hidden // 2, 1)     # predict P(enemy dies this action)
        self.block_pred = nn.Linear(hidden // 2, 1)    # predict block gained
        self._init()

    def _init(self):
        for m in self.modules():
            if isinstance(m, nn.Linear):
                nn.init.orthogonal_(m.weight, gain=0.5)
                nn.init.zeros_(m.bias)

    def forward(self, x, mask=None):
        f = self.shared(x)
        logits = self.policy(f)
        if mask is not None:
            logits = logits.masked_fill(~mask, -1e8)
        return (logits,
                self.value(f).squeeze(-1),
                self.hp_delta(f).squeeze(-1),
                self.kill_pred(f).squeeze(-1),
                self.block_pred(f).squeeze(-1))

# =============================================================================
# Experience Replay Buffer
# =============================================================================
class ReplayBuffer:
    def __init__(self, max_size=50000):
        self.obs = collections.deque(maxlen=max_size)
        self.actions = collections.deque(maxlen=max_size)
        self.masks = collections.deque(maxlen=max_size)
        self.hp_deltas = collections.deque(maxlen=max_size)
        self.kills = collections.deque(maxlen=max_size)
        self.blocks = collections.deque(maxlen=max_size)
        self.rewards = collections.deque(maxlen=max_size)

    def add(self, obs, action, mask, hp_delta, killed, block, reward):
        self.obs.append(obs)
        self.actions.append(action)
        self.masks.append(mask)
        self.hp_deltas.append(hp_delta)
        self.kills.append(killed)
        self.blocks.append(block)
        self.rewards.append(reward)

    def __len__(self):
        return len(self.obs)

    def sample(self, batch_size):
        n = len(self)
        if n <= batch_size:
            idx = list(range(n))
        else:
            idx = np.random.choice(n, batch_size, replace=False)
        return {
            'obs': torch.tensor(np.array([self.obs[i] for i in idx]), dtype=torch.float32),
            'actions': torch.tensor([self.actions[i] for i in idx], dtype=torch.long),
            'masks': torch.tensor(np.array([self.masks[i] for i in idx]), dtype=torch.bool),
            'hp_deltas': torch.tensor([self.hp_deltas[i] for i in idx], dtype=torch.float32),
            'kills': torch.tensor([self.kills[i] for i in idx], dtype=torch.float32),
            'blocks': torch.tensor([self.blocks[i] for i in idx], dtype=torch.float32),
            'rewards': torch.tensor([self.rewards[i] for i in idx], dtype=torch.float32),
        }

# =============================================================================
# Collect MCTS data with richer signals
# =============================================================================
def collect_data(buffer, n_eps, n_sims=20, seed_base=0):
    total_r = 0; ep_rewards = []
    for ep in range(n_eps):
        seed = seed_base + ep
        env = sts_sim.PyStsSim(seed=seed)
        env.reset(seed=seed)
        ep_r = 0; steps = 0

        while steps < MAX_EP_STEPS:
            screen = env.get_screen_type()
            if screen == 'GAME_OVER': break

            mask_list = env.get_valid_actions_mask()
            valid = [j for j, v in enumerate(mask_list) if v]
            if not valid:
                done, r = env.step(99); ep_r += r; steps += 1
                if done: break
                continue

            if screen == 'COMBAT':
                obs = np.array(env.get_observation(), dtype=np.float32)
                mask_arr = np.array(mask_list, dtype=bool)
                hp_before = env.get_hp()
                block_before = env.get_observation()[4] if len(env.get_observation()) > 4 else 0

                # Get enemy HP before
                obs_dict = None
                try:
                    obs_dict = env.get_observation_dict()
                    enemy_hp_before = sum(obs_dict.get('enemy_hp', [0]))
                except:
                    enemy_hp_before = 100

                result = env.mcts_evaluate(n_sims=n_sims, max_turns=10)
                best_action = result['best_action']

                done, r = env.step(best_action)
                ep_r += r; steps += 1

                hp_after = env.get_hp()
                hp_delta = hp_after - hp_before

                # Check if any enemy died
                try:
                    obs_dict2 = env.get_observation_dict()
                    enemy_hp_after = sum(obs_dict2.get('enemy_hp', [0]))
                    killed = 1.0 if (enemy_hp_after < enemy_hp_before and
                                     any(h <= 0 for h in obs_dict2.get('enemy_hp', [1]))) else 0.0
                    block_after = obs_dict2.get('block', 0)
                except:
                    killed = 0.0
                    block_after = 0

                # Shaped reward: base reward + sub-signals
                shaped_r = r
                if killed > 0: shaped_r += 1.0
                if hp_delta < 0: shaped_r += hp_delta * 0.1  # penalty for losing HP

                buffer.add(obs, best_action, mask_arr, hp_delta, killed,
                           float(block_after), shaped_r)

                if done: break
            else:
                action = pick_noncombat(valid, screen)
                done, r = env.step(action); ep_r += r; steps += 1
                if done: break

        total_r += ep_r; ep_rewards.append(ep_r)
    return total_r / max(n_eps, 1), ep_rewards

# =============================================================================
# Training
# =============================================================================
def train_batch(model, optimizer, data):
    logits, values, hp_preds, kill_preds, block_preds = model(data['obs'], mask=data['masks'])

    # Policy: imitate MCTS
    policy_loss = F.cross_entropy(logits, data['actions'])
    # Value: predict shaped reward
    value_loss = F.mse_loss(values, data['rewards'] / 50.0)
    # HP delta
    hp_loss = F.mse_loss(hp_preds, data['hp_deltas'] / 10.0)
    # Kill prediction
    kill_loss = F.binary_cross_entropy_with_logits(kill_preds, data['kills'])
    # Block prediction
    block_loss = F.mse_loss(block_preds, data['blocks'] / 20.0)

    loss = policy_loss + 0.3*value_loss + 0.1*hp_loss + 0.2*kill_loss + 0.1*block_loss

    optimizer.zero_grad(); loss.backward()
    nn.utils.clip_grad_norm_(model.parameters(), 1.0)
    optimizer.step()

    with torch.no_grad():
        acc = (logits.argmax(-1) == data['actions']).float().mean().item()
    return {'loss': loss.item(), 'policy': policy_loss.item(), 'acc': acc,
            'kill_loss': kill_loss.item()}

# =============================================================================
# Evaluation
# =============================================================================
def eval_agent(model, n_eps=20, seed_base=9000):
    rewards = []
    for ep in range(n_eps):
        env = sts_sim.PyStsSim(seed=seed_base + ep)
        env.reset(seed=seed_base + ep)
        total_r = 0; steps = 0
        while steps < MAX_EP_STEPS:
            screen = env.get_screen_type()
            if screen == 'GAME_OVER': break
            mask = env.get_valid_actions_mask()
            valid = [j for j, v in enumerate(mask) if v]
            if not valid:
                done, r = env.step(99); total_r += r; steps += 1
                if done: break
                continue
            if screen == 'COMBAT' and model is not None:
                obs = torch.tensor(env.get_observation(), dtype=torch.float32)
                mask_t = torch.tensor(mask, dtype=torch.bool)
                with torch.no_grad():
                    logits = model(obs.unsqueeze(0), mask=mask_t.unsqueeze(0))[0]
                    action = logits.squeeze(0).argmax().item()
            elif screen == 'COMBAT':
                action = valid[np.random.randint(len(valid))]
            else:
                action = pick_noncombat(valid, screen)
            done, r = env.step(action); total_r += r; steps += 1
            if done: break
        rewards.append(total_r)
    return np.mean(rewards), np.std(rewards)

# =============================================================================
# Main
# =============================================================================
def main():
    print("=" * 60)
    print("Expert Iteration v2 — Replay Buffer + Reward Shaping")
    print("=" * 60)

    env = sts_sim.PyStsSim(seed=0); env.reset()
    obs_dim = len(env.get_observation())
    act_dim = len(env.get_valid_actions_mask())

    model = ExpertNetV2(obs_dim, act_dim, hidden=256)
    optimizer = torch.optim.Adam(model.parameters(), lr=5e-4)
    params = sum(p.numel() for p in model.parameters())
    print(f"  Network: {obs_dim} → 256 → 256 → 128 → heads ({params:,} params)")
    print(f"  Heads: policy, value, hp_delta, kill, block")

    buffer = ReplayBuffer(max_size=50000)

    N_ITERS = 20
    EPS_PER_ITER = 5
    BATCH_SIZE = 1024
    TRAIN_BATCHES = 4
    print(f"  Config: {N_ITERS} iters × {EPS_PER_ITER} ep, replay up to 50K, batch {BATCH_SIZE}")

    print("\n[Baseline]")
    rand_r, rand_s = eval_agent(None, 20)
    print(f"  Random: {rand_r:.1f} ± {rand_s:.1f}")

    print(f"\n{'='*60}")
    best = -1e9; t0 = time.time()
    seed_offset = int(time.time()) % 100000

    for it in range(N_ITERS):
        ti = time.time()

        # Collect new data
        mcts_r, _ = collect_data(buffer, EPS_PER_ITER, n_sims=20,
                                  seed_base=seed_offset + it * EPS_PER_ITER)

        # Train on replay buffer
        losses = []
        for _ in range(TRAIN_BATCHES):
            batch = buffer.sample(min(BATCH_SIZE, len(buffer)))
            info = train_batch(model, optimizer, batch)
            losses.append(info)

        loss = np.mean([l['loss'] for l in losses])
        acc = np.mean([l['acc'] for l in losses])
        dt = time.time() - ti

        print(f"  Iter {it+1:2d}/{N_ITERS} | "
              f"MCTS_r: {mcts_r:+6.0f} | "
              f"Loss: {loss:.3f} | Acc: {acc:.0%} | "
              f"Buf: {len(buffer):5d} | {dt:.1f}s")

        # Evaluate every 5 iters
        if (it + 1) % 5 == 0 or it == N_ITERS - 1:
            nn_r, nn_s = eval_agent(model, 15)
            imp = nn_r - rand_r
            tag = " ★ NEW BEST" if nn_r > best else ""
            if nn_r > best:
                best = nn_r
                torch.save(model.state_dict(), 'scripts/best_expert_v2.pt')
            print(f"  [EVAL] NN: {nn_r:.1f}±{nn_s:.1f} | vs Random: {imp:+.1f}{tag}")

    print(f"\n{'='*60}")
    nn_r, nn_s = eval_agent(model, 30)
    print(f"  NN:     {nn_r:.1f} ± {nn_s:.1f}")
    print(f"  Random: {rand_r:.1f} ± {rand_s:.1f}")
    print(f"  Δ:      {nn_r - rand_r:+.1f}")
    print(f"  Best:   {best:.1f}")
    print(f"  Time:   {time.time()-t0:.0f}s | Buffer: {len(buffer):,}")
    if nn_r > rand_r: print(f"  >> EXPERT V2 BEATS RANDOM!")
    print("Done.")

if __name__ == '__main__':
    main()
