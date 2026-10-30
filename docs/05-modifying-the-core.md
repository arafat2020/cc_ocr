# 05 — Modifying the core

Guidance for changes inside `cc_ocr_core` (and shared API types). Prefer plugins for UI/platform logic; touch core for **orchestration, I/O, and shared mechanics**.

## Safe change patterns

### A. Change render behavior

File: [`crates/cc_ocr_core/src/render.rs`](../crates/cc_ocr_core/src/render.rs)

- Keep page-by-page output as `page_XXXX.png` so OCR plugins stay compatible.
- If you switch from PyMuPDF to native pdfium, keep the same function signature:  
  `render_pages(source, tmp_dir, dpi) -> Result<Vec<PageImage>>`.
- Update [`docs/06-python-side.md`](06-python-side.md) if the Python helper contract changes.

### B. Change preprocess math

File: [`crates/cc_ocr_core/src/preprocess.rs`](../crates/cc_ocr_core/src/preprocess.rs)

- Core should stay **config-driven** (`PreprocessConfig` from the situation).
- Do not hardcode Fiverr crop ratios here — put defaults on the adapter / TOML.
- New knobs: add fields to `PreprocessConfig` in `plugin_api`, then apply them here.

### C. Change cache key

File: [`crates/cc_ocr_core/src/cache.rs`](../crates/cc_ocr_core/src/cache.rs)

Current parts (`CacheKeyParts`):

```text
path | size | mtime | dpi | situation_fingerprint | ocr_id | ocr_version
```

- Adding a field invalidates all caches (good if intentional).
- Do **not** put classifier id in the OCR cache key (labels are a separate artifact).
- After changing the formula, document it in [03 — Data flow](03-data-flow-and-types.md).

### D. Change Engine orchestration

File: [`crates/cc_ocr_core/src/engine.rs`](../crates/cc_ocr_core/src/engine.rs)

Typical safe edits:

- Logging / progress messages
- Skipping blank pages
- Ordering of rule_label vs classifier
- Writing extra metadata into `CachedConversation` / `PipelineResult` (**requires `plugin_api` type updates**)

Avoid:

- Importing `cc_ocr_plugin_fiverr` (or any plugin) from core
- Embedding chrome regexes or intent keyword lists in the engine

### E. Change rollup

File: [`crates/cc_ocr_core/src/rollup.rs`](../crates/cc_ocr_core/src/rollup.rs)

- Keep policy data on `RiskPolicy`.
- Unit tests live in this file — extend them when flag semantics change.

### F. Change profile schema

File: [`crates/cc_ocr_core/src/profile.rs`](../crates/cc_ocr_core/src/profile.rs)

1. Add serde structs with defaults.
2. Update bundled TOMLs under `situations/`.
3. Wire new fields in `registry.rs`.
4. Mention in `MANUAL.md` § situation profiles.

---

## Changing `plugin_api` types

Treat this as a **breaking contract** for all plugins.

1. Edit `types.rs` / traits.
2. `cargo build --workspace` and fix every plugin + core + cli.
3. Old `out/cache/*.json` may fail to deserialize — delete `out/` or migrate.
4. Update [03 — Data flow](03-data-flow-and-types.md) and sample JSON in `MANUAL.md`.

---

## Testing expectations

```bash
# Unit tests (structure heuristics, rollup, …)
cargo test --workspace

# Smoke (from repo root)
cargo run -p cc_ocr_cli --release -- --force run Chat/_preview/conv3_png_mid.png
cargo run -p cc_ocr_cli --release -- --situation situations/generic_chat.toml --force \
  run Chat/_preview/conv3_png_mid.png
```

When changing Fiverr structure rules, keep/extend tests in `plugins/fiverr/src/lib.rs`.

When changing engine control flow, prefer a small integration smoke over only compiling.

---

## Anti-patterns

| Don’t | Do instead |
|-------|------------|
| `if path.contains("Fiverr")` in core | Situation plugin + profile |
| Copy-paste Tesseract into core | Keep `OcrBackend` plugin |
| Silent heuristic changes without fingerprint bump | Bump `cache_fingerprint` / `ocr.version` |
| Dynamic `dlopen` plugins (v1) | Compile-time registry match |
| Commit `out/` or huge Chat binaries into docs PRs | Keep artifacts local |

---

## Suggested PR checklist for core changes

- [ ] Still no plugin crate deps inside `cc_ocr_core`
- [ ] Cache / fingerprint story updated if OCR outputs change
- [ ] `cargo test --workspace` green
- [ ] Smoke `run` on a small Chat sample
- [ ] Docs touched (`docs/` + `MANUAL.md` if user-visible)
