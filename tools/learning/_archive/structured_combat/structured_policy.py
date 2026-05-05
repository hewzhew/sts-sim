#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import torch
from torch import nn
from torch.distributions import Categorical

from structured_combat_env import (
    ACTION_TYPE_CANCEL,
    ACTION_TYPE_CHOICE,
    ACTION_TYPE_COUNT,
    ACTION_TYPE_END_TURN,
    ACTION_TYPE_PLAY_CARD,
    ACTION_TYPE_PROCEED,
    ACTION_TYPE_USE_POTION,
    MAX_CHOICE_OPTIONS,
    MAX_HAND,
    MAX_MONSTERS,
    MAX_POTIONS,
    MAX_TARGET_OPTIONS,
)


def _masked_logits(logits: torch.Tensor, mask: torch.Tensor) -> torch.Tensor:
    bool_mask = mask > 0.5
    fallback = torch.zeros_like(bool_mask[..., :1], dtype=torch.bool)
    bool_mask = torch.where(bool_mask.any(dim=-1, keepdim=True), bool_mask, ~fallback)
    return logits.masked_fill(~bool_mask, -1e9)


def _masked_mean(encoded: torch.Tensor, mask: torch.Tensor) -> torch.Tensor:
    weights = mask.unsqueeze(-1)
    denom = weights.sum(dim=-2).clamp_min(1.0)
    return (encoded * weights).sum(dim=-2) / denom


class PowerEncoder(nn.Module):
    def __init__(self, power_vocab: int, embed_dim: int = 12, hidden_dim: int = 32) -> None:
        super().__init__()
        self.embedding = nn.Embedding(power_vocab, embed_dim)
        self.mlp = nn.Sequential(
            nn.Linear(embed_dim + 4, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )

    def forward(self, ids: torch.Tensor, numeric: torch.Tensor) -> torch.Tensor:
        embedded = self.embedding(ids.long())
        return self.mlp(torch.cat([embedded, numeric], dim=-1))


class CardEncoder(nn.Module):
    def __init__(self, card_vocab: int, hidden_dim: int = 64) -> None:
        super().__init__()
        self.card_embedding = nn.Embedding(card_vocab, 24)
        self.type_embedding = nn.Embedding(8, 8)
        self.target_embedding = nn.Embedding(8, 8)
        self.mlp = nn.Sequential(
            nn.Linear(24 + 8 + 8 + 7, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )

    def forward(
        self,
        card_ids: torch.Tensor,
        type_ids: torch.Tensor,
        target_ids: torch.Tensor,
        numeric: torch.Tensor,
    ) -> torch.Tensor:
        pieces = [
            self.card_embedding(card_ids.long()),
            self.type_embedding(type_ids.long()),
            self.target_embedding(target_ids.long()),
            numeric,
        ]
        return self.mlp(torch.cat(pieces, dim=-1))


class PotionEncoder(nn.Module):
    def __init__(self, potion_vocab: int, hidden_dim: int = 32) -> None:
        super().__init__()
        self.potion_embedding = nn.Embedding(potion_vocab, 16)
        self.target_embedding = nn.Embedding(8, 8)
        self.mlp = nn.Sequential(
            nn.Linear(16 + 8 + 2, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )

    def forward(self, potion_ids: torch.Tensor, target_ids: torch.Tensor, numeric: torch.Tensor) -> torch.Tensor:
        return self.mlp(
            torch.cat(
                [
                    self.potion_embedding(potion_ids.long()),
                    self.target_embedding(target_ids.long()),
                    numeric,
                ],
                dim=-1,
            )
        )


class MonsterEncoder(nn.Module):
    def __init__(self, monster_vocab: int, intent_vocab: int, hidden_dim: int = 64) -> None:
        super().__init__()
        self.monster_embedding = nn.Embedding(monster_vocab, 20)
        self.intent_embedding = nn.Embedding(intent_vocab, 12)
        self.mlp = nn.Sequential(
            nn.Linear(20 + 12 + 20 + 32, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )

    def forward(
        self,
        monster_ids: torch.Tensor,
        intent_ids: torch.Tensor,
        numeric: torch.Tensor,
        power_pooled: torch.Tensor,
    ) -> torch.Tensor:
        return self.mlp(
            torch.cat(
                [
                    self.monster_embedding(monster_ids.long()),
                    self.intent_embedding(intent_ids.long()),
                    numeric,
                    power_pooled,
                ],
                dim=-1,
            )
        )


class ChoiceEncoder(nn.Module):
    def __init__(self, card_vocab: int, hidden_dim: int = 32) -> None:
        super().__init__()
        self.card_embedding = nn.Embedding(card_vocab, 16)
        self.source_embedding = nn.Embedding(8, 8)
        self.mlp = nn.Sequential(
            nn.Linear(16 + 8 + 4, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )

    def forward(self, card_ids: torch.Tensor, source_ids: torch.Tensor, numeric: torch.Tensor) -> torch.Tensor:
        return self.mlp(
            torch.cat(
                [self.card_embedding(card_ids.long()), self.source_embedding(source_ids.long()), numeric],
                dim=-1,
            )
        )


@dataclass
class EncodedState:
    tactical: torch.Tensor
    value: torch.Tensor
    probes: torch.Tensor
    hand_repr: torch.Tensor
    potion_repr: torch.Tensor
    monster_repr: torch.Tensor
    choice_repr: torch.Tensor
    action_type_logits: torch.Tensor
    card_logits: torch.Tensor
    potion_logits: torch.Tensor
    choice_logits: torch.Tensor


class StructuredPolicyNet(nn.Module):
    def __init__(
        self,
        *,
        card_vocab: int,
        potion_vocab: int,
        power_vocab: int,
        monster_vocab: int,
        intent_vocab: int,
        latent_dim: int = 32,
    ) -> None:
        super().__init__()
        self.player_power_encoder = PowerEncoder(power_vocab)
        self.monster_power_encoder = PowerEncoder(power_vocab)
        self.card_encoder = CardEncoder(card_vocab)
        self.potion_encoder = PotionEncoder(potion_vocab)
        self.monster_encoder = MonsterEncoder(monster_vocab, intent_vocab)
        self.choice_encoder = ChoiceEncoder(card_vocab)

        trunk_in = 20 + 32 + 64 + 32 + 64 + 32
        self.trunk = nn.Sequential(
            nn.Linear(trunk_in, 192),
            nn.ReLU(),
            nn.Linear(192, 128),
            nn.ReLU(),
        )
        self.tactical = nn.Sequential(nn.Linear(128, latent_dim), nn.Tanh())
        self.probe_head = nn.Linear(latent_dim, 4)
        self.value_head = nn.Linear(latent_dim, 1)
        self.action_type_head = nn.Linear(latent_dim, ACTION_TYPE_COUNT)
        self.card_head = nn.Sequential(nn.Linear(latent_dim + 64, 64), nn.ReLU(), nn.Linear(64, 1))
        self.potion_head = nn.Sequential(nn.Linear(latent_dim + 32, 32), nn.ReLU(), nn.Linear(32, 1))
        self.choice_head = nn.Sequential(nn.Linear(latent_dim + 32, 32), nn.ReLU(), nn.Linear(32, 1))
        self.none_target_head = nn.Sequential(nn.Linear(latent_dim + 64, 32), nn.ReLU(), nn.Linear(32, 1))
        self.monster_target_head = nn.Sequential(
            nn.Linear(latent_dim + 64 + 64, 64),
            nn.ReLU(),
            nn.Linear(64, 1),
        )
        self.none_potion_target_head = nn.Sequential(nn.Linear(latent_dim + 32, 32), nn.ReLU(), nn.Linear(32, 1))
        self.monster_potion_target_head = nn.Sequential(
            nn.Linear(latent_dim + 32 + 64, 64),
            nn.ReLU(),
            nn.Linear(64, 1),
        )

    def encode(self, obs: dict[str, torch.Tensor]) -> EncodedState:
        player_power_repr = self.player_power_encoder(obs["player_power_ids"], obs["player_power_numeric"])
        player_power_pool = _masked_mean(player_power_repr, obs["player_power_mask"])

        hand_repr = self.card_encoder(
            obs["hand_ids"],
            obs["hand_type_ids"],
            obs["hand_target_ids"],
            obs["hand_numeric"],
        )
        hand_pool = _masked_mean(hand_repr, obs["hand_mask"])

        potion_repr = self.potion_encoder(
            obs["potion_ids"],
            obs["potion_target_ids"],
            obs["potion_numeric"],
        )
        potion_pool = _masked_mean(potion_repr, obs["potion_mask"])

        monster_power_repr = self.monster_power_encoder(obs["monster_power_ids"], obs["monster_power_numeric"])
        monster_power_pool = _masked_mean(monster_power_repr, obs["monster_power_mask"])
        monster_repr = self.monster_encoder(
            obs["monster_ids"],
            obs["monster_intent_ids"],
            obs["monster_numeric"],
            monster_power_pool,
        )
        monster_pool = _masked_mean(monster_repr, obs["monster_mask"])

        choice_repr = self.choice_encoder(
            obs["choice_card_ids"],
            obs["choice_source_ids"],
            obs["choice_numeric"],
        )
        choice_pool = _masked_mean(choice_repr, obs["choice_mask"])

        trunk_input = torch.cat(
            [
                obs["global_numeric"],
                player_power_pool,
                hand_pool,
                potion_pool,
                monster_pool,
                choice_pool,
            ],
            dim=-1,
        )
        trunk = self.trunk(trunk_input)
        tactical = self.tactical(trunk)
        action_type_logits = self.action_type_head(tactical)
        card_logits = self.card_head(
            torch.cat(
                [hand_repr, tactical.unsqueeze(1).expand(-1, MAX_HAND, -1)],
                dim=-1,
            )
        ).squeeze(-1)
        potion_logits = self.potion_head(
            torch.cat(
                [potion_repr, tactical.unsqueeze(1).expand(-1, MAX_POTIONS, -1)],
                dim=-1,
            )
        ).squeeze(-1)
        choice_logits = self.choice_head(
            torch.cat(
                [choice_repr, tactical.unsqueeze(1).expand(-1, MAX_CHOICE_OPTIONS, -1)],
                dim=-1,
            )
        ).squeeze(-1)
        return EncodedState(
            tactical=tactical,
            value=self.value_head(tactical).squeeze(-1),
            probes=self.probe_head(tactical),
            hand_repr=hand_repr,
            potion_repr=potion_repr,
            monster_repr=monster_repr,
            choice_repr=choice_repr,
            action_type_logits=action_type_logits,
            card_logits=card_logits,
            potion_logits=potion_logits,
            choice_logits=choice_logits,
        )

    def card_target_logits(self, state: EncodedState, card_repr: torch.Tensor) -> torch.Tensor:
        none_logit = self.none_target_head(torch.cat([state.tactical, card_repr], dim=-1))
        monster_logits = self.monster_target_head(
            torch.cat(
                [
                    state.monster_repr,
                    state.tactical.unsqueeze(1).expand(-1, MAX_MONSTERS, -1),
                    card_repr.unsqueeze(1).expand(-1, MAX_MONSTERS, -1),
                ],
                dim=-1,
            )
        ).squeeze(-1)
        return torch.cat([none_logit, monster_logits], dim=-1)

    def potion_target_logits(self, state: EncodedState, potion_repr: torch.Tensor) -> torch.Tensor:
        none_logit = self.none_potion_target_head(torch.cat([state.tactical, potion_repr], dim=-1))
        monster_logits = self.monster_potion_target_head(
            torch.cat(
                [
                    state.monster_repr,
                    state.tactical.unsqueeze(1).expand(-1, MAX_MONSTERS, -1),
                    potion_repr.unsqueeze(1).expand(-1, MAX_MONSTERS, -1),
                ],
                dim=-1,
            )
        ).squeeze(-1)
        return torch.cat([none_logit, monster_logits], dim=-1)


def to_device_obs(batch: dict[str, Any], device: torch.device) -> dict[str, torch.Tensor]:
    out: dict[str, torch.Tensor] = {}
    for key, value in batch.items():
        tensor = torch.as_tensor(value, device=device)
        if tensor.dtype in (torch.int8, torch.int16, torch.int32, torch.int64, torch.uint8):
            out[key] = tensor.long()
        else:
            out[key] = tensor.float()
    return out


def stack_obs(obs_list: list[dict[str, Any]]) -> dict[str, torch.Tensor]:
    return {key: torch.as_tensor([obs[key] for obs in obs_list]) for key in obs_list[0].keys()}


def _dist_from_logits(logits: torch.Tensor, mask: torch.Tensor) -> Categorical:
    return Categorical(logits=_masked_logits(logits, mask))


def sample_actions(
    model: StructuredPolicyNet,
    obs: dict[str, torch.Tensor],
    *,
    deterministic: bool = False,
) -> tuple[dict[str, torch.Tensor], torch.Tensor, torch.Tensor, torch.Tensor]:
    state = model.encode(obs)
    batch = state.tactical.shape[0]
    action_type_dist = _dist_from_logits(state.action_type_logits, obs["action_type_mask"])
    action_type = action_type_dist.probs.argmax(dim=-1) if deterministic else action_type_dist.sample()
    logprob = action_type_dist.log_prob(action_type)
    entropy = action_type_dist.entropy()

    card_slot = torch.zeros(batch, dtype=torch.long, device=state.tactical.device)
    target_slot = torch.zeros(batch, dtype=torch.long, device=state.tactical.device)
    potion_slot = torch.zeros(batch, dtype=torch.long, device=state.tactical.device)
    choice_index = torch.zeros(batch, dtype=torch.long, device=state.tactical.device)

    for index in range(batch):
        atype = int(action_type[index].item())
        if atype == ACTION_TYPE_PLAY_CARD:
            card_dist = _dist_from_logits(state.card_logits[index], obs["play_card_mask"][index])
            card_slot[index] = card_dist.probs.argmax() if deterministic else card_dist.sample()
            logprob[index] = logprob[index] + card_dist.log_prob(card_slot[index])
            entropy[index] = entropy[index] + card_dist.entropy()
            chosen_card = state.hand_repr[index, card_slot[index]].unsqueeze(0)
            target_logits = model.card_target_logits(
                EncodedState(
                    tactical=state.tactical[index : index + 1],
                    value=state.value[index : index + 1],
                    probes=state.probes[index : index + 1],
                    hand_repr=state.hand_repr[index : index + 1],
                    potion_repr=state.potion_repr[index : index + 1],
                    monster_repr=state.monster_repr[index : index + 1],
                    choice_repr=state.choice_repr[index : index + 1],
                    action_type_logits=state.action_type_logits[index : index + 1],
                    card_logits=state.card_logits[index : index + 1],
                    potion_logits=state.potion_logits[index : index + 1],
                    choice_logits=state.choice_logits[index : index + 1],
                ),
                chosen_card,
            ).squeeze(0)
            target_dist = _dist_from_logits(
                target_logits,
                obs["play_card_target_mask"][index, card_slot[index]],
            )
            target_slot[index] = target_dist.probs.argmax() if deterministic else target_dist.sample()
            logprob[index] = logprob[index] + target_dist.log_prob(target_slot[index])
            entropy[index] = entropy[index] + target_dist.entropy()
        elif atype == ACTION_TYPE_USE_POTION:
            potion_dist = _dist_from_logits(state.potion_logits[index], obs["use_potion_mask"][index])
            potion_slot[index] = potion_dist.probs.argmax() if deterministic else potion_dist.sample()
            logprob[index] = logprob[index] + potion_dist.log_prob(potion_slot[index])
            entropy[index] = entropy[index] + potion_dist.entropy()
            chosen_potion = state.potion_repr[index, potion_slot[index]].unsqueeze(0)
            target_logits = model.potion_target_logits(
                EncodedState(
                    tactical=state.tactical[index : index + 1],
                    value=state.value[index : index + 1],
                    probes=state.probes[index : index + 1],
                    hand_repr=state.hand_repr[index : index + 1],
                    potion_repr=state.potion_repr[index : index + 1],
                    monster_repr=state.monster_repr[index : index + 1],
                    choice_repr=state.choice_repr[index : index + 1],
                    action_type_logits=state.action_type_logits[index : index + 1],
                    card_logits=state.card_logits[index : index + 1],
                    potion_logits=state.potion_logits[index : index + 1],
                    choice_logits=state.choice_logits[index : index + 1],
                ),
                chosen_potion,
            ).squeeze(0)
            target_dist = _dist_from_logits(
                target_logits,
                obs["use_potion_target_mask"][index, potion_slot[index]],
            )
            target_slot[index] = target_dist.probs.argmax() if deterministic else target_dist.sample()
            logprob[index] = logprob[index] + target_dist.log_prob(target_slot[index])
            entropy[index] = entropy[index] + target_dist.entropy()
        elif atype == ACTION_TYPE_CHOICE:
            choice_dist = _dist_from_logits(state.choice_logits[index], obs["choice_option_mask"][index])
            choice_index[index] = choice_dist.probs.argmax() if deterministic else choice_dist.sample()
            logprob[index] = logprob[index] + choice_dist.log_prob(choice_index[index])
            entropy[index] = entropy[index] + choice_dist.entropy()

    actions = {
        "action_type": action_type,
        "card_slot": card_slot,
        "target_slot": target_slot,
        "potion_slot": potion_slot,
        "choice_index": choice_index,
    }
    return actions, logprob, state.value, state.probes


def evaluate_actions(
    model: StructuredPolicyNet,
    obs: dict[str, torch.Tensor],
    actions: dict[str, torch.Tensor],
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    state = model.encode(obs)
    batch = state.tactical.shape[0]
    logprob = torch.zeros(batch, dtype=torch.float32, device=state.tactical.device)
    entropy = torch.zeros(batch, dtype=torch.float32, device=state.tactical.device)

    action_type_dist = _dist_from_logits(state.action_type_logits, obs["action_type_mask"])
    action_type = actions["action_type"].long()
    logprob = logprob + action_type_dist.log_prob(action_type)
    entropy = entropy + action_type_dist.entropy()

    for index in range(batch):
        atype = int(action_type[index].item())
        if atype == ACTION_TYPE_PLAY_CARD:
            card_dist = _dist_from_logits(state.card_logits[index], obs["play_card_mask"][index])
            card_choice = actions["card_slot"][index].long()
            logprob[index] = logprob[index] + card_dist.log_prob(card_choice)
            entropy[index] = entropy[index] + card_dist.entropy()
            chosen_card = state.hand_repr[index, card_choice].unsqueeze(0)
            target_logits = model.card_target_logits(
                EncodedState(
                    tactical=state.tactical[index : index + 1],
                    value=state.value[index : index + 1],
                    probes=state.probes[index : index + 1],
                    hand_repr=state.hand_repr[index : index + 1],
                    potion_repr=state.potion_repr[index : index + 1],
                    monster_repr=state.monster_repr[index : index + 1],
                    choice_repr=state.choice_repr[index : index + 1],
                    action_type_logits=state.action_type_logits[index : index + 1],
                    card_logits=state.card_logits[index : index + 1],
                    potion_logits=state.potion_logits[index : index + 1],
                    choice_logits=state.choice_logits[index : index + 1],
                ),
                chosen_card,
            ).squeeze(0)
            target_dist = _dist_from_logits(target_logits, obs["play_card_target_mask"][index, card_choice])
            logprob[index] = logprob[index] + target_dist.log_prob(actions["target_slot"][index].long())
            entropy[index] = entropy[index] + target_dist.entropy()
        elif atype == ACTION_TYPE_USE_POTION:
            potion_dist = _dist_from_logits(state.potion_logits[index], obs["use_potion_mask"][index])
            potion_choice = actions["potion_slot"][index].long()
            logprob[index] = logprob[index] + potion_dist.log_prob(potion_choice)
            entropy[index] = entropy[index] + potion_dist.entropy()
            chosen_potion = state.potion_repr[index, potion_choice].unsqueeze(0)
            target_logits = model.potion_target_logits(
                EncodedState(
                    tactical=state.tactical[index : index + 1],
                    value=state.value[index : index + 1],
                    probes=state.probes[index : index + 1],
                    hand_repr=state.hand_repr[index : index + 1],
                    potion_repr=state.potion_repr[index : index + 1],
                    monster_repr=state.monster_repr[index : index + 1],
                    choice_repr=state.choice_repr[index : index + 1],
                    action_type_logits=state.action_type_logits[index : index + 1],
                    card_logits=state.card_logits[index : index + 1],
                    potion_logits=state.potion_logits[index : index + 1],
                    choice_logits=state.choice_logits[index : index + 1],
                ),
                chosen_potion,
            ).squeeze(0)
            target_dist = _dist_from_logits(target_logits, obs["use_potion_target_mask"][index, potion_choice])
            logprob[index] = logprob[index] + target_dist.log_prob(actions["target_slot"][index].long())
            entropy[index] = entropy[index] + target_dist.entropy()
        elif atype == ACTION_TYPE_CHOICE:
            choice_dist = _dist_from_logits(state.choice_logits[index], obs["choice_option_mask"][index])
            logprob[index] = logprob[index] + choice_dist.log_prob(actions["choice_index"][index].long())
            entropy[index] = entropy[index] + choice_dist.entropy()

    return logprob, entropy, state.value, state.probes

