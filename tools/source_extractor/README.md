# Source Extractor: Java to Rust Translation Toolchain

This directory contains the legacy wide-surface static analysis toolchain used to extract large human-readable reports from the Slay the Spire Java codebase (`d:\rust\cardcrawl`) into the Rust simulator environment.

It remains useful, but it is no longer the recommended primary extraction entrypoint. The current structured-first workflow is:

```powershell
python -m sts_tool cache
python -m sts_tool family exhaust
python tools/hook_query.py onApplyPower
```

Those commands emit JSON-first artifacts under `tools/analysis_cache/`, with Markdown as a renderer layer.

This legacy extractor now also emits structured sidecar JSON for its two most critical report families:
- `hooks.json`
- `scattered_logic.json`

Validation is split by cost:

```powershell
python -m analysis.quick_smoke
python -m analysis.full_smoke
```

## 1. Overview

The Slay the Spire codebase contains extensive "Scattered Logic"—mechanics for relics, powers, and events that do not reside solely within their respective object definitions. Instead, they are deeply hardcoded as engine-side conditionals (e.g., `if (AbstractDungeon.player.hasRelic("Torii"))`). 

This toolchain programmatically parses the raw Java AST (Abstract Syntax Tree) and regex patterns to extract these hidden dependencies into structured, human-and-AI-readable Markdown reports.

## 2. Core Components

- **`sts_extractor.py`**: The primary Python parsing engine. Scans the decompiled Java source to identify class boundaries, method behaviors, and dispersed engine-side hooks.
- **`dep_graph.json` / `dep_graph_summary.md`**: Legacy dependency outputs still present in `output/`. The historical `dep_graph.py` entry referenced by older docs is no longer part of the recommended workflow.
- **`AGENT_GUIDE.md`**: The internal operating protocol manual for AI Agents. It now points agents to the structured JSON truth layer first, with Markdown reports used mainly for human browsing.

## 3. Output Artifacts

Running the extraction pipeline generates critical reference sheets into the `output/` directory:

- **`scattered_logic.md`**: Exhaustive index mapping exactly where Relics and Powers hook into the global Java engine.
- **`taint_report.md`**: Identifies whether an action `update()` method is purely logical (`[LOGIC]`), purely visual (`[PRES]`), or mixed. Critical for filtering out animations to build a headless simulator.
- **`damage_pipeline.md`**: The exact, ordered chain of damage modifiers and rounding logic required for achieving bit-for-bit parity.
- **`hooks.md` / `powers.md` / `relics.md` / `cards.md`**: Human-readable summaries of implementations broken down by game entity categories.
- **`hooks.json` / `relics.json` / `scattered_logic.json`**: Structured sidecar facts for downstream tools that should not parse Markdown directly.

## 4. Execution Pipeline & Usage

This toolkit operates independently of the `sts_simulator` Cargo workspace and has zero impact on Rust compilation or static analysis. Use it when you want broad human-readable reports over the Java source tree.

### Environment Setup
Requires Python 3.9+ and the Tree-sitter AST parsing library.
```bash
pip install tree-sitter tree-sitter-java
```

### Standard Execution Pipeline

**Step 1: Extract AST and facts**
Scan the decompiled Slay the Spire source code (`d:\rust\cardcrawl`) to generate structured JSON facts plus human-readable reports.
```bash
python sts_extractor.py d:\rust\cardcrawl .\output
```

### Rust Porting Strategy
Strictly adhere to the generated structured facts—especially `scattered_logic.json`, `relics.json`, `hooks.json`, and `taint_report.md`—when authoring Rust implementations. Defining logic based on this structural blueprint eliminates hallucination and drastically minimizes trial-and-error debugging against the `diff_driver`.
