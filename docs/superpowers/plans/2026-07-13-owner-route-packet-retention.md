# Owner route packet retention implementation plan

1. Add a failing trace-format test whose route auto step carries a typed map packet and must serialize it.
2. Add optional packet retention to auto-applied steps and extract it from existing route annotations.
3. Run focused route/run-control tests and commit.
4. Rerun seed `20260713002` with JSONL tracing, compare the Act 3 candidate pools, then make only the behavior change supported by that evidence.
