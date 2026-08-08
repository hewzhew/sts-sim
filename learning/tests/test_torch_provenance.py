from __future__ import annotations

import importlib.util
import unittest
from dataclasses import replace

from learning.tests.semantic_fixtures import semantic_schema_fixture


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
        categorical_training_manifest_template,
    )


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class TorchProvenanceTests(unittest.TestCase):
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

        template = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            optimizer,
            device_type="cpu",
        )
        reordered = categorical_training_manifest_template(
            reversed_schema,
            scorer,
            behavior,
            optimizer,
            device_type="cpu",
        )
        changed_model = categorical_training_manifest_template(
            schema,
            replace(scorer, hidden_dim=8),
            behavior,
            optimizer,
            device_type="cpu",
        )
        changed_optimizer = categorical_training_manifest_template(
            schema,
            scorer,
            behavior,
            replace(optimizer, learning_rate=0.003),
            device_type="cpu",
        )

        self.assertEqual(reordered, template)
        self.assertNotEqual(changed_model.model_config, template.model_config)
        self.assertNotEqual(
            changed_optimizer.optimizer_config,
            template.optimizer_config,
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
                device_type="cpu",
            )


if __name__ == "__main__":
    unittest.main()
