# 06 — Python side

Python is used as a **helper boundary**, not the orchestrator. Rust owns the pipeline; Python owns PDF rasterization (today) and classifier inference (sidecar).

## `python/render_pages.py`

**Caller:** [`crates/cc_ocr_core/src/render.rs`](../crates/cc_ocr_core/src/render.rs)

```bash
python3 python/render_pages.py <input.pdf> <out_dir> <dpi>
```

**Contract**

- Create `out_dir` if needed.
- Write `page_0001.png`, `page_0002.png`, … (zero-padded).
- Print page count on stdout (informational).
- Exit non-zero on failure.

Rust also has an inline PyMuPDF fallback if the script path is missing; prefer keeping the script in sync with that fallback.

**Modify when:** changing DPI handling, color mode, or PDF backend. Keep filenames stable so `collect_page_pngs` keeps working.

---

## `python/classifier/serve.py`

**Caller:** [`plugins/tf_sidecar/src/lib.rs`](../plugins/tf_sidecar/src/lib.rs)

**stdin**

```json
{"texts": ["please revise the logo", "I will contact Fiverr Support"]}
```

**stdout**

```json
{
  "results": [
    {"text": "please revise the logo", "intent": "revision", "confidence": 0.75},
    {"text": "I will contact Fiverr Support", "intent": "dispute", "confidence": 0.75}
  ]
}
```

**Rules**

- One JSON object in, one JSON object out.
- `results.len()` must equal `texts.len()` (engine asserts this).
- Heuristic keyword rules apply when no TF SavedModel is loaded.
- Optional model dir: env `CC_OCR_MODEL_DIR` or `python/classifier/model/`.

**Modify when:** tuning keyword intents, wiring a real SavedModel signature, adding batching. After changes, re-run `classify`/`run` (OCR cache can stay).

Intent vocabulary expected by situations today:

```text
greeting | scope | price | revision | delivery | dispute | system | other
```

Situation `rule_label` may emit `system` without calling the sidecar.

---

## `python/classifier/train.py`

Stub for labeled JSONL (`text`, `intent`). Writes a meta file under `--out`; replace with real TensorFlow training when a dataset exists. See `requirements.txt` for the TF pin.

---

## Environment assumptions

| Need | Why |
|------|-----|
| `python3` on `PATH` | Render + sidecar |
| `pymupdf` installed | PDF pages |
| Working directory ≈ repo root | Script discovery walks up looking for `python/` |

Override interpreter for the sidecar only:

```bash
cc_ocr --python /path/to/python run export.pdf
```

---

## Debugging tips

```bash
# Test classifier alone
echo '{"texts":["refund please"]}' | python3 python/classifier/serve.py

# Test render alone
python3 python/render_pages.py Chat/4_aug-07_01-21-pm_Aug-13_12-08-PM.pdf /tmp/pages 150
ls /tmp/pages
```

If the Rust plugin cannot find `serve.py`, run from the repo root or set an absolute `script_path` in a future classifier config.
