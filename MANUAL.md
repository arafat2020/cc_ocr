# cc_ocr User Manual

Full guide for installing, running, configuring, and extending the **Conversation Chat OCR & Classification** engine.

For **codebase architecture** (how to modify crates/plugins safely), see [`docs/README.md`](docs/README.md).

---

## 1. What this tool does

`cc_ocr` turns **image-based chat exports** (PDF or PNG screenshots) into:

1. **Structured messages** — speaker, optional time, text  
2. **Intent labels** — e.g. revision, delivery, dispute, greeting  
3. **Conversation rollup** — intent histogram + risk flags  

It is built as a **pluggable core engine**: the same pipeline can serve Fiverr inbox exports, generic chat screenshots, or new platforms you add later, without rewriting the orchestrator.

```text
PDF/PNG → page images → preprocess → OCR → structured messages
       → JSON cache → intent classify → conversation rollup
```

---

## 2. Requirements

| Dependency | Purpose | Notes |
|------------|---------|--------|
| **Rust** (stable, 1.70+) | Build the CLI | [rustup.rs](https://rustup.rs) |
| **Tesseract OCR** | Character recognition | macOS: `brew install tesseract` |
| **Python 3** | PDF render helper + classifier sidecar | 3.10+ recommended |
| **PyMuPDF** (`pymupdf`) | Render PDF pages to PNG | `pip install pymupdf` |

Optional later:

- **TensorFlow** — only when you train a real model (`python/classifier/requirements.txt`). Until then, `serve.py` uses keyword heuristics.

### Check your machine

```bash
rustc --version
cargo --version
tesseract --version
python3 --version
python3 -c "import pymupdf; print('pymupdf ok')"
```

---

## 3. Install / build

From the project root (`cc_ocr/`):

```bash
# Debug build
cargo build -p cc_ocr_cli

# Release build (recommended for real PDFs)
cargo build -p cc_ocr_cli --release
```

Binary path:

```text
target/release/cc_ocr
```

Or run without installing:

```bash
cargo run -p cc_ocr_cli --release -- --help
```

> Always run from the **repo root** (or pass absolute paths). The tool looks for `situations/`, `python/`, and writes under `out/` relative to the current working directory.

---

## 4. Quick start

### 4.1 Fiverr inbox (default)

Sample exports live in `Chat/`. Start with a small file:

```bash
# Full pipeline (OCR + classify + rollup)
cargo run -p cc_ocr_cli --release -- run Chat/4_aug-07_01-21-pm_Aug-13_12-08-PM.pdf
```

Default situation profile: `situations/fiverr_inbox.toml`.

### 4.2 Generic chat screenshots

```bash
cargo run -p cc_ocr_cli --release -- \
  --situation situations/generic_chat.toml \
  run path/to/screenshot.png
```

### 4.3 OCR only (no classifier)

```bash
cargo run -p cc_ocr_cli --release -- ocr Chat/4_aug-07_01-21-pm_Aug-13_12-08-PM.pdf
```

Writes structured JSON under `out/cache/`.

### 4.4 Classify an existing cache file

```bash
cargo run -p cc_ocr_cli --release -- classify out/cache/<hash>.json
```

---

## 5. CLI reference

```text
cc_ocr [OPTIONS] <COMMAND>
```

### Commands

| Command | What it does | Primary output |
|---------|----------------|----------------|
| `ocr <INPUT>` | Render → preprocess → OCR → structure → cache | `out/cache/<key>.json` |
| `classify <INPUT>` | Classify messages (from cache JSON **or** media via OCR first) | `out/labels/<key>.json` |
| `run <INPUT>` | Full pipeline: OCR + classify + rollup | `out/labels/<key>.json` |
| `rollup <LABELS_JSON>` | Recompute rollup from a labels file; prints JSON to stdout | stdout |

`<INPUT>` may be:

- `.pdf`, `.png`, `.jpg`, `.jpeg`, `.webp` — media  
- `.json` — for `classify` only: a prior `out/cache/*.json`

### Global options

| Option | Default | Meaning |
|--------|---------|---------|
| `--situation <PATH>` | `situations/fiverr_inbox.toml` | Situation profile (which plugins + params) |
| `--dpi <N>` | `150` (or profile) | PDF render DPI; use `150`–`200` |
| `--force` | off | Ignore cache; re-run OCR |
| `--keep-tmp` | off | Keep page images under `out/tmp/` |
| `--python <BIN>` | profile / `python3` | Python for `tf_sidecar` classifier |
| `-q`, `--quiet` | off | Hide stage logs / progress bars (stdout paths unchanged) |
| `-h`, `--help` | | Help |

### Progress & logging

On an interactive terminal, stderr shows **stage labels** plus progress bars/spinners:

```text
[       init] situation=fiverr ocr=tesseract …
[     render] ⠋ 0:00:02 Chat/4_….pdf @ 150 DPI
[ preprocess] ################------------ 6/10 0:00:01 crop / enhance
[        ocr] ################------------ 6/10 0:00:12 page 6
[  structure] 40 message(s)
[   classify] ⠹ classifying via tf_sidecar
[       done] intents={…} risk=[…]
```

Bars advance **per page** (not per character), so they do not slow OCR.  
When stderr is not a TTY (pipes/CI), the same stages print as plain lines. Use `--quiet` for scripts that only want the output path on stdout.

### Examples

```bash
# Force re-OCR at 200 DPI, keep temp images
./target/release/cc_ocr --force --dpi 200 --keep-tmp \
  ocr Chat/3_aug-07_10-02-am_Aug-07_09-10-PM_Fiverr-Inbox-05-09-2026_05_54_PM.pdf

# Explicit Fiverr profile
./target/release/cc_ocr --situation situations/fiverr_inbox.toml \
  run Chat/3_aug-07_10-02-am_Aug-09_01-13-PM_Fiverr-Inbox-05-09-2026_05_54_PM.png

# Rollup only
./target/release/cc_ocr rollup out/labels/<hash>.json
```

Stdout prints the main output path (or rollup JSON). Progress and summaries go to stderr.

---

## 6. Situation profiles (plug-and-play)

A **situation** is a TOML file that selects plugins and parameters. No code change is required to switch between registered plugs — only the profile path.

### Bundled profiles

| File | Situation | Use when |
|------|-----------|----------|
| `situations/fiverr_inbox.toml` | `fiverr` | Fiverr Inbox PDF/PNG exports (sidebar crop, Fiverr chrome filter) |
| `situations/generic_chat.toml` | `generic` | Generic chat screenshots (no crop, weak speaker heuristics) |

### Profile schema

```toml
id = "fiverr_inbox"          # human label
situation = "fiverr"         # SituationAdapter plugin id
ocr = "tesseract"            # OcrBackend plugin id
classifier = "tf_sidecar"    # ClassifierBackend plugin id

[params]
dpi = 150
python = "python3"

# Plugin-specific tables (optional)
[params.fiverr]
crop_left_frac = 0.22
crop_right_frac = 0.72
min_width_for_crop = 1200

[params.generic]
self_aliases = ["me", "you"]
```

### Registered plugin IDs (v1)

| Slot | ID | Crate |
|------|-----|--------|
| Situation | `fiverr` | `plugins/fiverr` |
| Situation | `generic` | `plugins/generic` |
| OCR | `tesseract` | `plugins/tesseract_ocr` |
| Classifier | `tf_sidecar` | `plugins/tf_sidecar` |

Unknown IDs fail at startup with a clear error. To add new IDs, see [§10 Extending](#10-extending-the-engine).

---

## 7. Pipeline stages (what happens inside)

| Stage | Owner | Behavior |
|-------|--------|----------|
| 1. Render | Core | PDF → PNG pages via PyMuPDF; raster files copied as one page |
| 2. Preprocess | Situation | Grayscale/contrast; optional horizontal crop (Fiverr drops sidebars) |
| 3. OCR | OCR plugin | Tesseract TSV lines + bboxes; tall pages are tiled |
| 4. Structure | Situation | Drop UI chrome; group into `speaker` / `time` / `text` |
| 5. Cache | Core | Write `out/cache/<key>.json` |
| 6. Classify | Situation rules + classifier | System notices may skip the model; rest go to Python sidecar |
| 7. Rollup | Core + situation risk policy | Intent histogram + risk flags → `out/labels/<key>.json` |

---

## 8. Outputs

### Directories

```text
out/
  cache/     # structured OCR (lines + messages)
  labels/    # classified messages + rollup
  tmp/       # page images during OCR (deleted unless --keep-tmp)
```

### Cache JSON (simplified)

```json
{
  "cache_key": "...",
  "source": "Chat/4_....pdf",
  "dpi": 150,
  "situation_id": "fiverr",
  "situation_fingerprint": "fiverr-v1",
  "ocr_backend": "tesseract",
  "ocr_version": "1",
  "pages": 2,
  "lines": [{ "text": "...", "conf": 88.0, "bbox": [10, 20, 400, 40], "page": 1 }],
  "messages": [{ "speaker": "buyer", "time": "Aug 07, 1:21 PM", "text": "..." }]
}
```

### Labels JSON (simplified)

```json
{
  "conversation": { "...": "same as cache" },
  "classifier_id": "tf_sidecar",
  "labeled": [
    { "text": "...", "intent": "dispute", "confidence": 0.75, "speaker": "buyer" }
  ],
  "rollup": {
    "source": "Chat/4_....pdf",
    "intent_histogram": { "dispute": 5, "delivery": 12 },
    "risk_flags": ["intent:dispute", "keyword:fiverr support"]
  }
}
```

### Speakers (Fiverr situation)

| Label | Meaning |
|-------|---------|
| `seller` | Local user (`Me` / `You` in the UI) |
| `buyer` | Counterparty username |
| `system` | Delivery / platform notices |
| `unknown` | Could not detect speaker |

Generic situation uses `self` / `other` / `system` / `unknown`.

### Intents (heuristic classifier v1)

`greeting`, `scope`, `price`, `revision`, `delivery`, `dispute`, `system`, `other`

Confidence is heuristic (~0.4–0.75) until a trained TensorFlow model is wired in.

---

## 9. Caching

Cache key = SHA-256 of:

```text
absolute path | file size | mtime | dpi | situation_fingerprint | ocr_id | ocr_version
```

**Cache hits** skip render/OCR/structure (near-instant).  

**Cache misses** happen when you:

- Change the file  
- Change `--dpi`  
- Switch situation profile / bump adapter fingerprint  
- Change OCR backend or version  
- Pass `--force`

Classifier output is **not** part of the OCR cache key; re-run `classify` or `run` after changing the sidecar or risk rules if you need fresh labels (OCR cache can still hit).

---

## 10. Extending the engine

Architecture overview: see also [`ARCHITECTURE.md`](ARCHITECTURE.md).

### 10.1 Add a new situation (e.g. Upwork)

1. Create `plugins/upwork/` with a crate implementing `SituationAdapter`  
   (`id`, `preprocess_config`, `structure`, `rule_label`, `risk_policy`, `cache_fingerprint`).
2. Add the crate to the workspace `Cargo.toml` members and `[workspace.dependencies]`.
3. Depend on it from `crates/cc_ocr_cli/Cargo.toml`.
4. Register the string id in `crates/cc_ocr_cli/src/registry.rs`:

   ```rust
   "upwork" => Arc::new(UpworkAdapter::new(...)),
   ```

5. Add `situations/upwork_inbox.toml` pointing at `situation = "upwork"`.
6. Run:

   ```bash
   cc_ocr --situation situations/upwork_inbox.toml run export.pdf
   ```

### 10.2 Add an OCR or classifier backend

Implement `OcrBackend` or `ClassifierBackend` in a new plugin crate, register the id in `registry.rs`, set `ocr = "..."` or `classifier = "..."` in a TOML profile.

### 10.3 Train the TensorFlow classifier

```bash
pip install -r python/classifier/requirements.txt
cd python/classifier
# Seed data ships at data/seed_intents.jsonl — replace/extend with your labels
python3 train.py --data data/seed_intents.jsonl --out model/ --epochs 40
```

Writes `model/saved_model/`, `model/labels.json`, `model/train_meta.json`.  
`serve.py` loads that SavedModel automatically (stderr: `classifier_backend=tensorflow`).  
Without a model (or without TF installed), it falls back to keyword heuristics.

JSONL format: `{"text":"…","intent":"revision"}`  
Intents: `greeting`, `scope`, `price`, `revision`, `delivery`, `dispute`, `system`, `other`.

---

## 11. Project layout

```text
cc_ocr/
  ARCHITECTURE.md          # Engine / plugin design
  MANUAL.md                # This document
  Cargo.toml               # Workspace
  Chat/                    # Sample Fiverr exports
  situations/              # Plug-and-play profiles
  crates/
    cc_ocr_plugin_api/     # Types + traits
    cc_ocr_core/           # Engine, render, cache
    cc_ocr_cli/            # `cc_ocr` binary + registry
  plugins/
    fiverr/
    generic/
    tesseract_ocr/
    tf_sidecar/
  python/
    render_pages.py
    classifier/
      serve.py
      train.py
      requirements.txt
  out/                     # Generated (gitignored)
```

---

## 12. Performance notes

| Input | First run | Later runs (cache hit) |
|-------|-----------|-------------------------|
| Small PNG / 2-page PDF | seconds–tens of seconds | ~tens of ms for OCR stage |
| ~10-page PDF | roughly under a minute on a laptop | cache hit |
| 100MB+ multi-page PDF | minutes (OCR dominates) | cache hit |

Tips:

- Prefer `--dpi 150` unless OCR quality is poor.  
- Use `--release`.  
- Do not leave `--keep-tmp` on for large batches (disk).  
- Process large archives file-by-file; the engine already works page-by-page.

---

## 13. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `tesseract not found` | Tesseract not installed / not on `PATH` | Install Tesseract; reopen terminal |
| `PDF render failed` / pymupdf errors | Missing PyMuPDF | `pip install pymupdf` |
| `unsupported input extension` | Wrong file type | Use pdf/png/jpg/webp (or cache json for `classify`) |
| `unknown situation plugin id` | Typo in TOML or unregistered plugin | Check `situation =` and `registry.rs` |
| `locate python/classifier/serve.py` | Not running from repo root | `cd` to project root or set up paths |
| Empty / garbage messages | Wrong situation, or UI chrome | Try the other profile; tune crop; use `--force` |
| `Image too small to scale` on stderr | Harmless Tesseract noise on thin tiles | Ignore if the run completes |
| Cache never updates after heuristic change | Fingerprint not bumped | Bump `cache_fingerprint()` in the situation adapter |
| Slow every time | Using `--force` or changing dpi/profile each run | Drop `--force`; keep profile stable |

---

## 14. Privacy & safety

- Processing is **local** (no cloud OCR/LLM in the default path).  
- Chat exports may contain **credentials, personal data, and payment details** — treat `out/` as sensitive.  
- Do not commit `out/`, raw client exports, or labeled datasets with secrets.

---

## 15. Command cheat sheet

```bash
# Build
cargo build -p cc_ocr_cli --release

# Fiverr full run
./target/release/cc_ocr run Chat/4_aug-07_01-21-pm_Aug-13_12-08-PM.pdf

# Generic screenshot
./target/release/cc_ocr --situation situations/generic_chat.toml run shot.png

# OCR only / classify / rollup
./target/release/cc_ocr ocr export.pdf
./target/release/cc_ocr classify out/cache/<key>.json
./target/release/cc_ocr rollup out/labels/<key>.json

# Force refresh
./target/release/cc_ocr --force run export.pdf
```

For a short design summary, read [`ARCHITECTURE.md`](ARCHITECTURE.md).  
For contributor-oriented architecture (crate map, plugin checklists, core change rules), start at [`docs/README.md`](docs/README.md).
