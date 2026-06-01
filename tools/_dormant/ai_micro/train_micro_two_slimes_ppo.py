import argparse
import random
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.distributions.categorical import Categorical

from micro_two_slimes_env import ACTION_LEN, DEFAULT_DRIVER, OBS_LEN, MicroTwoSlimesEnv


ROOT = Path(__file__).resolve().parents[2]


@dataclass
class Args:
    driver: Path
    seed: int
    total_timesteps: int
    learning_rate: float
    num_envs: int
    num_steps: int
    gamma: float
    gae_lambda: float
    num_minibatches: int
    update_epochs: int
    norm_adv: bool
    clip_coef: float
    clip_vloss: bool
    ent_coef: float
    vf_coef: float
    max_grad_norm: float
    eval_episodes: int
    save_path: Path
    device: str


def parse_args() -> Args:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=DEFAULT_DRIVER)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--total-timesteps", type=int, default=20_000)
    parser.add_argument("--learning-rate", type=float, default=2.5e-4)
    parser.add_argument("--num-envs", type=int, default=4)
    parser.add_argument("--num-steps", type=int, default=128)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--gae-lambda", type=float, default=0.95)
    parser.add_argument("--num-minibatches", type=int, default=4)
    parser.add_argument("--update-epochs", type=int, default=4)
    parser.add_argument("--no-norm-adv", action="store_true")
    parser.add_argument("--clip-coef", type=float, default=0.2)
    parser.add_argument("--no-clip-vloss", action="store_true")
    parser.add_argument("--ent-coef", type=float, default=0.01)
    parser.add_argument("--vf-coef", type=float, default=0.5)
    parser.add_argument("--max-grad-norm", type=float, default=0.5)
    parser.add_argument("--eval-episodes", type=int, default=10)
    parser.add_argument(
        "--save-path",
        type=Path,
        default=ROOT / "tools" / "artifacts" / "micro_two_slimes_ppo.pt",
    )
    parser.add_argument("--device", default="cpu")
    ns = parser.parse_args()
    return Args(
        driver=ns.driver,
        seed=ns.seed,
        total_timesteps=ns.total_timesteps,
        learning_rate=ns.learning_rate,
        num_envs=ns.num_envs,
        num_steps=ns.num_steps,
        gamma=ns.gamma,
        gae_lambda=ns.gae_lambda,
        num_minibatches=ns.num_minibatches,
        update_epochs=ns.update_epochs,
        norm_adv=not ns.no_norm_adv,
        clip_coef=ns.clip_coef,
        clip_vloss=not ns.no_clip_vloss,
        ent_coef=ns.ent_coef,
        vf_coef=ns.vf_coef,
        max_grad_norm=ns.max_grad_norm,
        eval_episodes=ns.eval_episodes,
        save_path=ns.save_path,
        device=ns.device,
    )


def layer_init(layer: nn.Linear, std: float = np.sqrt(2), bias_const: float = 0.0):
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


class Agent(nn.Module):
    def __init__(self):
        super().__init__()
        self.critic = nn.Sequential(
            layer_init(nn.Linear(OBS_LEN, 128)),
            nn.Tanh(),
            layer_init(nn.Linear(128, 128)),
            nn.Tanh(),
            layer_init(nn.Linear(128, 1), std=1.0),
        )
        self.actor = nn.Sequential(
            layer_init(nn.Linear(OBS_LEN, 128)),
            nn.Tanh(),
            layer_init(nn.Linear(128, 128)),
            nn.Tanh(),
            layer_init(nn.Linear(128, ACTION_LEN), std=0.01),
        )

    def get_value(self, obs: torch.Tensor) -> torch.Tensor:
        return self.critic(obs).squeeze(-1)

    def get_action_and_value(
        self,
        obs: torch.Tensor,
        action_mask: torch.Tensor,
        action: torch.Tensor | None = None,
    ):
        logits = self.actor(obs)
        masked_logits = logits.masked_fill(~action_mask.bool(), -1e9)
        dist = Categorical(logits=masked_logits)
        if action is None:
            action = dist.sample()
        return (
            action,
            dist.log_prob(action),
            dist.entropy(),
            self.critic(obs).squeeze(-1),
        )

    def choose_deterministic(self, obs: torch.Tensor, action_mask: torch.Tensor) -> torch.Tensor:
        logits = self.actor(obs)
        masked_logits = logits.masked_fill(~action_mask.bool(), -1e9)
        return torch.argmax(masked_logits, dim=-1)


def make_envs(args: Args):
    return [MicroTwoSlimesEnv(args.driver) for _ in range(args.num_envs)]


def reset_envs(envs, seed: int):
    obs = []
    masks = []
    infos = []
    for idx, env in enumerate(envs):
        next_obs, info = env.reset(seed=seed + idx)
        obs.append(next_obs)
        masks.append(info["action_mask"])
        infos.append(info)
    return np.stack(obs), np.stack(masks), infos


def step_envs(envs, actions):
    next_obs = []
    next_masks = []
    rewards = []
    dones = []
    infos = []
    for env, action in zip(envs, actions):
        obs, reward, terminated, truncated, info = env.step(int(action))
        done = terminated or truncated
        if done:
            final_info = info
            obs, info = env.reset(seed=random.randrange(1, 2_147_483_647))
            info["final_info"] = final_info
        next_obs.append(obs)
        next_masks.append(info["action_mask"])
        rewards.append(reward)
        dones.append(done)
        infos.append(info)
    return (
        np.stack(next_obs),
        np.stack(next_masks),
        np.asarray(rewards, dtype=np.float32),
        np.asarray(dones, dtype=np.float32),
        infos,
    )


def evaluate(agent: Agent, args: Args, device: torch.device) -> dict[str, float]:
    env = MicroTwoSlimesEnv(args.driver)
    returns = []
    lengths = []
    kills = []
    final_hps = []
    try:
        for episode in range(args.eval_episodes):
            obs, info = env.reset(seed=args.seed + 10_000 + episode)
            total_reward = 0.0
            length = 0
            while True:
                obs_t = torch.tensor(obs, dtype=torch.float32, device=device).unsqueeze(0)
                mask_t = torch.tensor(
                    info["action_mask"], dtype=torch.bool, device=device
                ).unsqueeze(0)
                with torch.no_grad():
                    action = agent.choose_deterministic(obs_t, mask_t).item()
                obs, reward, terminated, truncated, info = env.step(action)
                total_reward += reward
                length += 1
                if terminated or truncated:
                    returns.append(total_reward)
                    lengths.append(length)
                    kills.append(1.0 if info["killed_all"] else 0.0)
                    final_hps.append(float(info["player_hp"]))
                    break
    finally:
        env.close()

    return {
        "eval_return": float(np.mean(returns)) if returns else 0.0,
        "eval_len": float(np.mean(lengths)) if lengths else 0.0,
        "eval_kill_rate": float(np.mean(kills)) if kills else 0.0,
        "eval_final_hp": float(np.mean(final_hps)) if final_hps else 0.0,
        "eval_hp_lost": float(80.0 - np.mean(final_hps)) if final_hps else 0.0,
        "eval_min_hp": float(np.min(final_hps)) if final_hps else 0.0,
    }


def train(args: Args):
    if not args.driver.exists():
        raise FileNotFoundError(
            f"Rust driver not found: {args.driver}. "
            "Build it with `cargo build --bin micro_two_slimes_env`."
        )

    random.seed(args.seed)
    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    torch.set_num_threads(1)
    device = torch.device(args.device)

    batch_size = args.num_envs * args.num_steps
    minibatch_size = batch_size // args.num_minibatches
    num_updates = args.total_timesteps // batch_size
    if num_updates <= 0:
        raise ValueError("total_timesteps must be at least num_envs * num_steps")

    envs = make_envs(args)
    agent = Agent().to(device)
    optimizer = optim.Adam(agent.parameters(), lr=args.learning_rate, eps=1e-5)

    obs = torch.zeros((args.num_steps, args.num_envs, OBS_LEN), device=device)
    masks = torch.zeros((args.num_steps, args.num_envs, ACTION_LEN), device=device, dtype=torch.bool)
    actions = torch.zeros((args.num_steps, args.num_envs), device=device, dtype=torch.long)
    logprobs = torch.zeros((args.num_steps, args.num_envs), device=device)
    rewards = torch.zeros((args.num_steps, args.num_envs), device=device)
    terminals = torch.zeros((args.num_steps, args.num_envs), device=device)
    values = torch.zeros((args.num_steps, args.num_envs), device=device)

    next_obs_np, next_mask_np, _ = reset_envs(envs, args.seed)
    next_obs = torch.tensor(next_obs_np, dtype=torch.float32, device=device)
    next_mask = torch.tensor(next_mask_np, dtype=torch.bool, device=device)
    recent_returns: list[float] = []
    recent_lengths: list[int] = []
    recent_kills: list[float] = []
    episode_returns = np.zeros(args.num_envs, dtype=np.float32)
    episode_lengths = np.zeros(args.num_envs, dtype=np.int32)
    global_step = 0
    start_time = time.time()

    try:
        for update in range(1, num_updates + 1):
            for step in range(args.num_steps):
                global_step += args.num_envs
                obs[step] = next_obs
                masks[step] = next_mask

                with torch.no_grad():
                    action, logprob, _, value = agent.get_action_and_value(next_obs, next_mask)
                actions[step] = action
                logprobs[step] = logprob
                values[step] = value

                next_obs_np, next_mask_np, reward_np, done_np, infos = step_envs(
                    envs, action.cpu().numpy()
                )
                rewards[step] = torch.tensor(reward_np, dtype=torch.float32, device=device)
                terminals[step] = torch.tensor(done_np, dtype=torch.float32, device=device)

                episode_returns += reward_np
                episode_lengths += 1
                for idx, done in enumerate(done_np):
                    if done:
                        final_info = infos[idx]["final_info"]
                        recent_returns.append(float(episode_returns[idx]))
                        recent_lengths.append(int(episode_lengths[idx]))
                        recent_kills.append(1.0 if final_info["killed_all"] else 0.0)
                        episode_returns[idx] = 0.0
                        episode_lengths[idx] = 0

                next_obs = torch.tensor(next_obs_np, dtype=torch.float32, device=device)
                next_mask = torch.tensor(next_mask_np, dtype=torch.bool, device=device)

            with torch.no_grad():
                next_value = agent.get_value(next_obs)
                advantages = torch.zeros_like(rewards, device=device)
                lastgaelam = torch.zeros(args.num_envs, device=device)
                for t in reversed(range(args.num_steps)):
                    if t == args.num_steps - 1:
                        nextvalues = next_value
                    else:
                        nextvalues = values[t + 1]
                    nextnonterminal = 1.0 - terminals[t]
                    delta = rewards[t] + args.gamma * nextvalues * nextnonterminal - values[t]
                    lastgaelam = (
                        delta
                        + args.gamma
                        * args.gae_lambda
                        * nextnonterminal
                        * lastgaelam
                    )
                    advantages[t] = lastgaelam
                returns = advantages + values

            b_obs = obs.reshape((-1, OBS_LEN))
            b_masks = masks.reshape((-1, ACTION_LEN))
            b_logprobs = logprobs.reshape(-1)
            b_actions = actions.reshape(-1)
            b_advantages = advantages.reshape(-1)
            b_returns = returns.reshape(-1)
            b_values = values.reshape(-1)

            b_inds = np.arange(batch_size)
            clipfracs = []
            for _ in range(args.update_epochs):
                np.random.shuffle(b_inds)
                for start in range(0, batch_size, minibatch_size):
                    end = start + minibatch_size
                    mb_inds = b_inds[start:end]

                    _, newlogprob, entropy, newvalue = agent.get_action_and_value(
                        b_obs[mb_inds], b_masks[mb_inds], b_actions[mb_inds]
                    )
                    logratio = newlogprob - b_logprobs[mb_inds]
                    ratio = logratio.exp()

                    with torch.no_grad():
                        old_approx_kl = (-logratio).mean()
                        approx_kl = ((ratio - 1) - logratio).mean()
                        clipfracs.append(
                            ((ratio - 1.0).abs() > args.clip_coef).float().mean().item()
                        )

                    mb_advantages = b_advantages[mb_inds]
                    if args.norm_adv:
                        mb_advantages = (mb_advantages - mb_advantages.mean()) / (
                            mb_advantages.std() + 1e-8
                        )

                    pg_loss1 = -mb_advantages * ratio
                    pg_loss2 = -mb_advantages * torch.clamp(
                        ratio, 1 - args.clip_coef, 1 + args.clip_coef
                    )
                    pg_loss = torch.max(pg_loss1, pg_loss2).mean()

                    newvalue = newvalue.view(-1)
                    if args.clip_vloss:
                        v_loss_unclipped = (newvalue - b_returns[mb_inds]) ** 2
                        v_clipped = b_values[mb_inds] + torch.clamp(
                            newvalue - b_values[mb_inds],
                            -args.clip_coef,
                            args.clip_coef,
                        )
                        v_loss_clipped = (v_clipped - b_returns[mb_inds]) ** 2
                        v_loss = 0.5 * torch.max(v_loss_unclipped, v_loss_clipped).mean()
                    else:
                        v_loss = 0.5 * ((newvalue - b_returns[mb_inds]) ** 2).mean()

                    entropy_loss = entropy.mean()
                    loss = pg_loss - args.ent_coef * entropy_loss + args.vf_coef * v_loss

                    optimizer.zero_grad()
                    loss.backward()
                    nn.utils.clip_grad_norm_(agent.parameters(), args.max_grad_norm)
                    optimizer.step()

            y_pred, y_true = b_values.cpu().numpy(), b_returns.cpu().numpy()
            var_y = np.var(y_true)
            explained_var = np.nan if var_y == 0 else 1 - np.var(y_true - y_pred) / var_y
            recent_window = slice(max(0, len(recent_returns) - 50), len(recent_returns))
            recent_return = (
                float(np.mean(recent_returns[recent_window]))
                if recent_returns
                else 0.0
            )
            recent_len = (
                float(np.mean(recent_lengths[recent_window]))
                if recent_lengths
                else 0.0
            )
            recent_kill = (
                float(np.mean(recent_kills[recent_window])) if recent_kills else 0.0
            )
            sps = int(global_step / max(time.time() - start_time, 1e-6))
            print(
                "update={update} step={step} sps={sps} return={ret:.2f} "
                "len={length:.1f} kill={kill:.2f} loss={loss:.3f} "
                "pg={pg:.3f} v={v:.3f} ent={ent:.3f} kl={kl:.4f} "
                "clipfrac={clip:.3f} ev={ev:.3f}".format(
                    update=update,
                    step=global_step,
                    sps=sps,
                    ret=recent_return,
                    length=recent_len,
                    kill=recent_kill,
                    loss=loss.item(),
                    pg=pg_loss.item(),
                    v=v_loss.item(),
                    ent=entropy_loss.item(),
                    kl=approx_kl.item(),
                    clip=float(np.mean(clipfracs)) if clipfracs else 0.0,
                    ev=explained_var,
                ),
                flush=True,
            )

        metrics = evaluate(agent, args, device)
        print(
            "eval return={eval_return:.2f} len={eval_len:.1f} "
            "kill={eval_kill_rate:.2f} final_hp={eval_final_hp:.1f} "
            "hp_lost={eval_hp_lost:.1f} min_hp={eval_min_hp:.1f}".format(**metrics),
            flush=True,
        )
        args.save_path.parent.mkdir(parents=True, exist_ok=True)
        torch.save(
            {
                "model_state_dict": agent.state_dict(),
                "args": vars(args),
                "metrics": metrics,
            },
            args.save_path,
        )
        print(f"saved={args.save_path}", flush=True)
    finally:
        for env in envs:
            env.close()


if __name__ == "__main__":
    train(parse_args())
