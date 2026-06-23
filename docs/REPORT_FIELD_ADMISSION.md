# Report Field Admission

Reports, journals, and learning samples are interfaces. A quick field can become
an accidental policy surface, so new fields need an explicit admission reason.

## Field Classes

Every new output field should be one of these:

- `fact`: raw state or candidate data, such as floor, HP, deck count, action id,
  route node, relic, or card choice.
- `diagnostic`: an intermediate view used for debugging a model or scheduler.
- `verdict`: an explicit conclusion with a named evaluator and evidence limits.
- `label`: a training or evaluation target with a documented data source.

If a field does not fit one of those classes, do not add it yet.

## Admission Rules

- Name the question the field answers before adding it.
- Do not present diagnostic extremes as winners. Examples: `furthest`,
  `best_hp`, and `cleanest` are facts or diagnostics, not route quality.
- If the evidence is insufficient, emit `verdict=insufficient_evidence` instead
  of inventing a winner-like field.
- Prefer candidate fact rows over summary winners when the comparison question is
  still unclear.
- Put unstable model details behind an explicit diagnostic section, not in the
  default headline.
- Before adding a default field, write one counterexample where it would mislead
  a human or downstream learner.

## Practical Checklist

- Is this field a fact, diagnostic, verdict, or label?
- What decision or investigation will read it?
- Can it be mistaken for a strategy recommendation?
- Does it duplicate an existing field with a different name?
- Should this live in a focused inspect command instead of the default report?

