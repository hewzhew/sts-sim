"""
Debug Play: Load trained PPO model and trace one game step-by-step.
Shows exactly what the AI sees, decides, and what happens — for sanity checking.
"""
import numpy as np, torch, sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import sts_sim
sts_sim.set_verbose(False)

# ── Load model ──
from train_ppo import ActorCritic

env = sts_sim.PyStsSim(seed=42)
env.reset(seed=42)
obs_dim = len(env.get_observation())
act_dim = len(env.get_valid_actions_mask())

model = ActorCritic(obs_dim, act_dim, hidden=(256, 128))
weights_path = 'scripts/best_ppo.pt'
if os.path.exists(weights_path):
    model.load_state_dict(torch.load(weights_path, weights_only=True))
    print(f"Loaded weights from {weights_path}")
else:
    print("WARNING: No trained weights found, using random policy")
model.eval()

# ── Action names ──
def action_name(env, action_id):
    if action_id <= 9:
        try:
            info = env.get_game_info()
            hand = info.get('hand', [])
            if action_id < len(hand):
                card = hand[action_id]
                name = card.get('name', card.get('id', f'Card{action_id}'))
                cost = card.get('cost', '?')
                return f"Play [{action_id}] {name} (cost={cost})"
        except:
            pass
        return f"Play card [{action_id}]"
    elif action_id == 10:
        return "End Turn"
    elif 11 <= action_id <= 14:
        return f"Use Potion [{action_id-11}]"
    elif 20 <= action_id <= 29:
        return f"Choose Card [{action_id-20}]"
    elif 30 <= action_id <= 39:
        return f"Map/Navigate [{action_id}]"
    elif action_id == 99:
        return "Proceed/Skip"
    else:
        return f"Action [{action_id}]"

# ── Play one game with full trace ──
env = sts_sim.PyStsSim(seed=42)
env.reset(seed=42)
step = 0
combat_num = 0

for step in range(300):
    screen = env.get_screen_type()
    if screen == "GAME_OVER":
        print(f"\n{'='*50}")
        print(f"GAME OVER at step {step}, HP={env.get_hp()}")
        break
    
    obs = np.array(env.get_observation(), dtype=np.float32)
    mask = np.array(env.get_valid_actions_mask(), dtype=bool)
    valid = np.where(mask)[0]
    
    if not valid.any():
        done, r = env.step(99)
        if done: break
        continue
    
    if screen == "COMBAT":
        # Get detailed state
        try:
            info = env.get_game_info()
            hand = info.get('hand', [])
            enemies = info.get('enemies', [])
            energy = info.get('energy', '?')
            hp = env.get_hp()
            block = info.get('block', 0)
            
            # Only print on new combat or first step of turn
            hand_str = ", ".join([
                f"{c.get('name', c.get('id','?'))}({c.get('cost','?')})" 
                for c in hand
            ])
            enemy_str = ", ".join([
                f"{e.get('name','?')} HP={e.get('hp','?')}/{e.get('max_hp','?')}"
                for e in enemies if e.get('hp', 0) > 0
            ])
        except:
            hand_str = f"{len(valid)-1} cards playable"
            enemy_str = "?"
            energy = obs[1] * 3  # rough estimate
            hp = obs[0] * 80
            block = obs[2] * 100
        
        # Get PPO decision
        obs_t = torch.tensor(obs, dtype=torch.float32).unsqueeze(0)
        mask_t = torch.tensor(mask, dtype=torch.bool).unsqueeze(0)
        with torch.no_grad():
            logits, value = model(obs_t)
            logits[~mask_t] = -1e8
            probs = torch.softmax(logits, dim=-1).squeeze()
            action = logits.argmax(dim=-1).item()
        
        # Top 3 action preferences
        valid_probs = [(a, probs[a].item()) for a in valid]
        valid_probs.sort(key=lambda x: -x[1])
        top3 = valid_probs[:3]
        
        aname = action_name(env, action)
        top3_str = " | ".join([f"{action_name(env, a)}={p:.1%}" for a, p in top3])
        
        print(f"[{step:3d}] HP={hp} E={energy} Blk={block} | {enemy_str}")
        print(f"       Hand: {hand_str}")
        print(f"       → {aname}  (V={value.item():.1f})  Top: {top3_str}")
        
        done, r = env.step(action)
        if r != 0:
            print(f"       Reward: {r:+.1f}")
        if done: break
    else:
        # Non-combat: use heuristic
        for a in [99, 39, 34, 35, 36]:
            if a in valid:
                action = a
                break
        else:
            for a in range(30, 34):
                if a in valid:
                    action = a
                    break
            else:
                for a in range(20, 30):
                    if a in valid:
                        action = a
                        break
                else:
                    action = valid[0]
        
        aname = action_name(env, action)
        print(f"[{step:3d}] {screen}: {aname}")
        done, r = env.step(action)
        if done: break

print(f"\nFinal HP: {env.get_hp()}")
