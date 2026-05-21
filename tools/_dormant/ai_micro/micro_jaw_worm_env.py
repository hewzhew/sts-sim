import json
import os
import subprocess
from pathlib import Path
from typing import Any

try:
    import gymnasium as gym
    from gymnasium import spaces
except ImportError:  # pragma: no cover
    gym = None
    spaces = None

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DRIVER = ROOT / "target" / "debug" / (
    "micro_jaw_worm_env.exe" if os.name == "nt" else "micro_jaw_worm_env"
)
OBS_LEN = 96
ACTION_LEN = 11


class MinimalSpireEnv(gym.Env if gym else object):
    metadata = {"render_modes": []}

    def __init__(self, driver_path: str | os.PathLike[str] | None = None):
        if np is None:
            raise RuntimeError("numpy is required for MinimalSpireEnv")
        if gym is None or spaces is None:
            raise RuntimeError("gymnasium is required for MinimalSpireEnv")

        self.driver_path = Path(driver_path or DEFAULT_DRIVER)
        self.proc: subprocess.Popen[str] | None = None
        self.action_space = spaces.Discrete(ACTION_LEN)
        self.observation_space = spaces.Box(
            low=-10.0,
            high=10.0,
            shape=(OBS_LEN,),
            dtype=np.float32,
        )

    def reset(self, *, seed: int | None = None, options: dict[str, Any] | None = None):
        super().reset(seed=seed)
        self._ensure_proc()
        response = self._call({"cmd": "reset", "seed": 1 if seed is None else seed})
        return self._obs(response), self._info(response)

    def step(self, action: int):
        self._ensure_proc()
        response = self._call({"cmd": "step", "action": int(action)})
        return (
            self._obs(response),
            float(response["reward"]),
            bool(response["done"] and not response["truncated"]),
            bool(response["truncated"]),
            self._info(response),
        )

    def close(self):
        if self.proc is None:
            return
        try:
            self._send({"cmd": "close"})
        except Exception:
            pass
        self.proc.terminate()
        self.proc = None

    def _ensure_proc(self):
        if self.proc is not None and self.proc.poll() is None:
            return
        if not self.driver_path.exists():
            raise FileNotFoundError(
                f"Rust driver not found: {self.driver_path}. "
                "Build it with `cargo build --bin micro_jaw_worm_env`."
            )
        self.proc = subprocess.Popen(
            [str(self.driver_path)],
            cwd=str(ROOT),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def _send(self, payload: dict[str, Any]):
        assert self.proc is not None and self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()

    def _call(self, payload: dict[str, Any]) -> dict[str, Any]:
        self._send(payload)
        assert self.proc is not None and self.proc.stdout is not None
        line = self.proc.stdout.readline()
        if not line:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"Rust driver exited without response. stderr={stderr}")
        response = json.loads(line)
        if "error" in response:
            raise RuntimeError(response)
        return response

    def _obs(self, response: dict[str, Any]):
        return np.asarray(response["obs"], dtype=np.float32)

    def _info(self, response: dict[str, Any]) -> dict[str, Any]:
        info = dict(response["info"])
        info["action_mask"] = np.asarray(response["action_mask"], dtype=bool)
        return info
