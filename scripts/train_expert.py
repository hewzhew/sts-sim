"""
Expert Iteration Training for Slay the Spire

MCTS (teacher) → searches for best combat action
NN   (student) → learns to predict MCTS's decisions + predict HP changes

Much more sample-efficient than PPO: dense labels every step, not sparse rewards.
"""
import time
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
import sts_sim

sts_sim.set_verbose(False)

MAX_STEPS_PER_EPISODE = 500  # Hard cap to prevent infinite loops

# =============================================================================
# Smart non-combat action selection
# =============================================================================
def pick_noncombat_action(valid_actions, screen):
    """
    Heuristic for non-combat screens. Prefers:
    - action 99 (proceed/skip) to advance the game
    - action 39 (skip rewards) for reward screen
    - otherwise random from valid
    """
    # Prefer skip/proceed actions to advance quickly
    for priority in [99, 39, 34, 35, 36]:
        if priority in valid_actions:
            return priority
    # For reward screen: take first card if available (30-33), else skip
    for a in [30, 31, 32, 33]:
        if a in valid_actions:
            return a
    # For map: pick first valid node
    for a in range(20, 30):
        if a in valid_actions:
            return a
    # For campfire: rest (60) > smith (61)
    for a in [60, 61, 62, 63, 64, 65]:
        if a in valid_actions:
            return a
    # For events: first choice
    for a in range(90, 100):
        if a in valid_actions:
            return a
    # Fallback: random from valid
    return valid_actions[np.random.randint(len(valid_actions))]


# =============================================================================
# Network
# =============================================================================
class ExpertNet(nn.Module):
    def __init__(self, obs_dim, act_dim, hidden=256):
        super().__init__()
        self.shared = nn.Sequential(
            nn.Linear(obs_dim, hidden), nn.ReLU(),
            nn.Linear(hidden, hidden // 2), nn.ReLU(),
        )
        self.policy = nn.Linear(hidden // 2, act_dim)
        self.value = nn.Linear(hidden // 2, 1)
        self.hp_delta = nn.Linear(hidden // 2, 1)
        for m in self.modules():
            if isinstance(m, nn.Linear):
                nn.init.orthogonal_(m.weight, gain=0.5)
                nn.init.zeros_(m.bias)

    def forward(self, x, mask=None):
        f = self.shared(x)
        logits = self.policy(f)
        if mask is not None:
            logits = logits.masked_fill(~mask, -1e8)
        return logits, self.value(f).squeeze(-1), self.hp_delta(f).squeeze(-1)


# =============================================================================
# Data collection with MCTS
# =============================================================================
def collect_expert_data(n_episodes, n_sims=20, max_turns=10, seed_offset=0):
    all_obs, all_actions, all_masks, all_hp_deltas = [], [], [], []
    total_reward = 0; all_rewards = []

    for ep in range(n_episodes):
        seed = seed_offset + ep
        env = sts_sim.PyStsSim(seed=seed)
        env.reset(seed=seed)
        ep_reward = 0; steps = 0

        while steps < MAX_STEPS_PER_EPISODE:
            screen = env.get_screen_type()
            if screen == 'GAME_OVER':
                break

            mask = env.get_valid_actions_mask()
            valid = [j for j, v in enumerate(mask) if v]
            if not valid:
                done, r = env.step(99)
                ep_reward += r; steps += 1
                if done: break
                continue

            if screen == 'COMBAT':
                obs = np.array(env.get_observation(), dtype=np.float32)
                mask_arr = np.array(mask, dtype=bool)
                hp_before = env.get_hp()

                result = env.mcts_evaluate(n_sims=n_sims, max_turns=max_turns)
                best_action = result['best_action']

                all_obs.append(obs)
                all_actions.append(best_action)
                all_masks.append(mask_arr)

                done, r = env.step(best_action)
                ep_reward += r; steps += 1
                all_hp_deltas.append(env.get_hp() - hp_before)
                if done: break
            else:
                action = pick_noncombat_action(valid, screen)
                done, r = env.step(action)
                ep_reward += r; steps += 1
                if done: break

        total_reward += ep_reward
        all_rewards.append(ep_reward)

    if not all_obs:
        return None

    return {
        'obs': torch.tensor(np.array(all_obs)),
        'actions': torch.tensor(all_actions, dtype=torch.long),
        'masks': torch.tensor(np.array(all_masks)),
        'hp_deltas': torch.tensor(np.array(all_hp_deltas, dtype=np.float32)),
        'mean_reward': total_reward / max(n_episodes, 1),
        'n_steps': len(all_obs),
    }


# =============================================================================
# Training step
# =============================================================================
def train_step(model, optimizer, data):
    logits, values, hp_preds = model(data['obs'], mask=data['masks'])
    policy_loss = F.cross_entropy(logits, data['actions'])
    value_loss = F.mse_loss(values, torch.full_like(values, data['mean_reward'] / 100.0))
    hp_loss = F.mse_loss(hp_preds, data['hp_deltas'] / 10.0)
    loss = policy_loss + 0.5 * value_loss + 0.1 * hp_loss

    optimizer.zero_grad(); loss.backward()
    nn.utils.clip_grad_norm_(model.parameters(), 1.0)
    optimizer.step()

    with torch.no_grad():
        acc = (logits.argmax(-1) == data['actions']).float().mean().item()
    return {'total': loss.item(), 'policy': policy_loss.item(), 'accuracy': acc}


# =============================================================================
# Evaluation
# =============================================================================
def run_agent(n_eps, model=None, seed_base=9000):
    """Run episodes. model=None → random agent."""
    rewards = []
    for ep in range(n_eps):
        env = sts_sim.PyStsSim(seed=seed_base + ep)
        env.reset(seed=seed_base + ep)
        total_r = 0; steps = 0

        while steps < MAX_STEPS_PER_EPISODE:
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
                    logits, _, _ = model(obs.unsqueeze(0), mask=mask_t.unsqueeze(0))
                    action = logits.squeeze(0).argmax().item()
            elif screen == 'COMBAT':
                action = valid[np.random.randint(len(valid))]
            else:
                action = pick_noncombat_action(valid, screen)

            done, r = env.step(action); total_r += r; steps += 1
            if done: break

        rewards.append(total_r)
    return np.mean(rewards), np.std(rewards)


# =============================================================================
# Main
# =============================================================================
def main():
    print("=" * 60)
    print("STS — Expert Iteration Training")
    print("=" * 60)

    env = sts_sim.PyStsSim(seed=0); env.reset()
    obs_dim = len(env.get_observation())
    act_dim = len(env.get_valid_actions_mask())

    model = ExpertNet(obs_dim, act_dim)
    optimizer = torch.optim.Adam(model.parameters(), lr=1e-3)
    params = sum(p.numel() for p in model.parameters())
    print(f"  Network: {obs_dim}→256→128→{act_dim} ({params:,} params)")

    N_ITERS = 12
    EPS_PER_ITER = 6
    MCTS_SIMS = 20
    print(f"  Config: {N_ITERS} iters × {EPS_PER_ITER} ep, MCTS-{MCTS_SIMS}")

    print("\n[Baseline]")
    rand_r, rand_s = run_agent(20, model=None)
    print(f"  Random: {rand_r:.1f} ± {rand_s:.1f}")

    print(f"\n{'='*60}")
    best = -1e9; t0 = time.time(); total_steps = 0
    seed_offset = int(time.time()) % 100000

    for it in range(N_ITERS):
        ti = time.time()
        data = collect_expert_data(EPS_PER_ITER, n_sims=MCTS_SIMS, max_turns=10,
                                   seed_offset=seed_offset + it * EPS_PER_ITER)
        if data is None:
            print(f"  Iter {it+1}: no data"); continue

        total_steps += data['n_steps']
        losses = [train_step(model, optimizer, data) for _ in range(3)]
        loss = np.mean([l['total'] for l in losses])
        acc = np.mean([l['accuracy'] for l in losses])
        dt = time.time() - ti

        print(f"  Iter {it+1:2d}/{N_ITERS} | "
              f"MCTS_r: {data['mean_reward']:+6.0f} | "
              f"Loss: {loss:.3f} | Acc: {acc:.0%} | "
              f"Steps: {data['n_steps']:3d} | {dt:.1f}s")

        if (it + 1) % 4 == 0 or it == N_ITERS - 1:
            nn_r, nn_s = run_agent(15, model=model)
            imp = nn_r - rand_r
            tag = " (NEW BEST)" if nn_r > best else ""
            if nn_r > best:
                best = nn_r
                torch.save(model.state_dict(), 'scripts/best_expert.pt')
            print(f"  [EVAL] NN: {nn_r:.1f}±{nn_s:.1f} | vs Random: {imp:+.1f}{tag}")

    print(f"\n{'='*60}")
    nn_r, nn_s = run_agent(30, model=model)
    print(f"  NN agent:  {nn_r:.1f} ± {nn_s:.1f}")
    print(f"  Random:    {rand_r:.1f} ± {rand_s:.1f}")
    print(f"  vs Random: {nn_r - rand_r:+.1f}")
    print(f"  Best:      {best:.1f}")
    print(f"  Time:      {time.time()-t0:.0f}s | Steps: {total_steps:,}")
    if nn_r > rand_r:
        print(f"\n  >> EXPERT NN BEATS RANDOM!")
    print("Done.")

if __name__ == '__main__':
    main()
