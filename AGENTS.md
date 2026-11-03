# AGENTS.md — AI Agent & Developer Engineering Playbook for RasgaPDF

This document is the definitive engineering guide for AI coding agents and human contributors working on **RasgaPDF** (`rasgapdf`). It formalizes architectural invariants, coordinate contracts, testing gates, multi-platform build matrix rules, and the complete version-release lifecycle.

---

## 1. Project Architecture & Philosophy

RasgaPDF is an ultra-fast, zero-dependency 2D spatial PDF parser implemented in **100% pure Rust** (no C/C++ dependencies like Poppler, MuPDF, or PDFium).

```
                      ┌──────────────────────────────────────┐
                      │    RasgaPDF Core (Pure Rust Engine)  │
                      │   ISO 32000-1 Lexer, XRef, Content   │
                      │  Viewport Normalization & Clustering  │
                      └──────────────────┬───────────────────┘
                                         │
        ┌────────────────────────────────┼────────────────────────────────┐
        ▼                                ▼                                ▼
  Native C-ABI                      WebAssembly                     Standalone CLI
  (librasgapdf.so /                 (rasgapdf.wasm                   (rasgapdf binary
   rasgapdf.dll /                    wasm32-unknown)                  Cargo bin)
   librasgapdf.dylib)                    │                                │
        │                          Web Visualizer &                       │
 ┌──────┴──────┬──────────┬────────┐ Hugging Face Space                   │
 ▼             ▼          ▼        ▼                                      │
Python        Node.js     C# (.NET) Go                                    │
(ctypes/CFFI) (koffi FFI) (P/Invoke)(cgo)                                 │
```

### Core Invariants:
1. **Zero External C Dependencies:** The engine compiles with standard `cargo build` across Linux, Windows, macOS, and WebAssembly (`wasm32-unknown-unknown`).
2. **Top-Left Point System:** Coordinates are normalized to Top-Left `(x0, y0)` in typographic points ($1/72$ inch), matching browser DOM, HTML5 Canvas, and SVG standards.
3. **Zero Loss of Physical Geometry:** Every text block, line, span, table, drawing, and chunk retains exact physical bounding boxes.

---

## 2. Repository Layout

```
rasgapdf/
├── src/
│   ├── lib.rs              # Root crate exports & version constant
│   ├── main.rs             # CLI entrypoint with Clap
│   ├── ffi.rs              # C99 extern "C" ABI exports and memory allocators
│   ├── types.rs            # Serde data models (SpatialDocument, SpatialChunk, BoundingBox)
│   ├── spatial.rs          # Parsing orchestration and layout pipelines
│   ├── table.rs            # 2D projection matrix table detection
│   ├── drawing.rs          # Vector path / drawings extractor
│   ├── image.rs            # Embedded image stream extraction & file sidecar
│   └── engine/             # ISO 32000-1 parser (lexer, xref, font cmap, content stream)
│       ├── doc.rs          # PurePdfDoc, PageInfo, CropBox & MediaBox hierarchy
│       └── content.rs      # ContentInterpreter, state stack, and glyph transforms
├── bindings/
│   ├── python/             # PyPI package (rasgapdf) + LangChain & LlamaIndex integrations
│   ├── nodejs/             # npm package (rasgapdf) powered by koffi
│   ├── csharp/             # NuGet package (RasgaPdf) for .NET 8
│   └── go/                 # Go module with cgo bindings
├── space_hf/               # Interactive WebAssembly visualizer & Hugging Face Space
├── docs/                   # GitHub Pages web app & Diátaxis documentation
├── tests/
│   ├── extraction.rs       # Rust integration test suite
│   ├── coordinate_contract.rs # Dedicated CropBox, rotation & chunking contract tests
│   └── test_integrations.py# 28-case Python benchmark & verification battery
└── .github/workflows/
    ├── ci.yml              # Multi-OS matrix continuous integration
    └── release.yml         # Automated multi-platform build matrix & registry release
```

---

## 3. The Coordinate Contract & Geometric Normalization

When editing or extending the geometry pipeline in `src/engine/doc.rs`, `src/engine/content.rs`, or `src/types.rs`, adhere strictly to these rules:

### A. CropBox Precedence (ISO 32000-1 §14.11.2)
* The visible viewport is defined by `/CropBox`.
* If `/CropBox` is missing, fall back to `/MediaBox`.
* Both attributes can be inherited from `/Parent` nodes in the page tree (`get_inherited_page_attribute`).

### B. Non-Zero Origin Normalization
PDF coordinates originate in the bottom-left of user space. For non-zero origins (e.g. `CropBox [36, 54, 576, 756]`):
$$\text{width} = \max(x_0, x_1) - \min(x_0, x_1)$$
$$\text{height} = \max(y_0, y_1) - \min(y_0, y_1)$$
$$\text{norm\_x} = X_{\text{user}} - \min(x_0, x_1)$$
$$\text{norm\_y} = \max(y_0, y_1) - Y_{\text{user}}$$

This guarantees $(0, 0)$ is strictly the Top-Left corner of the visible screen.

### C. Rotation Handling (`/Rotate 0, 90, 180, 270`)
* **Default Output:** Bboxes remain in unrotated page space; `page.rotation` reports viewer orientation.
* **Post-Rotation Mode:** When `apply_rotation: true` (or `page.rotated()` / `doc.with_applied_rotation()`), bounding boxes are rotated clockwise into final display orientation:
  - **90°:** $x0' = H - y1$, $x1' = H - y0$, $y0' = x0$, $y1' = x1$ (width and height swap).
  - **180°:** $x0' = W - x1$, $x1' = W - x0$, $y0' = H - y1$, $y1' = H - y0$.
  - **270°:** $x0' = y0$, $x1' = y1$, $y0' = W - x1$, $y1' = W - x0$ (width and height swap).

### D. Natural Reading Order via Recursive XY-Cut
* **Algorithm:** When `reading_order: "natural"` (the default), `PurePdfDoc` sorts non-table spans using the **Recursive XY-Cut** algorithm.
* **Projection Cuts:** It projects whitespace histograms along the horizontal (Y-cut) axis to partition sections and paragraphs, and vertical (X-cut) gutters (width $\ge 16.0$ pt) to separate multi-column layouts and sidebars.
* **Benefit:** Solves the multi-column interleaving defect where left-column and right-column lines with identical vertical coordinates would otherwise be merged into the same block.

### E. Sub-Page VLM Triage (`is_complex_layout` & `complexity_score`)
* **Scoring Metric:** Each `SpatialBlock` and `SpatialChunk` is scored for geometric complexity $S(R) \in [0.0, 1.0]$ based on:
  1. Intersection with vector paths/drawings ($+0.25..0.55$).
  2. Overlapping text spans and collisions ($+0.20..0.40$).
  3. Irregular multi-span column fragmentation / borderless grids ($+0.25$).
* **Threshold:** If $S(R) \ge 0.45$, `is_complex_layout` is set to `true`.
* **VLM Cost Efficiency:** Downstream AI pipelines can crop **only** the flagged sub-page bounding boxes to send to Vision-Language Models (VLM), saving 80%+ image tokens compared to full-page 300 DPI rasterization.

### F. Native Model Context Protocol (MCP) Server
* **Subcommand:** `rasgapdf mcp` executes a high-performance JSON-RPC 2.0 stdio server with zero external dependencies.
* **Supported Tools:**
  - `parse_pdf`: Returns structured 2D spatial text blocks, tables, and exact bounding boxes.
  - `search_document`: Searches terms and returns matching spans with visual bounding boxes for citations.
  - `get_chunks`: Returns semantic chunks with constituent bounding boxes, token estimates, and complexity flags.
  - `get_triage`: Scans a PDF and returns the exact list of sub-page bounding boxes flagged as `is_complex_layout`.

### G. Sub-Character Glyph Coalescence & Dynamic Kerning Tolerance
* **Stream-Level Coalescence (`ContentInterpreter`):** PDFs generated by Chromium, Cairo, Skia, or ticketing engines often emit each letter with an isolated `Tj` + `Td` command. `emit_text_string` coalesces consecutive glyphs on the same baseline (`|Δy0| <= 3.0 pt`) when `gap` falls within `[-max_overlap, +char_spacing_limit]`, eliminating 80%+ redundant span allocations at the source.
* **Proportional Kerning & Intra-Word Thresholds:** Both `ContentInterpreter` and `cluster_touching_spans` compute `max_overlap = (font_size * 0.20).max(1.5)` and `char_spacing_limit = (font_size * 0.09).clamp(0.4, 1.35)`, preventing negative-kerning splits in large headings (e.g., `"LATAM"`, `"Termos"`) while preserving inter-word spaces across all modes (`extract_text_bytes`, `parse_raw`, and `parse_structured`).

---

## 4. Spatial RAG Chunking Contract

When generating retrieval units for AI pipelines (LangChain, LlamaIndex, vector databases):

1. **Multi-Rect Highlighting:** Never discard individual block/line bounding boxes. A chunk must carry `bboxes: Vec<BoundingBox>` alongside `overall_bbox: BoundingBox`.
2. **Deterministic Chunk Model:** Use `SpatialChunk`:
   ```rust
   pub struct SpatialChunk {
       pub chunk_id: usize,
       pub page_number: usize,
       pub text: String,
       pub bboxes: Vec<BoundingBox>,
       pub overall_bbox: BoundingBox,
       pub block_count: usize,
       pub token_estimate: usize,
   }
   ```
3. **Native Helper:** Call `doc.to_spatial_chunks(max_chars, overlap_blocks)` in Rust or `rasgapdf.to_spatial_chunks(doc, max_chars, overlap_blocks)` in Python.

---

## 5. Local Development & Quality Gates

Before pushing any commit, run all quality gates locally. **Zero regression policy is mandatory.**

### Gate 1: Rust Unit & Integration Tests
```bash
cargo test
```
*Must pass all unit tests and integration tests (`coordinate_contract.rs`, `extraction.rs`).*

### Gate 2: Local Native Build
```bash
cargo build --release
```
*Produces `target/release/rasgapdf.dll` (Windows) or `.so` (Linux) / `.dylib` (macOS).*

### Gate 3: WebAssembly Build (for Docs & Space)
```bash
cargo build --target wasm32-unknown-unknown --release
# Copy updated wasm to demo sites:
cp target/wasm32-unknown-unknown/release/rasgapdf.wasm docs/rasgapdf.wasm
cp target/wasm32-unknown-unknown/release/rasgapdf.wasm space_hf/rasgapdf.wasm
```

### Gate 4: Python Benchmark & Integration Battery (28 Cases)
```bash
# Sync release dll to python package
cp target/release/rasgapdf.dll bindings/python/rasgapdf/

# Run 28-case verification suite
python tests/test_integrations.py
```
*All 28 benchmark cases must pass (including memory safety, throughput, table matrix consistency, and spatial chunking).*

---

## 6. Multi-Platform Binary Matrix Architecture

Because native shared libraries (`*.so`, `*.dll`, `*.dylib`) are binary artifacts, **they are intentionally excluded from git via `.gitignore`**.

### The Cloud Build Matrix Workflow:
When a release tag (`vX.Y.Z`) is pushed:
1. GitHub Actions initiates `build-native-matrix` with 3 parallel native runners:
   - 🐧 **`ubuntu-latest`**: Compiles `librasgapdf.so` (Linux glibc x64).
   - 🪟 **`windows-latest`**: Compiles `rasgapdf.dll` (Windows PE x64).
   - 🍏 **`macos-latest`**: Compiles `librasgapdf.dylib` (Universal Fat Binary `ARM64` + `x86_64` via `lipo`).
2. Downstream publish jobs (`publish-npm`, `publish-pypi`, `publish-nuget`) download **all three binaries** and bundle them together:
   - **npm:** Includes `.so`, `.dll`, and `.dylib` directly in the package tarball.
   - **PyPI:** Bundles `.so`, `.dll`, and `.dylib` in the multi-platform wheel.
   - **NuGet:** Packages into `runtimes/linux-x64/`, `runtimes/win-x64/`, `runtimes/osx-x64/`, and `runtimes/osx-arm64/`.

---

## 7. Version Bumping & Release Checklist

When releasing a new version `X.Y.Z`, follow this exact checklist:

### Step 1: Synchronize Version Across Config Files
Update version string `X.Y.Z` across all 8 configuration files:
- [ ] `Cargo.toml`: `version = "X.Y.Z"`
- [ ] `bindings/python/pyproject.toml`: `version = "X.Y.Z"`
- [ ] `bindings/nodejs/package.json`: `"version": "X.Y.Z"`
- [ ] `bindings/nodejs/index.js`: version getter returns `'X.Y.Z'`
- [ ] `bindings/csharp/RasgaPdf.csproj`: `<Version>X.Y.Z</Version>`
- [ ] `docs/index.html`: Version badges and headers
- [ ] `space_hf/index.html`: Version badges and headers
- [ ] `llms-full.txt`: Version declarations

### Step 2: Update Cargo.lock
```bash
cargo check
```

### Step 3: Recompile Local Targets
```bash
cargo build --release
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/rasgapdf.wasm docs/
cp target/wasm32-unknown-unknown/release/rasgapdf.wasm space_hf/
```

### Step 4: Run Test Suites
```bash
cargo test
python tests/test_integrations.py
```

### Step 5: Git Commit & Tag
```bash
git add -A
git commit -m "feat(release): vX.Y.Z - <summary of changes>"
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git tag -a bindings/go/vX.Y.Z -m "Go module vX.Y.Z"
git push origin main
git push origin vX.Y.Z
git push origin bindings/go/vX.Y.Z
```

### Step 6: Monitor GitHub Actions Pipeline
Track the release workflow until all jobs report `success`:
```bash
gh run list --workflow=release.yml
gh run watch <run-id>
```

Verify published packages:
- **npm:** `npm view rasgapdf version`
- **PyPI:** `curl -s https://pypi.org/pypi/rasgapdf/json | jq .info.version`
- **crates.io:** `https://crates.io/crates/rasgapdf`
- **NuGet:** `https://www.nuget.org/packages/RasgaPdf/`
- **Hugging Face:** `https://huggingface.co/spaces/MartinsMessias/rasgapdf`

---

## 8. Autonomous Self-Updating Documentation Directive

All AI coding agents (and human contributors) interacting with this codebase are bound by the **Self-Updating Documentation Contract**:

1. **Immediate Synchronization:** Whenever any change is made to:
   - CLI flags, commands, or subcommands (`src/main.rs`).
   - Data models, fields, or serialization options (`src/types.rs`).
   - Parser options or defaults (`ParserOptions`).
   - Engine algorithms or coordinate handling (`src/engine/`).
2. **Mandatory Documentation Targets:** The agent must automatically inspect and synchronize:
   - `AGENTS.md` (Architecture, contracts, and checklists).
   - `README.md` (Features, CLI usage, benchmarks, and API examples).
   - `docs/SCHEMA.md` (JSON data model specifications).
   - `llms.txt` and `llms-full.txt` (AI context declarations).
   - Cross-platform bindings (`bindings/go/rasgapdf.go`, `bindings/csharp/RasgaPdf.cs`).
3. **Automated Verification:**
   - Run `cargo fmt -- --check` and `cargo clippy -- -D warnings`.
   - Run `cargo test` and `python tests/test_integrations.py`.
   - Verify that all documented CLI options match `rasgapdf --help`.
