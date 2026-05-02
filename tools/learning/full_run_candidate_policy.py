#!/usr/bin/env python3
from __future__ import annotations

from typing import Any

import numpy as np
import torch as th
from functools import partial
from sb3_contrib.common.maskable.policies import MaskableActorCriticPolicy
from stable_baselines3.common.type_aliases import Schedule
from torch import nn

from full_run_env import ACTION_FEATURES, BASE_OBS_DIM, MAX_ACTIONS


class FullRunCandidateScorerPolicy(MaskableActorCriticPolicy):
    """Shared candidate scorer for the flat full-run observation contract.

    The default SB3 MLP has one output head per action index, so knowledge learned
    for candidate slot 3 does not naturally transfer to slot 17. This policy
    instead encodes each candidate with shared weights and scores every legal
    candidate against the same encoded run state.
    """

    def __init__(
        self,
        *args: Any,
        state_dim: int = 128,
        candidate_dim: int = 96,
        hidden_dim: int = 128,
        **kwargs: Any,
    ) -> None:
        self.full_run_state_dim = int(state_dim)
        self.full_run_candidate_dim = int(candidate_dim)
        self.full_run_hidden_dim = int(hidden_dim)
        super().__init__(*args, **kwargs)

    def _build(self, lr_schedule: Schedule) -> None:
        activation = self.activation_fn
        self.state_encoder = nn.Sequential(
            nn.Linear(BASE_OBS_DIM, self.full_run_state_dim),
            activation(),
            nn.Linear(self.full_run_state_dim, self.full_run_state_dim),
            activation(),
        )
        self.candidate_encoder = nn.Sequential(
            nn.Linear(ACTION_FEATURES, self.full_run_candidate_dim),
            activation(),
            nn.Linear(self.full_run_candidate_dim, self.full_run_candidate_dim),
            activation(),
        )
        scorer_in = self.full_run_state_dim + self.full_run_candidate_dim
        self.candidate_scorer = nn.Sequential(
            nn.Linear(scorer_in, self.full_run_hidden_dim),
            activation(),
            nn.Linear(self.full_run_hidden_dim, 1),
        )
        value_in = self.full_run_state_dim + self.full_run_candidate_dim
        self.value_net = nn.Sequential(
            nn.Linear(value_in, self.full_run_hidden_dim),
            activation(),
            nn.Linear(self.full_run_hidden_dim, 1),
        )

        if self.ortho_init:
            module_gains = {
                self.state_encoder: np.sqrt(2),
                self.candidate_encoder: np.sqrt(2),
                self.candidate_scorer: 0.01,
                self.value_net: 1.0,
            }
            for module, gain in module_gains.items():
                module.apply(partial(self.init_weights, gain=gain))

        self.optimizer = self.optimizer_class(
            self.parameters(),
            lr=lr_schedule(1),
            **self.optimizer_kwargs,
        )

    def _split_full_run_obs(self, obs: th.Tensor) -> tuple[th.Tensor, th.Tensor]:
        if obs.dim() == 1:
            obs = obs.unsqueeze(0)
        base = obs[:, :BASE_OBS_DIM]
        action_flat = obs[:, BASE_OBS_DIM : BASE_OBS_DIM + MAX_ACTIONS * ACTION_FEATURES]
        candidates = action_flat.reshape((-1, MAX_ACTIONS, ACTION_FEATURES))
        return base, candidates

    def _encode_state_and_candidates(self, obs: th.Tensor) -> tuple[th.Tensor, th.Tensor, th.Tensor]:
        base, candidates = self._split_full_run_obs(obs)
        state_latent = self.state_encoder(base)
        candidate_latent = self.candidate_encoder(candidates)

        candidate_valid = candidates[:, :, 0:1].clamp(0.0, 1.0)
        valid_count = candidate_valid.sum(dim=1).clamp_min(1.0)
        pooled_candidates = (candidate_latent * candidate_valid).sum(dim=1) / valid_count
        value_latent = th.cat([state_latent, pooled_candidates], dim=-1)
        return state_latent, candidate_latent, value_latent

    def _action_logits_and_values(self, obs: th.Tensor) -> tuple[th.Tensor, th.Tensor]:
        state_latent, candidate_latent, value_latent = self._encode_state_and_candidates(obs)
        state_for_each_candidate = state_latent.unsqueeze(1).expand(-1, MAX_ACTIONS, -1)
        scorer_input = th.cat([state_for_each_candidate, candidate_latent], dim=-1)
        logits = self.candidate_scorer(scorer_input).squeeze(-1)
        values = self.value_net(value_latent)
        return logits, values

    def _distribution_from_obs(
        self,
        obs: th.Tensor,
        action_masks: np.ndarray | th.Tensor | None = None,
    ):
        logits, _ = self._action_logits_and_values(obs)
        distribution = self.action_dist.proba_distribution(action_logits=logits)
        if action_masks is not None:
            distribution.apply_masking(action_masks)
        return distribution

    def forward(
        self,
        obs: th.Tensor,
        deterministic: bool = False,
        action_masks: np.ndarray | None = None,
    ) -> tuple[th.Tensor, th.Tensor, th.Tensor]:
        logits, values = self._action_logits_and_values(obs)
        distribution = self.action_dist.proba_distribution(action_logits=logits)
        if action_masks is not None:
            distribution.apply_masking(action_masks)
        actions = distribution.get_actions(deterministic=deterministic)
        log_prob = distribution.log_prob(actions)
        return actions.reshape((-1, *self.action_space.shape)), values, log_prob

    def get_distribution(self, obs: th.Tensor, action_masks: np.ndarray | None = None):
        return self._distribution_from_obs(obs, action_masks)

    def evaluate_actions(
        self,
        obs: th.Tensor,
        actions: th.Tensor,
        action_masks: th.Tensor | None = None,
    ) -> tuple[th.Tensor, th.Tensor, th.Tensor | None]:
        logits, values = self._action_logits_and_values(obs)
        distribution = self.action_dist.proba_distribution(action_logits=logits)
        if action_masks is not None:
            distribution.apply_masking(action_masks)
        log_prob = distribution.log_prob(actions)
        return values, log_prob, distribution.entropy()

    def predict_values(self, obs: th.Tensor) -> th.Tensor:
        _, values = self._action_logits_and_values(obs)
        return values
