---
title: RasgaPDF 2D Spatial PDF Parser
emoji: ⚡
colorFrom: blue
colorTo: indigo
sdk: static
pinned: false
license: agpl-3.0
tags:
- pdf
- spatial
- rust
- layout-analysis
- document-parsing
- bounding-box
---

# ⚡ RasgaPDF // 2D Spatial PDF Parser (Web Visualizer)

<div align="center">

[![Duplicate this Space](https://huggingface.co/datasets/huggingface/badges/resolve/main/duplicate-this-space-md.svg)](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)
[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Crates.io](https://img.shields.io/crates/v/rasgapdf.svg?color=orange&logo=rust)](https://crates.io/crates/rasgapdf)
[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=red&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![Model Repo](https://img.shields.io/badge/%F0%9F%A4%97%20Hub-Model%20Repository-blue)](https://huggingface.co/martinsmessias/rasgapdf)

</div>

### 🚀 Client-Side Pure Rust WebAssembly Engine
- **⚡ Pure Rust WebAssembly Engine (`rasgapdf.wasm`):** Powered by the official Rust spatial PDF parser compiled directly to WebAssembly (`wasm32-unknown-unknown`). Provides microsecond-level extraction of text spans, tables, bounding boxes, drawings, and images directly inside your browser with 100% privacy and zero data transmission to external servers.
- **🛡️ Zero Server Costs:** Runs 100% locally on your device via client-side WebAssembly with zero API quotas or cloud latency.
- **Want to run your own instance?** Click the **[Duplicate this Space](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)** button.

### 🧩 Client-side mode: full document, same schema
The fallback is not a reduced preview — it extracts the **whole document** and exports the same JSON shape as the Rust engine:

```json
{ "file_name": "...", "total_pages": 3, "parse_time_ms": 42.1,
  "pages": [ { "page_number": 1, "width": 612, "height": 792, "rotation": 0,
               "text_blocks": [...], "tables": [...], "raw_span_count": 23,
               "images": [...], "drawings": [...] } ] }
```

- **Progressive parsing:** page 1 is extracted first so the canvas appears immediately; the remaining pages are processed in the background. **Copy JSON** / **Download JSON** wait for every page before exporting (`rasgapdf_document.json`).
- **Tables:** same detector and thresholds as `src/table.rs` (minimum 3 columns, 16 pt column tolerance, 6 pt row tolerance), including `headers`, `rows` and per-cell `bbox`.
- **Drawings:** paths are walked from the PDF operator list into `items` (`line`, `curve`, `rect`), with fill/stroke colours, opacity, dash pattern, line join/cap and `bbox` in points.
- **Text blocks:** geometric grouping of spans into lines/paragraphs, with per-span `font_name`, `font_size`, `char_count` and `bbox`.

### ⚠️ Known differences vs. the native engine
| Field | Native Rust (`librasgapdf`) | Browser (PDF.js) |
|---|---|---|
| `drawings[].seqno` | counts every display-list operation, including text | counts only painted vector paths |
| `images[].xref` / `filter` | real stream values | `0` / `null` (not exposed by PDF.js) |
| `images[].sha256` / `byte_length` | digest of the original stream | digest of the normalized PNG rendered by the browser |
| glyph boxes | Pure Rust font metrics | WASM font metrics (line/table `bbox` may differ by ~2 pt) |
| `images[].mime_type` | detected from the stream | always `image/png` |

These differences are metadata/geometry only: counts, ordering, table structure, colours and the JSON schema are identical.

### ⚠️ Known limitation: protected PDFs
- **DRM (Adobe ADEPT / `EBX_HANDLER`)**: typical of e-books. The content is locked by the store's key, so no PDF library can read it without removing the DRM. The visualizer reports this explicitly instead of failing silently.
- **Password-protected**: detected and reported; remove the password (export an unprotected copy) and try again.
