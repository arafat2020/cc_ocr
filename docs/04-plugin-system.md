# 04 — Plugin system

This is the main extension path. Prefer a **new plugin + TOML** over editing `engine.rs` for platform-specific behavior.

## Traits (API)

Source: [`crates/cc_ocr_plugin_api/src/traits.rs`](../crates/cc_ocr_plugin_api/src/traits.rs)

### `SituationAdapter`

| Method | Responsibility |
|--------|----------------|
| `id()` | Stable string used in profiles (`fiverr`, `generic`, …) |
| `preprocess_config()` | Crop / contrast for this UI |
| `structure(lines)` | OCR lines → messages |
| `rule_label(msg)` | Optional skip-model labels (e.g. system notices) |
| `risk_policy()` | What counts as conversation risk |
| `cache_fingerprint()` | Bump when heuristics change |

### `OcrBackend`

| Method | Responsibility |
|--------|----------------|
| `id()` / `version()` | Identity for cache key |
| `ocr_pages(pages)` | Images → `OcrLine`s |

### `ClassifierBackend`

| Method | Responsibility |
|--------|----------------|
| `id()` | Identity stored on `PipelineResult` |
| `classify(texts)` | Batch texts → `LabeledMessage`s (speaker/time may be empty; engine fills) |

## Registry (compile-time wiring)

Source: [`crates/cc_ocr_cli/src/registry.rs`](../crates/cc_ocr_cli/src/registry.rs)

```rust
match profile.situation.as_str() {
    "fiverr" => Arc::new(FiverrAdapter::new(...)),
    "generic" => Arc::new(GenericAdapter::new(...)),
    other => bail!("unknown situation plugin id: {other}"),
}
```

Same pattern for `ocr` and `classifier`.  
**If it is not in this match, the TOML id will not work.**

## Profile TOML

Example: [`situations/fiverr_inbox.toml`](../situations/fiverr_inbox.toml)

```toml
id = "fiverr_inbox"
situation = "fiverr"
ocr = "tesseract"
classifier = "tf_sidecar"

[params]
dpi = 150
python = "python3"

[params.fiverr]
crop_left_frac = 0.22
crop_right_frac = 0.72
min_width_for_crop = 1200
```

Profile parsing lives in [`crates/cc_ocr_core/src/profile.rs`](../crates/cc_ocr_core/src/profile.rs).  
If you add a new `[params.xyz]` table:

1. Add a `XyzParams` struct + field on `ProfileParams`.
2. Read it in `registry.rs` when constructing the plugin.

---

## Checklist: add a situation plugin

Replace `acme` with your id.

1. **Create crate**

   ```text
   plugins/acme/
     Cargo.toml
     src/lib.rs
   ```

   `Cargo.toml` should depend on `cc_ocr_plugin_api` (and `regex` if needed).

2. **Implement `SituationAdapter`** in `src/lib.rs`  
   Start from `plugins/generic` (simple) or `plugins/fiverr` (rich heuristics).

3. **Workspace**

   - Add `"plugins/acme"` to root `Cargo.toml` `members`.
   - Add `cc_ocr_plugin_acme = { path = "plugins/acme" }` under `[workspace.dependencies]`.

4. **CLI dependency**

   - Add `cc_ocr_plugin_acme.workspace = true` to `crates/cc_ocr_cli/Cargo.toml`.

5. **Register**

   - `use cc_ocr_plugin_acme::AcmeAdapter;`
   - Add `"acme" => …` arm in `registry.rs`.

6. **Profile params (if needed)**

   - Extend `ProfileParams` in `profile.rs`.
   - Pass into `AcmeAdapter::new(...)`.

7. **Ship a profile**

   ```toml
   # situations/acme.toml
   id = "acme"
   situation = "acme"
   ocr = "tesseract"
   classifier = "tf_sidecar"
   [params]
   dpi = 150
   ```

8. **Verify**

   ```bash
   cargo test -p cc_ocr_plugin_acme
   cargo run -p cc_ocr_cli --release -- --situation situations/acme.toml run sample.png
   ```

9. **Docs**

   - Mention the new id in `MANUAL.md` and this folder’s crate map if it is a first-class plug.

---

## Checklist: add OCR or classifier backend

Same pattern as above, but implement `OcrBackend` or `ClassifierBackend`, register under `profile.ocr` / `profile.classifier`, and point a TOML profile at the new id.

For classifiers that need config (model path, python bin), put knobs on the plugin struct and feed them from `[params]` / CLI (`--python` already overrides `params.python` for `tf_sidecar`).

---

## Existing plugins (behavior notes)

### `fiverr`

- Crops left ~22% and right beyond ~72% when width ≥ 1200px.
- Drops Fiverr chrome (`Dashboard`, `Last seen`, …).
- Maps `Me`/`You` → `seller`, other usernames → `buyer`.
- Risk includes `fiverr support`, scam/dispute language.
- Fingerprint: `fiverr-v1` — **bump when structure/chrome rules change**.

### `generic`

- No crop (`PreprocessConfig::no_crop()`).
- Weaker speaker+time headers; speakers `self`/`other`.
- Fingerprint: `generic-v1`.

### `tesseract`

- Requires `tesseract` on `PATH`.
- PSM 6, TSV word grouping, vertical tiles if height > 3500.
- Version: `"1"`.

### `tf_sidecar`

- Spawns `python/classifier/serve.py` with stdin JSON `{"texts":[...]}`.
- Heuristic intents until a SavedModel exists under `python/classifier/model/`.
