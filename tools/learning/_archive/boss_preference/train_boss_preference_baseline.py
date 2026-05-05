#!/usr/bin/env python3
import argparse
import json
import math
import random
from collections import Counter, defaultdict
from pathlib import Path


LABELS = ["prefer_a", "prefer_b", "close_enough"]


def read_jsonl(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def feature_vocab(rows):
    vocab = {}
    for row in rows:
        for key in row["features"]:
            if key not in vocab:
                vocab[key] = len(vocab)
    return vocab


def vectorize(rows, vocab):
    vectors = []
    for row in rows:
        vec = [0.0] * len(vocab)
        for key, value in row["features"].items():
            vec[vocab[key]] = float(value)
        vectors.append(
            {
                "case_id": row["case_id"],
                "pack": row["validation_pack"],
                "label": row["label"],
                "vector": vec,
            }
        )
    return vectors


def standardize(train, test):
    dims = len(train[0]["vector"])
    means = [0.0] * dims
    stds = [1.0] * dims
    for i in range(dims):
        values = [row["vector"][i] for row in train]
        mean = sum(values) / len(values)
        var = sum((value - mean) ** 2 for value in values) / len(values)
        std = math.sqrt(var) if var > 1e-8 else 1.0
        means[i] = mean
        stds[i] = std
    for split in (train, test):
        for row in split:
            row["vector"] = [
                (value - means[i]) / stds[i] for i, value in enumerate(row["vector"])
            ]


def softmax(logits):
    max_logit = max(logits)
    exps = [math.exp(logit - max_logit) for logit in logits]
    total = sum(exps)
    return [value / total for value in exps]


def train_softmax(rows, epochs=400, lr=0.05, l2=1e-4):
    dims = len(rows[0]["vector"])
    weights = [[0.0] * dims for _ in LABELS]
    bias = [0.0] * len(LABELS)
    rng = random.Random(7)

    for _ in range(epochs):
        shuffled = rows[:]
        rng.shuffle(shuffled)
        for row in shuffled:
            logits = [
                sum(weight * value for weight, value in zip(weights[c], row["vector"])) + bias[c]
                for c in range(len(LABELS))
            ]
            probs = softmax(logits)
            true_class = LABELS.index(row["label"])
            for c in range(len(LABELS)):
                error = probs[c] - (1.0 if c == true_class else 0.0)
                for i, value in enumerate(row["vector"]):
                    weights[c][i] -= lr * (error * value + l2 * weights[c][i])
                bias[c] -= lr * error
    return weights, bias


def predict(weights, bias, row):
    logits = [
        sum(weight * value for weight, value in zip(weights[c], row["vector"])) + bias[c]
        for c in range(len(LABELS))
    ]
    probs = softmax(logits)
    best = max(range(len(LABELS)), key=lambda idx: probs[idx])
    return LABELS[best], probs


def accuracy(rows, weights, bias):
    correct = 0
    outputs = []
    for row in rows:
        pred, probs = predict(weights, bias, row)
        correct += int(pred == row["label"])
        outputs.append({"case_id": row["case_id"], "label": row["label"], "pred": pred, "probs": probs})
    return correct / len(rows), outputs


def confusion_matrix(outputs):
    matrix = {label: {pred: 0 for pred in LABELS} for label in LABELS}
    for row in outputs:
        matrix[row["label"]][row["pred"]] += 1
    return matrix


def dominant_features(weights, vocab, top_k=8):
    reverse_vocab = {index: name for name, index in vocab.items()}
    result = {}
    for class_index, label in enumerate(LABELS):
        ranked = sorted(
            enumerate(weights[class_index]),
            key=lambda item: abs(item[1]),
            reverse=True,
        )[:top_k]
        result[label] = [
            {"feature": reverse_vocab[index], "weight": round(weight, 4)}
            for index, weight in ranked
        ]
    return result


def split_leave_one_pack_out(rows, held_out_pack):
    train = [row for row in rows if row["pack"] != held_out_pack]
    test = [row for row in rows if row["pack"] == held_out_pack]
    return train, test


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset", required=True, type=Path)
    parser.add_argument("--report-out", required=True, type=Path)
    args = parser.parse_args()

    raw_rows = list(read_jsonl(args.dataset))
    vocab = feature_vocab(raw_rows)
    rows = vectorize(raw_rows, vocab)
    packs = sorted({row["pack"] for row in rows})
    if len(packs) < 2:
        raise ValueError("need at least two packs for leave-one-pack-out evaluation")

    pack_reports = []
    for held_out_pack in packs:
        train_rows, test_rows = split_leave_one_pack_out(rows, held_out_pack)
        if not train_rows or not test_rows:
            continue
        train_copy = [dict(row, vector=row["vector"][:]) for row in train_rows]
        test_copy = [dict(row, vector=row["vector"][:]) for row in test_rows]
        standardize(train_copy, test_copy)
        weights, bias = train_softmax(train_copy)
        train_acc, train_outputs = accuracy(train_copy, weights, bias)
        test_acc, test_outputs = accuracy(test_copy, weights, bias)
        pack_reports.append(
            {
                "held_out_pack": held_out_pack,
                "train_packs": sorted({row["pack"] for row in train_rows}),
                "train_accuracy": round(train_acc, 4),
                "test_accuracy": round(test_acc, 4),
                "train_outputs": train_outputs,
                "test_outputs": test_outputs,
                "train_confusion": confusion_matrix(train_outputs),
                "test_confusion": confusion_matrix(test_outputs),
                "dominant_features": dominant_features(weights, vocab),
            }
        )

    majority = Counter(row["label"] for row in raw_rows).most_common(1)[0][0]
    baseline = {
        "majority_label": majority,
        "counts": dict(Counter(row["label"] for row in raw_rows)),
    }

    args.report_out.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "dataset_size": len(raw_rows),
        "feature_count": len(vocab),
        "packs": packs,
        "majority_baseline": baseline,
        "leave_one_pack_out": pack_reports,
    }
    args.report_out.write_text(json.dumps(report, indent=2, ensure_ascii=True), encoding="utf-8")
    print(f"wrote baseline report to {args.report_out}")


if __name__ == "__main__":
    main()
