# Developer documentation

Guides for **understanding and modifying** the `cc_ocr` codebase.

| Doc | Audience | Contents |
|-----|----------|----------|
| [01 — Architecture overview](01-architecture-overview.md) | Everyone | Goals, layers, dependency rules, mental model |
| [02 — Crate & file map](02-crate-and-file-map.md) | Anyone editing code | What each crate/module owns |
| [03 — Data flow & types](03-data-flow-and-types.md) | Pipeline changes | Stage-by-stage contracts, JSON shapes |
| [04 — Plugin system](04-plugin-system.md) | New platforms / backends | Traits, registry, TOML profiles, checklists |
| [05 — Modifying the core](05-modifying-the-core.md) | Engine / cache / render | Safe change patterns, fingerprints, tests |
| [06 — Python side](06-python-side.md) | Classifier / PDF render | `serve.py`, `render_pages.py`, training stub |

**End-user usage** (install, CLI flags, troubleshooting): see [`../MANUAL.md`](../MANUAL.md).  
**Short design summary**: see [`../ARCHITECTURE.md`](../ARCHITECTURE.md).

## Quick orientation

```text
TOML profile  →  CLI registry  →  Engine(situation, ocr, classifier)
                                      │
                    render → preprocess → ocr → structure → cache
                                      │
                         rule_label + classify → rollup → labels
```

**Rule of thumb**

- Platform / UI / risk heuristics → **situation plugin** (`plugins/*`)
- OCR implementation → **OCR plugin**
- Intent model wiring → **classifier plugin**
- Orchestration, I/O, cache → **`cc_ocr_core`**
- Shared contracts → **`cc_ocr_plugin_api`** (change carefully; everything depends on it)
