import argparse
import random
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch

from micro_jaw_worm_env import DEFAULT_DRIVER, MinimalSpireEnv
from train_micro_jaw_worm_ppo import Agent


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CHECKPOINT = ROOT / "tools" / "artifacts" / "micro_jaw_worm_ppo.pt"


@dataclass
class EpisodeResult:
    seed: int
    reward: float
    steps: int
    killed: bool
    truncated: bool
    player_hp: int
    enemy_hp: int


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", type=Path, default=DEFAULT_CHECKPOINT)
    parser.add_argument("--driver", type=Path, default=DEFAULT_DRIVER)
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--seed", type=int, default=10_000)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--no-random-baseline", action="store_true")
    parser.add_argument("--print-worst", type=int, default=5)
    return parser.parse_args()


def load_agent(checkpoint: Path, device: torch.device) -> Agent:
    if not checkpoint.exists():
        raise FileNotFoundError(
            f"checkpoint not found: {checkpoint}. Run train_micro_jaw_worm_ppo.py first."
        )
    payload = torch.load(checkpoint, map_location=device, weights_only=False)
    agent = Agent().to(device)
    agent.load_state_dict(payload["model_state_dict"])
    agent.eval()
    return agent


def run_policy(name: str, env: MinimalSpireEnv, seeds: list[int], agent: Agent | None, device):
    rng = random.Random(12345)
    results = []
    for seed in seeds:
        obs, info = env.reset(seed=seed)
        total_reward = 0.0
        steps = 0
        while True:
            if name == "random_legal":
                legal = [idx for idx, allowed in enumerate(info["action_mask"]) if allowed]
                action = rng.choice(legal)
            else:
                obs_t = torch.tensor(obs, dtype=torch.float32, device=device).unsqueeze(0)
                mask_t = torch.tensor(
                    info["action_mask"], dtype=torch.bool, device=device
                ).unsqueeze(0)
                with torch.no_grad():
                    action = int(agent.choose_deterministic(obs_t, mask_t).item())

            obs, reward, terminated, truncated, info = env.step(action)
            total_reward += reward
            steps += 1
            if terminated or truncated:
                results.append(
                    EpisodeResult(
                        seed=seed,
                        reward=total_reward,
                        steps=steps,
                        killed=bool(info["killed_enemy"]),
                        truncated=bool(truncated),
                        player_hp=int(info["player_hp"]),
                        enemy_hp=int(info["enemy_hp"]),
                    )
                )
                break
    return results


def summarize(name: str, results: list[EpisodeResult], worst_count: int):
    rewards = np.asarray([result.reward for result in results], dtype=np.float32)
    steps = np.asarray([result.steps for result in results], dtype=np.float32)
    kills = np.asarray([1.0 if result.killed else 0.0 for result in results], dtype=np.float32)
    hps = np.asarray([result.player_hp for result in results], dtype=np.float32)
    hp_lost = 80.0 - hps
    truncs = np.asarray([1.0 if result.truncated else 0.0 for result in results], dtype=np.float32)

    print(
        "{name}: episodes={episodes} kill={kill:.3f} return={ret:.2f} "
        "return_std={ret_std:.2f} len={length:.2f} hp={hp:.2f} "
        "hp_lost={hp_lost:.2f} min_hp={min_hp:.0f} max_hp={max_hp:.0f} "
        "truncated={trunc:.3f}".format(
            name=name,
            episodes=len(results),
            kill=float(kills.mean()),
            ret=float(rewards.mean()),
            ret_std=float(rewards.std()),
            length=float(steps.mean()),
            hp=float(hps.mean()),
            hp_lost=float(hp_lost.mean()),
            min_hp=float(hps.min()),
            max_hp=float(hps.max()),
            trunc=float(truncs.mean()),
        )
    )

    worst = sorted(results, key=lambda result: (result.player_hp, result.reward))[:worst_count]
    if worst:
        print("  worst_by_hp:")
        for result in worst:
            print(
                "    seed={seed} hp={hp} enemy_hp={enemy_hp} reward={reward:.2f} "
                "steps={steps} killed={killed} truncated={truncated}".format(
                    seed=result.seed,
                    hp=result.player_hp,
                    enemy_hp=result.enemy_hp,
                    reward=result.reward,
                    steps=result.steps,
                    killed=result.killed,
                    truncated=result.truncated,
                )
            )


def main():
    args = parse_args()
    device = torch.device(args.device)
    seeds = [args.seed + idx for idx in range(args.episodes)]

    agent = load_agent(args.checkpoint, device)
    env = MinimalSpireEnv(args.driver)
    try:
        ppo_results = run_policy("ppo_argmax", env, seeds, agent, device)
        summarize("ppo_argmax", ppo_results, args.print_worst)

        if not args.no_random_baseline:
            random_results = run_policy("random_legal", env, seeds, None, device)
            summarize("random_legal", random_results, args.print_worst)
    finally:
        env.close()


if __name__ == "__main__":
    main()
