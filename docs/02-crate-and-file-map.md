# 02 — Crate & file map

Use this when you need to know **which file to open**.

## Workspace root

| Path | Role |
|------|------|
| [`Cargo.toml`](../Cargo.toml) | Workspace members + shared dependency versions |
| [`situations/*.toml`](../situations/) | Plug-and-play profiles (operator-facing) |
| [`python/`](../python/) | PDF render helper + classifier sidecar |
| [`Chat/`](../Chat/) | Sample Fiverr exports (input data, not library code) |
| [`out/`](../out/) | Generated cache/labels/tmp (gitignored) |
| [`MANUAL.md`](../MANUAL.md) | End-user manual |
| [`ARCHITECTURE.md`](../ARCHITECTURE.md) | Short design summary |

---

## `crates/cc_ocr_plugin_api`

**Shared contracts.** Change only when you intentionally evolve the plugin ABI.

| File | Contents |
|------|----------|
| `src/lib.rs` | Re-exports |
| `src/types.rs` | `OcrLine`, `Message`, `LabeledMessage`, `CachedConversation`, `PipelineResult`, `ConversationRollup` |
| `src/page.rs` | `PageImage` (path + dimensions) |
| `src/preprocess.rs` | `PreprocessConfig` (crop / contrast knobs) |
| `src/risk.rs` | `RiskPolicy` (intents + keywords for rollup) |
| `src/traits.rs` | `SituationAdapter`, `OcrBackend`, `ClassifierBackend` |

---

## `crates/cc_ocr_core`

**Engine and I/O.** No marketplace-specific regexes here.

| File | Contents |
|------|----------|
| `src/lib.rs` | Module exports |
| `src/engine.rs` | `Engine`, `EngineOpts`, `run_ocr` / `run_classify` / `run_full` / `run_rollup` |
| `src/render.rs` | PDF/PNG → `page_*.png` under tmp |
| `src/preprocess.rs` | Apply `PreprocessConfig` to a `PageImage` |
| `src/cache.rs` | Cache key, load/save JSON under `out/` |
| `src/profile.rs` | Deserialize situation TOML → `SituationProfile` |
| `src/rollup.rs` | Intent histogram + risk flags from `RiskPolicy` |

---

## `crates/cc_ocr_cli`

**Binary entrypoint.** This is the only place that names concrete plugins.

| File | Contents |
|------|----------|
| `src/main.rs` | Clap CLI; load profile; call engine |
| `src/registry.rs` | `situation` / `ocr` / `classifier` string → `Arc<dyn …>` |

Binary name: **`cc_ocr`** (`[[bin]]` in this crate’s `Cargo.toml`).

---

## `plugins/`

| Crate | Implements | Notes |
|-------|------------|--------|
| `plugins/fiverr` | `SituationAdapter` | Crop sidebars, Fiverr chrome filter, buyer/seller mapping |
| `plugins/generic` | `SituationAdapter` | No crop, weak headers, `self`/`other` speakers |
| `plugins/tesseract_ocr` | `OcrBackend` | System `tesseract` CLI, TSV parse, vertical tiling |
| `plugins/tf_sidecar` | `ClassifierBackend` | Spawns `python/classifier/serve.py` |

Each plugin is a normal Cargo package with `src/lib.rs` as the main surface.

---

## `python/`

| File | Called by | Purpose |
|------|-----------|---------|
| `python/render_pages.py` | `cc_ocr_core::render` | PDF → PNG at DPI |
| `python/classifier/serve.py` | `tf_sidecar` | stdin JSON → intent labels |
| `python/classifier/train.py` | humans | Training stub for future TF models |
| `python/classifier/requirements.txt` | humans | Optional TF pin |

---

## Where to edit for common tasks

| Task | Start here |
|------|------------|
| Fix Fiverr speaker parsing | `plugins/fiverr/src/lib.rs` |
| Change default crop | `situations/fiverr_inbox.toml` and/or `FiverrAdapter` |
| Improve OCR tiling | `plugins/tesseract_ocr/src/lib.rs` |
| Change intent keywords | `python/classifier/serve.py` (+ situation `rule_label` if rules) |
| Change risk keywords | situation `risk_policy()` in the plugin |
| Change cache invalidation inputs | `cc_ocr_core/src/cache.rs` + adapter `cache_fingerprint` |
| Add CLI flag | `cc_ocr_cli/src/main.rs` (+ `EngineOpts` if it affects the engine) |
| Add new situation | See [04 — Plugin system](04-plugin-system.md) |
| Change shared JSON schema | `cc_ocr_plugin_api/src/types.rs` (update all serializers/consumers) |
