# Campaign Wrapper Usage

`tools/campaign.ps1` is a compatibility launcher for the Rust campaign driver.
It exists for local Windows convenience while campaign ownership migrates into
Rust.

For target architecture, read:

- [Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md)
- [Campaign CLI Contract](CAMPAIGN_CLI_CONTRACT.md)
- [Campaign Migration Plan](CAMPAIGN_MIGRATION_PLAN.md)

## Current Safe Use

These commands are still useful as launcher conveniences:

```powershell
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Rounds 1
.\tools\campaign.ps1 -Inspect
.\tools\campaign.ps1 -Probe final-boss-combat
.\tools\campaign.ps1 -DryRun
```

Use `-DryRun` before commands that read one artifact and write another. Treat
the printed Rust driver command and artifact paths as the authority.

## Ownership Boundary

PowerShell may temporarily handle:

- choosing a build profile
- building the campaign driver
- forwarding convenience flags
- printing short preflight information

PowerShell must not gain new ownership of:

- source, latest, scratch, or output semantics
- milestone loops
- coverage-gap orchestration
- campaign manifest writing
- strategy, scheduling, or branch-selection logic
- report/checkpoint/journal parsing as workflow control

If a new workflow needs one of those behaviors, implement it in Rust first and
let PowerShell forward to it.

## Compatibility Rules

The wrapper still contains old switches and modules because they are being
migrated. Do not use compatibility names in new docs, tests, or scripts.

Avoid:

```text
-More
-InspectScratchLatest
-FromScratchLatest as a semantic shortcut
-MaxRounds as continuation shorthand
latest.campaign.json / latest.checkpoint.json as default source
```

Preferred Rust-owned shapes are documented in
[Campaign CLI Contract](CAMPAIGN_CLI_CONTRACT.md). Until those shapes are fully
implemented, wrapper commands should be treated as transitional conveniences,
not as the architecture.

## Adding Wrapper Code

Before adding or changing wrapper code, answer:

1. Is this only launch/build/argument forwarding?
2. Could this change be expressed as a typed Rust campaign command instead?
3. Does this require reading or writing report/checkpoint/journal content?
4. Does this decide how continuation, coverage, or milestone execution works?

If the answer to 3 or 4 is yes, the change belongs in Rust, not PowerShell.

## Retirement Target

The wrapper is acceptable when it is small enough to audit as:

```text
parse convenience flags
build or locate branch_campaign_driver
invoke branch_campaign_driver campaign ...
print returned artifact paths
```

All other behavior is migration debt.
