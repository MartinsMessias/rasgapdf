# AGENTS.md — AI Coding Agent Instructions for RasgaPDF

## Project Overview

**RasgaPDF** (`rasgapdf`) is a high-performance 2D spatial PDF parser written in pure Rust (`MuPDF` + `Rayon`) with native C-ABI (`cdylib`) bindings for **Python**, **Node.js**, **Go**, and **C# (.NET 8)**, plus a standalone CLI binary.

Its primary purpose is extracting **exact 2D physical geometry (`[x0, y0, x1, y1]` bounding boxes)**, typography (`font_name`, `font_size`), visual paragraphs (`text_blocks`), auto-detected matrix `tables`, embedded `images`, and vector `drawings` at native hardware speeds (~3–8 ms per page).

## Repository Layout

- `src/lib.rs` — Core Rust library (`parse_file`, `parse_bytes`, `parse_raw_file`, `parse_raw_bytes`, `extract_images`, `write_images_to_dir`).
- `src/main.rs` — Standalone CLI binary (`rasgapdf`).
- `src/ffi.rs` — C99 ABI exported symbols (`rasgapdf_parse_file`, `rasgapdf_parse_raw_file`, `rasgapdf_extract_images_to_dir`, `rasgapdf_free_string`, `rasgapdf_version`).
- `src/table.rs` — 2D column-projection matrix table detector.
- `src/graphics.rs` — Opt-in embedded image metadata/stream extractor and vector path (`drawings`) walker.
- `src/schema.rs` — Serde JSON data structures (`SpatialDocument`, `RawSpatialDocument`, `SpatialPage`, `SpatialBlock`, `SpatialTable`, `SpatialImage`, `SpatialDrawing`, `BoundingBox`).
- `bindings/python/` — PyPI package (`rasgapdf`).
- `bindings/nodejs/` — npm package (`rasgapdf`).
- `bindings/go/` — Go cgo bindings.
- `bindings/csharp/` — .NET 8 P/Invoke class library (`RasgaPdf`).
- `space_hf/` — Interactive Web Visualizer (Hugging Face Space + FastAPI native backend + PDF.js client-side fallback).
- `docs/` — Documentation, JSON Schema specification (`SCHEMA.md`), Architecture (`ARCHITECTURE.md`), and GitHub Pages site.

## Build & Test Commands

```bash
# Build release binary + shared library (librasgapdf.so / rasgapdf.dll / librasgapdf.dylib)
cargo build --release

# Run Rust test suite
cargo test

# Publish to registries (Cargo, PyPI, npm)
./publish_all.sh

# Sync to GitHub + Hugging Face Hub + Hugging Face Space
./sync_repos.sh
```

## Integrating RasgaPDF in Downstream Projects

- **Python:** `pip install rasgapdf` -> `rasgapdf.parse_raw(path)` or `rasgapdf.parse_pdf(path, detect_tables=True, extract_images=True, extract_drawings=True)`
- **Node.js:** `npm install rasgapdf` -> `const { parseRaw, parsePdf, extractImages } = require('rasgapdf')`
- **Rust:** `cargo add rasgapdf` -> `rasgapdf::parse_raw_file(path)` or `rasgapdf::parse_file(path, &ParserOptions::default())`
