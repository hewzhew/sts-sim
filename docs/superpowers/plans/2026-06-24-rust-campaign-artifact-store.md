# Rust Campaign ArtifactStore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move campaign artifact source/output/latest/scratch semantics into Rust as the first executable step of the Rust-owned campaign architecture.

**Architecture:** Add a focused Rust `campaign_artifact_store` module under `branch_campaign_driver`. It owns artifact path conventions, source selector resolution, latest/scratch pointer schemas, and output artifact path construction. Existing report/checkpoint JSON read/write stays in `campaign_artifacts.rs` for now; this first phase creates the authority boundary and tests it before migrating wrapper call sites.

**Tech Stack:** Rust, serde, existing `branch_campaign_driver` binary modules, Windows-compatible path handling.

---

### Task 1: Add Rust ArtifactStore Path And Selector Core

**Files:**
- Create: `src/bin/branch_campaign_driver/campaign_artifact_store.rs`
- Modify: `src/bin/branch_campaign_driver/main.rs` or module root that declares sibling modules

- [x] **Step 1: Write failing tests**

Add tests proving:

```rust
let store = CampaignArtifactStoreV1::new(repo.join("tools").join("artifacts").join("campaigns"));
assert_eq!(store.run_artifact_ref_v1("abc").report_path, root.join("runs").join("abc").join("campaign.json.gz"));
assert_eq!(store.scratch_artifact_ref_v1("probe").report_path, root.join("scratch").join("probe.campaign.json.gz"));
assert_eq!(store.resolve_source_selector_v1("run:abc").unwrap().label, "run:abc");
assert_eq!(store.resolve_source_selector_v1("scratch:probe").unwrap().label, "scratch:probe");
```

- [x] **Step 2: Run failing test**

Run:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
```

Expected: fail because the module does not exist.

- [x] **Step 3: Implement minimal module**

Define:

```rust
pub(super) struct CampaignArtifactStoreV1 { campaign_dir: PathBuf }
pub(super) struct CampaignArtifactRefV1 { kind, id, label, dir, report_path, state_path, journal_path, checkpoint_path, manifest_path, command_path, log_path }
pub(super) fn resolve_source_selector_v1(&self, selector: &str) -> Result<CampaignArtifactRefV1, String>
```

Support `run:<id>`, `scratch:<id>`, and `path:<path>` first.

- [x] **Step 4: Verify**

Run:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
cargo fmt --check
```

### Task 2: Add Latest And Scratch Pointer Read/Write

**Files:**
- Modify: `src/bin/branch_campaign_driver/campaign_artifact_store.rs`

- [x] **Step 1: Write failing tests**

Add tests proving:

```rust
store.write_latest_pointer_v1(&run_ref, fixed_time).unwrap();
assert_eq!(store.resolve_source_selector_v1("latest").unwrap().id, run_ref.id);
store.write_scratch_latest_pointer_v1(&scratch_ref, fixed_time).unwrap();
assert_eq!(store.resolve_source_selector_v1("scratch-latest").unwrap().id, scratch_ref.id);
```

- [x] **Step 2: Run failing test**

Run:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
```

- [x] **Step 3: Implement pointer schemas**

Define typed pointer structs:

```rust
CampaignLatestPointerV1
CampaignScratchLatestPointerV1
```

Read/write UTF-8 JSON with LF. Reject missing or wrong schema names with clear errors.

- [x] **Step 4: Verify**

Run the same test and `git diff --check`.

### Task 3: Add Output Allocation Without Wrapper Semantics

**Files:**
- Modify: `src/bin/branch_campaign_driver/campaign_artifact_store.rs`

- [x] **Step 1: Write failing tests**

Add tests proving:

```rust
let output = store.run_output_ref_v1("continue-seed521", "20260624-010203", "abcdef12");
assert!(output.id.starts_with("continue-seed521-20260624-010203-abcdef12"));
let scratch = store.scratch_output_ref_v1("gap probe", "20260624-010203", "abcdef12");
assert_eq!(scratch.kind, CampaignArtifactKindV1::Scratch);
```

- [x] **Step 2: Implement deterministic output constructors**

Do not use wall-clock or GUID directly inside tests. The caller passes stamp and suffix.

- [x] **Step 3: Verify**

Run:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
cargo check --bin branch_campaign_driver
```

### Task 4: Integration Decision

**Files:**
- Review: `src/bin/branch_campaign_driver/command_inputs.rs`
- Review: `src/bin/branch_campaign_driver/campaign_run.rs`
- Review: `tools/campaign_artifact_source.ps1`
- Review: `tools/campaign_manifest.ps1`

- [x] **Step 1: Identify first call-site migration**

Choose the smallest Rust-owned workflow that can use `CampaignArtifactStoreV1` without preserving wrapper semantics. Preferred target: a new direct Rust artifact resolve/list subcommand or replacing driver-side explicit `--resume` path plumbing.

- [x] **Step 2: Do not migrate PowerShell in this task**

PowerShell deletion waits until Rust has a full replacement command path.

Implemented call-site: `branch_campaign_driver artifact resolve|allocate|write-latest`.
This is intentionally a narrow Rust-owned artifact API, not a campaign execution path migration.
