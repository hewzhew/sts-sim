from __future__ import annotations

import unittest

import numpy as np

from learning.tests.policy_fixtures import BEHAVIOR_MANIFEST_ID
from sts_learning import BatchPolicyChoice, BehaviorManifestId, PolicyChoiceError


class PolicyChoiceTests(unittest.TestCase):
    def test_manifest_identity_requires_one_immutable_sha256_digest(self) -> None:
        with self.assertRaisesRegex(PolicyChoiceError, "immutable bytes"):
            BehaviorManifestId(bytearray(32))  # type: ignore[arg-type]
        with self.assertRaisesRegex(PolicyChoiceError, "32 bytes"):
            BehaviorManifestId(b"short")

    def test_batch_choice_normalizes_integer_ordinals_and_preserves_identity(self) -> None:
        choice = BatchPolicyChoice.create(
            [np.uint64(2), 0],
            BEHAVIOR_MANIFEST_ID,
        )

        self.assertEqual(choice.ordinals, (2, 0))
        self.assertEqual(choice.behavior_manifest_id, BEHAVIOR_MANIFEST_ID)

    def test_batch_choice_rejects_non_integer_ordinals(self) -> None:
        with self.assertRaisesRegex(PolicyChoiceError, "ordinal must be an integer"):
            BatchPolicyChoice.create([0.5], BEHAVIOR_MANIFEST_ID)  # type: ignore[list-item]


if __name__ == "__main__":
    unittest.main()
