"""Test: verify card rewards actually increase master_deck size."""
import sts_sim

env = sts_sim.PyStsSim(seed=42)
env.reset(seed=42)

for step in range(200):
    screen = env.get_screen_type()
    if screen == 'GAME_OVER':
        print(f"[{step}] GAME OVER")
        break
    
    obs = env.get_observation_dict()
    mask = env.get_valid_actions_mask()
    valid = [i for i, v in enumerate(mask) if v]
    state_str = env.get_state_str()
    
    if screen == 'REWARD':
        print(f"[{step}] REWARD valid={valid}")
        if 30 in valid:
            done, r = env.step(30)
            # Check state after picking
            obs2 = env.get_observation_dict()
            new_draw = obs2.get('draw_pile_size', 0)
            new_hand = len(obs2.get('hand', []))
            new_disc = obs2.get('discard_pile_size', 0)
            new_total = new_draw + new_hand + new_disc
            print(f"  -> Picked card 0 (r={r:.1f}) newTotal={new_total}")
        elif 33 in valid:
            done, r = env.step(33)
        else:
            done, r = env.step(99)
        if done: break
    elif screen == 'COMBAT':
        hand = obs.get('hand', [])
        turn = obs.get('turn', 0)
        draw = obs.get('draw_pile_size', 0)
        disc = obs.get('discard_pile_size', 0)
        total = len(hand) + draw + disc
        if turn == 1:
            print(f"[{step}] COMBAT T1 Hand={len(hand)} Draw={draw} Disc={disc} Total={total}")
        # play first card or end turn
        if 10 in valid:
            done, r = env.step(10)
        else:
            done, r = env.step(valid[0])
        if done: break
    else:
        a = 99 if 99 in valid else valid[0]
        done, r = env.step(a)
        if done: break
