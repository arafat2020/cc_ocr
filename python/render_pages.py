#!/usr/bin/env python3
"""Render PDF pages to PNG at a given DPI (PyMuPDF).

Usage:
  python3 python/render_pages.py <input.pdf> <out_dir> <dpi>
"""

from __future__ import annotations

import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: render_pages.py <pdf> <out_dir> <dpi>", file=sys.stderr)
        return 2

    src = Path(sys.argv[1])
    out = Path(sys.argv[2])
    dpi = float(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)

    try:
        import pymupdf
    except ImportError:
        print("pymupdf required: pip install pymupdf", file=sys.stderr)
        return 1

    doc = pymupdf.open(src)
    zoom = dpi / 72.0
    mat = pymupdf.Matrix(zoom, zoom)
    for i, page in enumerate(doc):
        pix = page.get_pixmap(matrix=mat, alpha=False)
        pix.save(out / f"page_{i + 1:04d}.png")
    print(doc.page_count)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
