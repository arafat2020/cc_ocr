# Conversation Chat OCR — Pluggable Core Engine

Lean local pipeline: image-based chat PDF/PNG → structured messages → intent labels.  
The **core engine** is situation-agnostic; platforms and backends plug in via Rust crates + TOML profiles.

- **End-user usage:** [`MANUAL.md`](MANUAL.md)
- **Modifier / contributor guides:** [`docs/`](docs/README.md) (crate map, data flow, plugin checklists)

## Stack

| Layer | Crate / tool | Role |
|-------|----------------|------|
| Plugin API | `cc_ocr_plugin_api` | Shared types + `SituationAdapter` / `OcrBackend` / `ClassifierBackend` |
| Core | `cc_ocr_core` | Render, preprocess, cache, `Engine` orchestration |
| CLI | `cc_ocr_cli` (`cc_ocr` binary) | Load TOML profile → registry → run |
| Situation | `plugins/fiverr`, `plugins/generic` | Crop, chrome, structure, risk |
| OCR | `plugins/tesseract_ocr` | Tesseract CLI |
| Classifier | `plugins/tf_sidecar` | Python `serve.py` |

## Data flow

```text
SituationProfile (TOML)
  → Engine(situation, ocr, classifier)
  → PDF/PNG → render → situation.preprocess → ocr → situation.structure
  → cache JSON → classifier (+ situation.rule_label) → rollup(risk_policy)
```

## CLI

```bash
# Default profile: situations/fiverr_inbox.toml
cargo run -p cc_ocr_cli -- run Chat/4_....pdf

cargo run -p cc_ocr_cli -- --situation situations/fiverr_inbox.toml ocr Chat/4_....pdf
cargo run -p cc_ocr_cli -- --situation situations/generic_chat.toml run some.png
cargo run -p cc_ocr_cli -- classify out/cache/<id>.json
cargo run -p cc_ocr_cli -- rollup out/labels/<id>.json
```

## Situation profile

```toml
id = "fiverr_inbox"
situation = "fiverr"       # → SituationAdapter
ocr = "tesseract"          # → OcrBackend
classifier = "tf_sidecar"  # → ClassifierBackend

[params]
dpi = 150
python = "python3"

[params.fiverr]
crop_left_frac = 0.22
crop_right_frac = 0.72
```

## How to add a situation

1. Create `plugins/<name>/` implementing `SituationAdapter`.
2. Add workspace member + path dep in root `Cargo.toml` / `cc_ocr_cli`.
3. Register the ID in [`crates/cc_ocr_cli/src/registry.rs`](crates/cc_ocr_cli/src/registry.rs).
4. Add `situations/<name>.toml`.
5. Run: `cc_ocr --situation situations/<name>.toml run <file>`.

Same pattern for new OCR or classifier backends (`OcrBackend` / `ClassifierBackend`).

## Cache

Key = SHA-256 of  
`(path, size, mtime, dpi, situation_fingerprint, ocr_id, ocr_version)`.

Swapping situation heuristics or OCR backend invalidates cache automatically.

## Layout

```text
crates/cc_ocr_plugin_api/
crates/cc_ocr_core/
crates/cc_ocr_cli/
plugins/fiverr/
plugins/generic/
plugins/tesseract_ocr/
plugins/tf_sidecar/
situations/fiverr_inbox.toml
situations/generic_chat.toml
python/classifier/          # used by tf_sidecar
```
