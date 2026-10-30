# 03 — Data flow & types

## End-to-end flow

```text
INPUT (.pdf / .png / …)
  │
  ├─[Engine::run_ocr]
  │    1. cache lookup (key = path+size+mtime+dpi+situation_fp+ocr_id+ocr_ver)
  │    2. render_pages()           → Vec<PageImage>
  │    3. situation.preprocess_config()
  │       preprocess_page()        → Vec<PageImage> (_prep.png)
  │    4. ocr.ocr_pages()          → Vec<OcrLine>
  │    5. situation.structure()    → Vec<Message>
  │    6. save CachedConversation  → out/cache/<key>.json
  │
  ├─[Engine::run_classify / run_full]
  │    7. for each Message:
  │         situation.rule_label()  OR  classifier.classify(texts)
  │                                → Vec<LabeledMessage>
  │    8. rollup(source, labeled, situation.risk_policy())
  │                                → ConversationRollup
  │    9. save PipelineResult      → out/labels/<key>.json
  │
  └─[Engine::run_rollup]
       reload labels JSON → recompute rollup with current risk_policy
```

## Type contracts

Defined in [`crates/cc_ocr_plugin_api/src/types.rs`](../crates/cc_ocr_plugin_api/src/types.rs).

### `PageImage` (in-memory / on-disk handle)

| Field | Meaning |
|-------|---------|
| `page` | 1-based page index |
| `path` | PNG path under `out/tmp/…` |
| `width` / `height` | pixels |

### `OcrLine`

| Field | Meaning |
|-------|---------|
| `text` | Recognized line |
| `conf` | Average word confidence (−1 if unknown) |
| `bbox` | Optional `[x0,y0,x1,y1]` in page pixels |
| `page` | Page index |

### `Message`

| Field | Meaning |
|-------|---------|
| `speaker` | Situation-defined role (`buyer`/`seller`/… or `self`/`other`) |
| `time` | Optional timestamp string from UI |
| `text` | Message body (may be multi-line) |

### `LabeledMessage`

| Field | Meaning |
|-------|---------|
| `text` | Usually same as message text |
| `intent` | Label string |
| `confidence` | 0.0–1.0 |
| `speaker` / `time` | Optional; engine fills from `Message` if classifier omits them |

### `CachedConversation`

OCR-stage artifact. Important identity fields:

- `cache_key`
- `situation_id`, `situation_fingerprint`
- `ocr_backend`, `ocr_version`
- `lines`, `messages`

### `PipelineResult`

Labels-stage artifact: embeds `CachedConversation` + `classifier_id` + `labeled` + `rollup`.

### `ConversationRollup`

| Field | Meaning |
|-------|---------|
| `intent_histogram` | Count per intent |
| `risk_flags` | `intent:…` and `keyword:…` strings from `RiskPolicy` |

## Preprocess & risk (plugin-owned config)

- **`PreprocessConfig`** — grayscale, contrast, crop fractions. Core applies it; situation supplies it.
- **`RiskPolicy`** — `risk_intents` + `risk_keywords`. Core `rollup` applies it; situation supplies it.

## Version / fingerprint discipline

When you change behavior that affects cached OCR output:

1. Bump the situation’s `cache_fingerprint()` **or**
2. Bump the OCR plugin’s `version()` **or**
3. Change DPI / input file

Otherwise developers will chase “stale cache” bugs.

Classifier-only changes do **not** need an OCR cache bump; re-run `classify` / `run` (OCR may still hit).

## CLI mapping

| Command | Engine method |
|---------|---------------|
| `ocr` | `run_ocr` |
| `classify` | `run_classify` |
| `run` | `run_full` |
| `rollup` | `run_rollup` |
