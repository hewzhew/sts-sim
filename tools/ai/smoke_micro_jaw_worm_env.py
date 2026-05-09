import argparse
import json
import os
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DRIVER = ROOT / "target" / "debug" / (
    "micro_jaw_worm_env.exe" if os.name == "nt" else "micro_jaw_worm_env"
)


def call(proc: subprocess.Popen[str], payload: dict):
    assert proc.stdin is not None and proc.stdout is not None
    proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        stderr = proc.stderr.read() if proc.stderr else ""
        raise RuntimeError(f"driver exited without response: {stderr}")
    response = json.loads(line)
    if "error" in response:
        raise RuntimeError(response)
    return response


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", default=str(DEFAULT_DRIVER))
    parser.add_argument("--episodes", type=int, default=5)
    parser.add_argument("--max-steps", type=int, default=80)
    args = parser.parse_args()

    driver = Path(args.driver)
    if not driver.exists():
        raise SystemExit(
            f"driver not found: {driver}\n"
            "build it with: cargo build --bin micro_jaw_worm_env"
        )

    proc = subprocess.Popen(
        [str(driver)],
        cwd=str(ROOT),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    try:
        for episode in range(args.episodes):
            response = call(proc, {"cmd": "reset", "seed": episode + 1})
            total_reward = 0.0
            steps = 0
            while not response["done"] and steps < args.max_steps:
                legal = [
                    idx
                    for idx, allowed in enumerate(response["action_mask"])
                    if allowed
                ]
                action = random.choice(legal)
                response = call(proc, {"cmd": "step", "action": action})
                total_reward += float(response["reward"])
                steps += 1
            info = response["info"]
            print(
                "episode={episode} reward={reward:.2f} steps={steps} "
                "player_hp={player_hp} enemy_hp={enemy_hp} killed={killed} "
                "truncated={truncated}".format(
                    episode=episode,
                    reward=total_reward,
                    steps=steps,
                    player_hp=info["player_hp"],
                    enemy_hp=info["enemy_hp"],
                    killed=info["killed_enemy"],
                    truncated=response["truncated"],
                )
            )
        assert proc.stdin is not None
        proc.stdin.write(json.dumps({"cmd": "close"}, separators=(",", ":")) + "\n")
        proc.stdin.flush()
    finally:
        proc.terminate()


if __name__ == "__main__":
    main()
