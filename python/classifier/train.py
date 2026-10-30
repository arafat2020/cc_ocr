#!/usr/bin/env python3
"""Train a TensorFlow text intent classifier and export a SavedModel.

Input JSONL lines: {"text": "...", "intent": "revision"}
Intents: greeting, scope, price, revision, delivery, dispute, system, other

Usage:
  pip install -r requirements.txt
  python3 train.py --data data/seed_intents.jsonl --out model/
  python3 train.py --data data/seed_intents.jsonl --out model/ --epochs 20

Artifacts written to --out:
  saved_model/     TensorFlow SavedModel (string in → probs out)
  labels.json      ordered label list
  train_meta.json  metrics + hyperparams
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

LABELS = [
    "greeting",
    "scope",
    "price",
    "revision",
    "delivery",
    "dispute",
    "system",
    "other",
]


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    with path.open() as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError as e:
                raise SystemExit(f"invalid JSONL at {path}:{line_no}: {e}") from e
    return rows


def prepare_xy(rows: list[dict]) -> tuple[list[str], list[int]]:
    label_to_id = {name: i for i, name in enumerate(LABELS)}
    texts: list[str] = []
    y: list[int] = []
    skipped = 0
    for r in rows:
        text = (r.get("text") or "").strip()
        intent = r.get("intent")
        if not text or intent not in label_to_id:
            skipped += 1
            continue
        texts.append(text)
        y.append(label_to_id[intent])
    if skipped:
        print(f"skipped {skipped} rows (empty text or unknown intent)")
    if not texts:
        raise SystemExit("no usable training rows")
    return texts, y


def stratified_split(
    texts: list[str], y: list[int], val_frac: float, seed: int
) -> tuple[list[str], list[int], list[str], list[int]]:
    by_label: dict[int, list[str]] = {}
    for t, label in zip(texts, y):
        by_label.setdefault(label, []).append(t)

    rng = random.Random(seed)
    train_x, train_y, val_x, val_y = [], [], [], []
    for label, items in by_label.items():
        rng.shuffle(items)
        n_val = max(1, int(round(len(items) * val_frac))) if len(items) > 1 else 0
        val_items = items[:n_val]
        train_items = items[n_val:] or items  # keep at least one train example
        if not train_items:
            train_items = items
            val_items = []
        for t in train_items:
            train_x.append(t)
            train_y.append(label)
        for t in val_items:
            val_x.append(t)
            val_y.append(label)
    return train_x, train_y, val_x, val_y


def build_model(vectorizer, num_classes: int):
    """Bag-of-words multi-hot MLP — strong on small labeled sets.

    `vectorizer` must already be `.adapt()`-ed so output dims are concrete.
    """
    import tensorflow as tf
    from tensorflow.keras import layers

    model = tf.keras.Sequential(
        [
            vectorizer,
            layers.Dense(128, activation="relu"),
            layers.Dropout(0.3),
            layers.Dense(64, activation="relu"),
            layers.Dropout(0.2),
            layers.Dense(num_classes, activation="softmax", name="probs"),
        ],
        name="intent_classifier",
    )
    # Concrete input for SavedModel / string batches
    model.build(input_shape=(None,))
    return model


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--data",
        type=Path,
        default=Path(__file__).parent / "data" / "seed_intents.jsonl",
        help="JSONL with text,intent",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=Path(__file__).parent / "model",
        help="Output directory for SavedModel + labels",
    )
    ap.add_argument("--epochs", type=int, default=40)
    ap.add_argument("--batch-size", type=int, default=8)
    ap.add_argument("--max-tokens", type=int, default=5000)
    ap.add_argument("--seq-len", type=int, default=64)  # unused (multi-hot); kept for CLI compat
    ap.add_argument("--embed-dim", type=int, default=64)  # unused (multi-hot); kept for CLI compat
    ap.add_argument("--val-frac", type=float, default=0.2)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    if not args.data.exists():
        print(f"missing data file: {args.data}")
        return 1

    try:
        import tensorflow as tf
    except ImportError:
        print("tensorflow is required: pip install -r requirements.txt")
        return 1

    tf.random.set_seed(args.seed)
    random.seed(args.seed)

    rows = load_jsonl(args.data)
    texts, y = prepare_xy(rows)
    train_x, train_y, val_x, val_y = stratified_split(
        texts, y, args.val_frac, args.seed
    )
    print(
        f"examples={len(texts)} train={len(train_x)} val={len(val_x)} "
        f"classes={len(LABELS)}"
    )

    from tensorflow.keras import layers

    vectorizer = layers.TextVectorization(
        max_tokens=args.max_tokens,
        output_mode="multi_hot",
        name="text_vectorizer",
    )
    vectorizer.adapt(tf.constant(train_x))
    model = build_model(vectorizer, num_classes=len(LABELS))

    model.compile(
        optimizer=tf.keras.optimizers.Adam(1e-3),
        loss="sparse_categorical_crossentropy",
        metrics=["accuracy"],
    )

    callbacks = [
        tf.keras.callbacks.EarlyStopping(
            monitor="val_loss" if val_x else "loss",
            patience=8,
            restore_best_weights=True,
        )
    ]

    history = model.fit(
        tf.constant(train_x),
        tf.constant(train_y),
        validation_data=(tf.constant(val_x), tf.constant(val_y)) if val_x else None,
        epochs=args.epochs,
        batch_size=args.batch_size,
        callbacks=callbacks,
        verbose=2,
    )

    metrics: dict = {}
    if val_x:
        loss, acc = model.evaluate(tf.constant(val_x), tf.constant(val_y), verbose=0)
        metrics["val_loss"] = float(loss)
        metrics["val_accuracy"] = float(acc)
        print(f"val_accuracy={acc:.3f} val_loss={loss:.3f}")

    args.out.mkdir(parents=True, exist_ok=True)
    saved_dir = args.out / "saved_model"
    if saved_dir.exists():
        import shutil

        shutil.rmtree(saved_dir)

    # Explicit serving signature: batch of strings → probability vectors
    @tf.function(input_signature=[tf.TensorSpec(shape=[None], dtype=tf.string)])
    def serve_fn(texts: tf.Tensor) -> tf.Tensor:
        return model(texts, training=False)

    tf.saved_model.save(
        model,
        str(saved_dir),
        signatures={"serving_default": serve_fn},
    )

    (args.out / "labels.json").write_text(
        json.dumps({"labels": LABELS}, indent=2) + "\n"
    )
    meta = {
        "status": "trained",
        "framework": "tensorflow",
        "tf_version": tf.__version__,
        "n_examples": len(texts),
        "n_train": len(train_x),
        "n_val": len(val_x),
        "labels": LABELS,
        "epochs_ran": len(history.history.get("loss", [])),
        "hyperparams": {
            "epochs": args.epochs,
            "batch_size": args.batch_size,
            "max_tokens": args.max_tokens,
            "seq_len": args.seq_len,
            "embed_dim": args.embed_dim,
            "val_frac": args.val_frac,
            "seed": args.seed,
        },
        "metrics": metrics,
        "data": str(args.data),
        "saved_model": str(saved_dir),
    }
    (args.out / "train_meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(f"wrote SavedModel → {saved_dir}")
    print(f"wrote labels     → {args.out / 'labels.json'}")
    print(f"wrote meta       → {args.out / 'train_meta.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
