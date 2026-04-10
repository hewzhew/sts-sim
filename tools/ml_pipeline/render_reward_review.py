import argparse
import json
from collections import Counter
from pathlib import Path
from typing import List


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render reward ranker review jsonl + metrics into a readable markdown report."
    )
    parser.add_argument(
        "--review",
        default=r"D:\rust\sts_simulator\data\reward_ranker_review.jsonl",
        help="Path to reward_ranker_review.jsonl",
    )
    parser.add_argument(
        "--metrics",
        default=r"D:\rust\sts_simulator\data\reward_ranker_metrics.json",
        help="Path to reward_ranker_metrics.json",
    )
    parser.add_argument(
        "--out",
        default=r"D:\rust\sts_simulator\data\reward_ranker_review.md",
        help="Output markdown path",
    )
    parser.add_argument(
        "--top-wrong",
        type=int,
        default=12,
        help="How many wrong examples to render",
    )
    return parser.parse_args()


def load_json(path: str):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def load_jsonl(path: str) -> List[dict]:
    rows = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def score_margin(record: dict) -> float:
    candidates = record.get("candidates", [])
    if len(candidates) < 2:
        return 0.0
    return float(candidates[0]["score"]) - float(candidates[1]["score"])


def render_metrics(metrics: dict) -> List[str]:
    lines = [
        "# Reward Ranker Review",
        "",
        "## Metrics",
        "",
        f"- model: `{metrics.get('model_kind')}`",
        f"- class: `{metrics.get('class_filter')}`",
        f"- rows: `{metrics.get('num_rows')}`",
        f"- samples: `{metrics.get('num_samples')}`",
        f"- features: `{metrics.get('num_features')}`",
        f"- disagreement weighting: `{metrics.get('weight_disagreements')}`",
        f"- train top-1: `{metrics.get('train_top1_accuracy')}`",
        f"- val top-1: `{metrics.get('val_top1_accuracy')}`",
        f"- disagreement samples: `{metrics.get('disagreement_samples')}`",
        "",
        "## Top Features",
        "",
    ]
    for feat in metrics.get("top_features", []):
        lines.append(f"- `{feat['feature']}`: `{feat['importance']}`")
    lines.append("")
    return lines


def render_summary(records: List[dict]) -> List[str]:
    split_counter = Counter(record.get("split", "unknown") for record in records)
    source_counter = Counter(record.get("source", "unknown") for record in records)
    correct_counter = Counter("correct" if record.get("correct") else "wrong" for record in records)
    wrong_cards = Counter(
        record.get("actual_card_id", "unknown")
        for record in records
        if not record.get("correct")
    )

    lines = [
        "## Review Summary",
        "",
        f"- total review samples: `{len(records)}`",
        f"- correct: `{correct_counter.get('correct', 0)}`",
        f"- wrong: `{correct_counter.get('wrong', 0)}`",
        "",
        "### Split Counts",
        "",
    ]
    for split, count in split_counter.items():
        lines.append(f"- `{split}`: `{count}`")
    lines.extend(["", "### Source Counts", ""])
    for source, count in source_counter.items():
        lines.append(f"- `{source}`: `{count}`")
    lines.extend(["", "### Most Missed Actual Choices", ""])
    for card_id, count in wrong_cards.most_common(10):
        lines.append(f"- `{card_id}`: `{count}`")
    lines.append("")
    return lines


def render_wrong_examples(records: List[dict], limit: int) -> List[str]:
    wrong = [record for record in records if not record.get("correct")]
    wrong.sort(key=score_margin, reverse=True)

    lines = ["## Wrong Examples", ""]
    for record in wrong[:limit]:
        margin = score_margin(record)
        lines.append(f"### `{record['sample_id']}`")
        lines.append("")
        lines.append(f"- split: `{record.get('split')}`")
        lines.append(f"- source: `{record.get('source')}`")
        lines.append(f"- predicted: `{record.get('predicted_card_id')}`")
        lines.append(f"- actual: `{record.get('actual_card_id')}`")
        lines.append(f"- top score margin: `{margin:.6f}`")
        lines.append("")
        lines.append("| card | label | score |")
        lines.append("| --- | --- | --- |")
        for cand in record.get("candidates", []):
            lines.append(
                f"| `{cand['card_id']}` | `{cand['label']}` | `{float(cand['score']):.6f}` |"
            )
        lines.append("")
    return lines


def ensure_parent(path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)


def main() -> None:
    args = parse_args()
    metrics = load_json(args.metrics)
    records = load_jsonl(args.review)

    lines: List[str] = []
    lines.extend(render_metrics(metrics))
    lines.extend(render_summary(records))
    lines.extend(render_wrong_examples(records, args.top_wrong))

    ensure_parent(args.out)
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")

    print(f"wrote reward review report to {args.out}")


if __name__ == "__main__":
    main()
