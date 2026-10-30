#!/usr/bin/env python3
"""Intent classifier sidecar.

Reads one JSON object from stdin:
  {"texts": ["...", "..."]}

Writes one JSON object to stdout:
  {"results": [{"text": "...", "intent": "revision", "confidence": 0.9}, ...]}

Resolution order:
  1. TensorFlow SavedModel at MODEL_DIR/saved_model (+ labels.json)
  2. Keyword heuristic fallback

MODEL_DIR defaults to ./model next to this file, or env CC_OCR_MODEL_DIR.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

LABELS = (
    "greeting",
    "scope",
    "price",
    "revision",
    "delivery",
    "dispute",
    "system",
    "other",
)

KEYWORD_RULES: list[tuple[str, tuple[str, ...]]] = [
    ("dispute", ("scam", "scammed", "dispute", "fiverr support", "refund", "legal", "chargeback")),
    ("revision", ("revis", "change the", "update the", "fix the", "logo color")),
    ("delivery", ("deliver", "handoff", "github", "source code", "repository", ".env")),
    ("price", ("price", "budget", "payment", "pay you", "owe", "invoice")),
    ("greeting", ("hello", "hi,", "assalam", "hope you are", "good morning")),
    ("scope", ("requirement", "checklist", "feature", "scope", "need you to")),
    ("system", ("order was delivered", "order delivered")),
]


def heuristic_intent(text: str) -> tuple[str, float]:
    lower = text.lower()
    for intent, keys in KEYWORD_RULES:
        for k in keys:
            if k in lower:
                return intent, 0.75
    if re.search(r"\b(thanks|thank you)\b", lower):
        return "greeting", 0.6
    return "other", 0.4


def load_labels(model_dir: Path) -> list[str]:
    path = model_dir / "labels.json"
    if not path.exists():
        return list(LABELS)
    data = json.loads(path.read_text())
    labels = data.get("labels")
    if isinstance(labels, list) and labels:
        return [str(x) for x in labels]
    return list(LABELS)


def load_tf_bundle(model_dir: Path):
    """Return (predict_fn, labels) or None."""
    saved = model_dir / "saved_model"
    if not saved.exists():
        return None
    try:
        import tensorflow as tf  # type: ignore
    except ImportError:
        print("tensorflow missing; falling back to heuristics", file=sys.stderr)
        return None
    try:
        model = tf.saved_model.load(str(saved))
    except Exception as e:
        print(f"failed to load SavedModel ({e}); heuristics", file=sys.stderr)
        return None

    labels = load_labels(model_dir)
    # Prefer explicit serving signature; fall back to calling the model object.
    fn = None
    if hasattr(model, "signatures") and "serving_default" in model.signatures:
        fn = model.signatures["serving_default"]
    elif callable(model):
        fn = model
    else:
        print("SavedModel has no callable signature; heuristics", file=sys.stderr)
        return None

    def predict_batch(texts: list[str]) -> list[tuple[str, float]]:
        import tensorflow as tf  # type: ignore

        tensor = tf.constant(texts)
        out = fn(tensor)
        # Signature may return a dict of tensors or a raw tensor.
        if isinstance(out, dict):
            # common keys: output_0, probs, or first value
            if "probs" in out:
                probs = out["probs"]
            elif "output_0" in out:
                probs = out["output_0"]
            else:
                probs = next(iter(out.values()))
        else:
            probs = out
        probs = probs.numpy()
        results: list[tuple[str, float]] = []
        for row in probs:
            idx = int(row.argmax())
            conf = float(row[idx])
            intent = labels[idx] if 0 <= idx < len(labels) else "other"
            results.append((intent, conf))
        return results

    return predict_batch, labels


def predict(texts: list[str], bundle) -> list[dict]:
    if bundle is None:
        return [
            {"text": t, "intent": intent, "confidence": conf}
            for t, (intent, conf) in ((t, heuristic_intent(t)) for t in texts)
        ]
    predict_batch, _labels = bundle
    try:
        preds = predict_batch(texts)
    except Exception as e:
        print(f"TF inference failed ({e}); heuristics", file=sys.stderr)
        return [
            {"text": t, "intent": intent, "confidence": conf}
            for t, (intent, conf) in ((t, heuristic_intent(t)) for t in texts)
        ]
    return [
        {"text": t, "intent": intent, "confidence": conf}
        for t, (intent, conf) in zip(texts, preds)
    ]


def main() -> int:
    raw = sys.stdin.read()
    if not raw.strip():
        print(json.dumps({"results": []}))
        return 0
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"invalid json: {e}"}), file=sys.stderr)
        return 1

    texts = payload.get("texts") or []
    if not isinstance(texts, list):
        print(json.dumps({"error": "texts must be a list"}), file=sys.stderr)
        return 1

    model_dir = Path(os.environ.get("CC_OCR_MODEL_DIR", Path(__file__).parent / "model"))
    bundle = load_tf_bundle(model_dir)
    backend = "tensorflow" if bundle is not None else "heuristic"
    results = predict([str(t) for t in texts], bundle)
    # backend is useful for debugging; Rust ignores unknown top-level keys if any —
    # keep stdout as results-only for the plugin contract, log backend on stderr.
    print(f"classifier_backend={backend}", file=sys.stderr)
    print(json.dumps({"results": results}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
