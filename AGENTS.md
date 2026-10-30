# AGENTS.md — AI Coding Agent Instructions for RasgaPDF

## Project Overview

**RasgaPDF** (`rasgapdf`) is a high-performance 2D spatial PDF parser written in 100% pure Rust (zero C/C++ dependencies, powered by internal ISO 32000 engine + `Rayon`) with native C-ABI (`cdylib`) bindings for **Python**, **Node.js**, **Go**, and **C# (.NET 8)**, plus a standalone CLI binary.

Its primary purpose is extracting **exact 2D physical geometry (`[x0, y0, x1, y1]` bounding boxes)**, typography (`font_name`, `font_size`), visual paragraphs (`text_blocks`), auto-detected matrix `tables`, embedded `images`, and vector `drawings` at native hardware speeds (~3–8 ms per page).

## Repository Layout

- `src/lib.rs` — Core Rust library exports (`parse_file`, `parse_bytes`, `parse_raw_file`, `parse_raw_bytes`, `extract_images`, `write_images_to_dir`).
- `src/main.rs` — Standalone CLI binary (`rasgapdf`).
- `src/ffi.rs` — C99 ABI exported symbols (`rasgapdf_parse_raw`, `rasgapdf_parse_raw_bytes`, `rasgapdf_parse_file`, `rasgapdf_parse_file_options`, `rasgapdf_extract_images`, `rasgapdf_extract_images_bytes`, `rasgapdf_extract_text_bytes`, `rasgapdf_free_string`, `rasgapdf_version`).
- `src/spatial.rs` — High-level spatial extraction orchestration.
- `src/table.rs` — 2D column-projection matrix table detector.
- `src/image.rs` — Opt-in embedded image metadata/stream extractor.
- `src/drawing.rs` — Opt-in vector path (`drawings`) walker.
- `src/types.rs` — Serde JSON data structures (`SpatialDocument`, `RawSpatialDocument`, `SpatialPage`, `SpatialBlock`, `SpatialTable`, `SpatialImage`, `SpatialDrawing`, `BoundingBox`).
- `src/engine/` — Pure Rust ISO 32000-1 engine (lexical analyzer, xref parser, stream decoding, font metrics).
- `bindings/python/` — PyPI package (`rasgapdf`).
- `bindings/nodejs/` — npm package (`rasgapdf`).
- `bindings/go/` — Go cgo bindings.
- `bindings/csharp/` — .NET 8 P/Invoke class library (`RasgaPdf`).
- `space_hf/` — Interactive Web Visualizer (Hugging Face Space + pure Rust WASM client-side extraction).
- `docs/` — Documentation, Diátaxis guides, JSON Schema specification (`SCHEMA.md`), Architecture (`ARCHITECTURE.md`), and GitHub Pages site.

## Build & Test Commands

```bash
# Build release binary + shared library (librasgapdf.so / rasgapdf.dll / librasgapdf.dylib)
cargo build --release

# Run Rust library unit tests
cargo test --lib

# Run Python integration test suite
pytest tests/test_integrations.py
```

## Integrating RasgaPDF in Downstream Projects

- **Python:** `pip install rasgapdf` -> `rasgapdf.parse_raw(path)` or `rasgapdf.parse_pdf(path, detect_tables=True, extract_images=True, extract_drawings=True)`
- **Node.js:** `npm install rasgapdf` -> `const { parseRaw, parsePdf, extractImages } = require('rasgapdf')`
- **Rust:** `cargo add rasgapdf` -> `rasgapdf::parse_raw_file(path)` or `rasgapdf::parse_file(path, &ParserOptions::default())`
