# Source Extractor: Java to Rust Translation Toolchain

This directory contains the custom static analysis toolchain designed to accelerate and strictly verify the porting of Slay the Spire's Java codebase (`d:\rust\cardcrawl`) into the Rust simulator environment.

## 1. Overview

The Slay the Spire codebase contains extensive "Scattered Logic"—mechanics for relics, powers, and events that do not reside solely within their respective object definitions. Instead, they are deeply hardcoded as engine-side conditionals (e.g., `if (AbstractDungeon.player.hasRelic("Torii"))`). 

This toolchain programmatically parses the raw Java AST (Abstract Syntax Tree) and regex patterns to extract these hidden dependencies into structured, human-and-AI-readable Markdown reports.

## 2. Core Components

- **`sts_extractor.py`**: The primary Python parsing engine. Scans the decompiled Java source to identify class boundaries, method behaviors, and dispersed engine-side hooks.
- **`dep_graph.py` & `dep_graph.json`**: Generates and maps out execution dependencies, providing the hierarchical structure for porting tasks (ensuring baseline engine hooks are built before their dependent relics).
- **`AGENT_GUIDE.md`**: The internal operating protocol manual for AI Agents. It strictly dictates *how* to use the extracted Markdown files (e.g., `scattered_logic.md`) to write Rust code with 100% logical completeness, eliminating hallucination or guesswork.

## 3. Output Artifacts

Running the extraction pipeline generates critical reference sheets into the `output/` directory:

- **`scattered_logic.md`**: Exhaustive index mapping exactly where Relics and Powers hook into the global Java engine.
- **`taint_report.md`**: Identifies whether an action `update()` method is purely logical (`[LOGIC]`), purely visual (`[PRES]`), or mixed. Critical for filtering out animations to build a headless simulator.
- **`damage_pipeline.md`**: The exact, ordered chain of damage modifiers and rounding logic required for achieving bit-for-bit parity.
- **`hooks.md` / `powers.md` / `relics.md` / `cards.md`**: Summaries of implementations broken down by game entity categories.

## 4. Execution Pipeline & Usage

This toolkit operates independently of the `sts_simulator` Cargo workspace and has zero impact on Rust compilation or static analysis. Follow these steps to extract updated Java data:

### Environment Setup
Requires Python 3.9+ and the Tree-sitter AST parsing library.
```bash
pip install tree-sitter tree-sitter-java
```

### Standard Execution Pipeline

**Step 1: Extract AST and Hooks**
Scan the decompiled Slay the Spire source code (`d:\rust\cardcrawl`) to generate the core Markdown reports.
```bash
python sts_extractor.py d:\rust\cardcrawl .\output
```

**Step 2: Generate Dependency Graphs**
Parse the extracted data to build porting priority mappings.
```bash
python dep_graph.py .\output
```

### Rust Porting Strategy
Strictly adhere to the generated output reports—especially `scattered_logic.md` and `taint_report.md`—when authoring Rust implementations. Defining logic based on this structural blueprint eliminates hallucination and drastically minimizes trial-and-error debugging against the `diff_driver`.

