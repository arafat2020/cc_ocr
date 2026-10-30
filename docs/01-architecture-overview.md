# 01 — Architecture overview

## Purpose

`cc_ocr` converts **image-based chat exports** (PDF/PNG) into structured messages and intent labels. The codebase is split so that:

1. The **core engine** never hardcodes a marketplace UI.
2. New “situations” (Fiverr, generic chat, future Upwork, etc.) are **plugins + TOML**, not forks of `pipeline`/`engine`.
3. OCR and classifiers can be swapped the same way.

## Layered design

```text
┌─────────────────────────────────────────────────────────┐
│  cc_ocr_cli          CLI + compile-time plugin registry │
├─────────────────────────────────────────────────────────┤
│  plugins/*           Situation / OCR / Classifier impls │
├─────────────────────────────────────────────────────────┤
│  cc_ocr_core         Engine, render, preprocess, cache  │
├─────────────────────────────────────────────────────────┤
│  cc_ocr_plugin_api   Types + traits (shared contracts)  │
└─────────────────────────────────────────────────────────┘
         │                         │
         ▼                         ▼
   Tesseract (system)      python/ (render + serve)
```

### Dependency rules (do not reverse)

| Crate | May depend on | Must not depend on |
|-------|---------------|--------------------|
| `cc_ocr_plugin_api` | `anyhow`, `serde` only | core, cli, plugins |
| `cc_ocr_core` | `plugin_api` | cli, specific plugins |
| `plugins/*` | `plugin_api` (+ tool crates) | core, cli, other plugins\* |
| `cc_ocr_cli` | core + **all** plugins it registers | — |

\*Plugins should stay independent of each other so situations can be composed freely.

## Runtime object graph

```text
SituationProfile (TOML)
        │
        ▼
 registry::build_engine()
        │
        ▼
 Engine {
   situation:  Arc<dyn SituationAdapter>,
   ocr:        Arc<dyn OcrBackend>,
   classifier: Arc<dyn ClassifierBackend>,
   opts:       EngineOpts { dpi, force, keep_tmp }
 }
```

The engine calls **traits**, never concrete Fiverr/Tesseract types. Concrete types are chosen only in [`crates/cc_ocr_cli/src/registry.rs`](../crates/cc_ocr_cli/src/registry.rs).

## What belongs where

| Concern | Put it in |
|---------|-----------|
| Message JSON fields, trait methods | `cc_ocr_plugin_api` |
| PDF→PNG, image crop helper, cache hash, `Engine::run_*` | `cc_ocr_core` |
| Fiverr chrome regexes, crop fractions, risk keywords | `plugins/fiverr` |
| Tesseract CLI / tiling | `plugins/tesseract_ocr` |
| Calling `serve.py` | `plugins/tf_sidecar` |
| `--situation`, clap, matching TOML ids → structs | `cc_ocr_cli` |
| Operator docs (how to run) | `MANUAL.md` |
| Modifier docs (how to change code) | `docs/` (this folder) |

## Design defaults (locked for v1)

- Plugins are **compile-time** (no `dlopen`).
- Profiles are **TOML** under `situations/`.
- PDF render uses **PyMuPDF** via Python helper (core `render` module).
- Default OCR is **Tesseract**; default classifier is **Python sidecar** (heuristic until a TF model exists).

## Related docs

- [02 — Crate & file map](02-crate-and-file-map.md)
- [03 — Data flow & types](03-data-flow-and-types.md)
- [04 — Plugin system](04-plugin-system.md)
