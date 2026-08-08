from __future__ import annotations

import importlib.util
import unittest
from dataclasses import replace

from learning.tests.semantic_fixtures import semantic_schema_fixture
from sts_learning import (
    CombatAllWinAxis,
    CombatWinObjectiveConfig,
    FloorProgressReturnConfig,
    OnPolicyObjectiveConfig,
    RunDecisionScope,
    TerminalAdvantageMode,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None

if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_policy import (
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )
    from sts_learning.torch_provenance import (
        AdamTrainingConfig,
        TorchProvenanceError,
        combat_win_training_manifest_template,
        combat_win_trainer_implementation,
        categorical_training_manifest_template,
        categorical_trainer_implementation,
    )


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class TorchProvenanceTests(unittest.TestCase):
    def test_combat_win_trainer_has_distinct_exact_provenance(self) -> None:
        schema = semantic_schema_fixture()
        scorer = RaggedScorerConfig(hidden_dim=4, relation_layers=1)
        behavior = RaggedCategoricalPolicyConfig(temperature=0.8)
        optimizer = AdamTrainingConfig(learning_rate=0.002)
        combat = CombatWinObjectiveConfig(groups_per_update=1)
        terminal = OnPolicyObjectiveConfig(
            terminal_return=FloorProgressReturnConfig(target_floor=52),
            attempts_per_update=1,
        )

        combat_template = combat_win_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            combat,
            device_type="cpu",
        )
        wider = combat_win_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            CombatWinObjectiveConfig(groups_per_update=2),
            device_type="cpu",
        )
        win_only = combat_win_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            CombatWinObjectiveConfig(all_win_axis=CombatAllWinAxis.NONE),
            device_type="cpu",
        )
        terminal_template = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            terminal,
            device_type="cpu",
        )

        self.assertEqual(
            combat_template.model_definition,
            terminal_template.model_definition,
        )
        self.assertEqual(combat_template.model_config, terminal_template.model_config)
        self.assertEqual(combat_template.behavior_rule, terminal_template.behavior_rule)
        self.assertEqual(
            combat_template.semantic_schema,
            terminal_template.semantic_schema,
        )
        self.assertEqual(
            combat_template.optimizer_config,
            terminal_template.optimizer_config,
        )
        self.assertEqual(
            combat_template.trainer_implementation,
            combat_win_trainer_implementation(combat),
        )
        self.assertEqual(
            terminal_template.trainer_implementation,
            categorical_trainer_implementation(terminal),
        )
        self.assertNotEqual(
            combat_template.trainer_implementation,
            terminal_template.trainer_implementation,
        )
        self.assertNotEqual(
            wider.trainer_implementation,
            combat_template.trainer_implementation,
        )
        self.assertNotEqual(
            win_only.trainer_implementation,
            combat_template.trainer_implementation,
        )

    def test_template_is_canonical_and_changes_with_runtime_profile(self) -> None:
        schema = semantic_schema_fixture()
        reversed_schema = {
            key: (
                dict(reversed(tuple(value.items())))
                if isinstance(value, dict)
                else value
            )
            for key, value in reversed(tuple(schema.items()))
        }
        scorer = RaggedScorerConfig(hidden_dim=4, relation_layers=0)
        behavior = RaggedCategoricalPolicyConfig(temperature=0.8)
        optimizer = AdamTrainingConfig(learning_rate=0.002)
        terminal_return = FloorProgressReturnConfig(target_floor=52)
        objective = OnPolicyObjectiveConfig(
            terminal_return=terminal_return,
            attempts_per_update=8,
        )

        template = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            objective,
            device_type="cpu",
        )
        reordered = categorical_training_manifest_template(
            reversed_schema,
            scorer,
            behavior,
            optimizer,
            objective,
            device_type="cpu",
        )
        changed_model = categorical_training_manifest_template(
            schema,
            replace(scorer, hidden_dim=8),
            behavior,
            optimizer,
            objective,
            device_type="cpu",
        )
        changed_optimizer = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            replace(optimizer, learning_rate=0.003),
            objective,
            device_type="cpu",
        )
        changed_return = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            replace(
                objective,
                terminal_return=replace(terminal_return, target_floor=51),
            ),
            device_type="cpu",
        )
        changed_attempt_batch = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            replace(objective, attempts_per_update=4),
            device_type="cpu",
        )
        changed_advantage = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            replace(
                objective,
                advantage_mode=TerminalAdvantageMode.LEAVE_ONE_OUT,
            ),
            device_type="cpu",
        )
        changed_scope = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            replace(
                objective,
                decision_scope=RunDecisionScope.STRATEGIC,
            ),
            device_type="cpu",
        )

        self.assertEqual(reordered, template)
        self.assertNotEqual(changed_model.model_config, template.model_config)
        self.assertNotEqual(
            changed_optimizer.optimizer_config,
            template.optimizer_config,
        )
        self.assertNotEqual(
            changed_return.trainer_implementation,
            template.trainer_implementation,
        )
        self.assertNotEqual(
            changed_attempt_batch.trainer_implementation,
            template.trainer_implementation,
        )
        self.assertNotEqual(
            changed_advantage.trainer_implementation,
            template.trainer_implementation,
        )
        self.assertNotEqual(
            changed_scope.trainer_implementation,
            template.trainer_implementation,
        )

    def test_adam_factory_matches_the_provenance_configuration(self) -> None:
        config = AdamTrainingConfig(
            learning_rate=0.002,
            beta1=0.8,
            beta2=0.95,
            epsilon=1e-7,
            weight_decay=0.01,
            amsgrad=True,
        )
        parameter = torch.nn.Parameter(torch.tensor([1.0]))
        optimizer = config.create([parameter])
        group = optimizer.param_groups[0]

        self.assertEqual(group["lr"], config.learning_rate)
        self.assertEqual(group["betas"], (config.beta1, config.beta2))
        self.assertEqual(group["eps"], config.epsilon)
        self.assertEqual(group["weight_decay"], config.weight_decay)
        self.assertTrue(group["amsgrad"])
        self.assertFalse(group["foreach"])
        self.assertFalse(group["fused"])

    def test_unsupported_schema_values_fail_closed(self) -> None:
        schema = semantic_schema_fixture()
        schema["foreign"] = [1, 2, 3]

        with self.assertRaisesRegex(TorchProvenanceError, "unsupported list"):
            categorical_training_manifest_template(
                schema,
                RaggedScorerConfig(hidden_dim=4, relation_layers=0),
                RaggedCategoricalPolicyConfig(temperature=0.8),
                AdamTrainingConfig(),
                OnPolicyObjectiveConfig(),
                device_type="cpu",
            )


if __name__ == "__main__":
    unittest.main()
