# sts_sim - Slay the Spire Simulator

A high-performance, headless Slay the Spire simulator written in Rust with Python bindings for RL training.

## Features

- **Data-driven**: Card and enemy logic loaded from JSON
- **Deterministic**: Seeded RNG for reproducible simulations
- **Fast**: Optimized Rust implementation with parallel batch simulation
- **Python bindings**: PyO3 FFI for Gym-style RL training

## Installation

```bash
# Build with maturin
pip install maturin
maturin develop --release
```

## Quick Start

```python
import sts_sim

# Create environment
env = sts_sim.PyStsSim(seed=42)

# Check current screen
print("Screen:", env.get_screen_type())

# Take an action (stub for now)
done, reward = env.step(0)
print(f"Done: {done}, Reward: {reward}")

# Get state as JSON
print(env.get_state_str())
```

## License

MIT
