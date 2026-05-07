#!/usr/bin/env python3
"""Export a sklearn MLP verified proposer pickle to Rust-readable JSON."""
from __future__ import annotations

import argparse
import json
import pickle
from pathlib import Path
from typing import Any

import numpy as np


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    with args.input.open("rb") as handle:
        payload: dict[str, Any] = pickle.load(handle)
    model = payload.get("model")
    if model is None or not hasattr(model, "coefs_"):
        raise SystemExit("input does not contain a sklearn MLP model")
    if str(payload.get("feature_set")) != "candidate_only":
        raise SystemExit("Rust exporter currently supports feature_set=candidate_only only")
    coefs = [np.asarray(coef, dtype=np.float32) for coef in model.coefs_]
    intercepts = [np.asarray(bias, dtype=np.float32) for bias in model.intercepts_]
    if len(coefs) != 2 or len(intercepts) != 2:
        raise SystemExit(f"expected one hidden layer, got {len(coefs)} coefficient matrices")
    if coefs[1].shape[1] != 1:
        raise SystemExit(f"expected binary output shape (*, 1), got {coefs[1].shape}")
    out = {
        "schema_version": "verified_proposer_mlp_json_v0",
        "model_type": "verified_proposer_mlp_json_v0",
        "source_schema_version": payload.get("schema_version"),
        "source_model_type": payload.get("model_type"),
        "model_kind": payload.get("model_kind"),
        "feature_set": payload.get("feature_set"),
        "target_mode": payload.get("target_mode"),
        "feature_dim": int(payload.get("feature_dim") or coefs[0].shape[0]),
        "activation": str(getattr(model, "activation", "relu")),
        "out_activation": str(getattr(model, "out_activation_", "logistic")),
        "classes": [int(value) for value in getattr(model, "classes_", [0, 1])],
        "input_weights": coefs[0].tolist(),
        "hidden_bias": intercepts[0].tolist(),
        "output_weights": coefs[1][:, 0].tolist(),
        "output_bias": float(intercepts[1][0]),
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as handle:
        json.dump(out, handle, separators=(",", ":"))
        handle.write("\n")
    print(
        json.dumps(
            {
                "out": str(args.out),
                "feature_dim": out["feature_dim"],
                "hidden_dim": len(out["hidden_bias"]),
                "bytes": args.out.stat().st_size,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
