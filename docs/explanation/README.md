# RasgaPDF Architecture & Engine Explanation

This section explains the design rationale, engineering philosophy, and architectural trade-offs behind RasgaPDF.

---

## 1. Primary Architecture Document

- **[System Architecture & Design Decisions](../ARCHITECTURE.md)**: High-level architectural flowchart, Rayon parallel pipeline details, C-ABI polyglot architecture, and memory footprint benchmarks.

---

## 2. Why Pure Rust? The ISO 32000-1 Engine Rationale

Historically, high-performance PDF tools relied on legacy C/C++ engines (such as Poppler, MuPDF, or PDFium). While capable of high-fidelity rendering, these engines carry significant operational and security liabilities:

### A. Memory Safety and CVE Exposure
PDF is one of the most notoriously complex file formats in computer history. C-based parsers have suffered from hundreds of buffer overflows, use-after-free bugs, integer overflows, and out-of-bounds reads when processing malformed or adversarial PDFs.
By implementing the ISO 32000-1 lexical parser, xref table resolver, and stream decompressor in **100% safe Rust**, RasgaPDF systematically eliminates entire classes of memory corruption vulnerabilities at compile time.

### B. True Multi-Threaded Concurrency
Many C/C++ libraries rely on global mutable state, non-reentrant allocators, or thread-local contexts that require heavy global locks or single-threaded execution.
In contrast, RasgaPDF takes advantage of Rust's `Send + Sync` type system:
- Pages are completely isolated, immutable data structures.
- Parsing utilizes `rayon::into_par_iter()`, distributing pages evenly across CPU cores without locks, mutexes, or Python Global Interpreter Lock (GIL) contention.
- Scaling is near-linear with physical CPU cores.

### C. Seamless WebAssembly (WASM) Compilation
C/C++ engines compiled to WASM via Emscripten frequently result in massive 10–30 MB binaries burdened by virtual POSIX filesystems and complex runtime glues.
Because RasgaPDF has zero external C dependencies and clean abstractions, it compiles directly to native `wasm32-unknown-unknown` at under 1 MB, enabling pure client-side PDF spatial extraction inside any browser.

---

## 3. The 2D Spatial Layout Philosophy

Traditional PDF text extractors collapse visual layouts into flat strings, attempting to guess headings, lists, and multi-column flows using arbitrary indentation thresholds. When applied to multi-column newspapers, academic papers, or financial invoices, these heuristics fail unpredictably.

RasgaPDF operates on a fundamental principle: **Preserve Exact Physical Geometry**.
- Every single glyph or word retains typographical coordinates: `[x0, y0, x1, y1]` in points (1/72 inch).
- Downstream LLM pipelines, RAG frameworks, and OCR engines receive ground truth coordinates rather than guesses.
- Document AI agents can project visual bounding boxes directly back onto rendered canvas images for visual grounding.

---

## 4. Deterministic Table Reconstruction vs. Heavy ML Models

Extracting tables with Vision-Language Models (VLMs) or deep-learning architectures (such as LayoutLM or Table-Transformer) presents severe production drawbacks:
- High latency: 500 ms to 3,000 ms per page.
- Expensive GPU hardware requirements.
- Stochastic hallucinations: numbers or decimal points occasionally altered.

RasgaPDF uses **deterministic 2D column-projection geometry**:
1. All text spans on a page are spatially sorted into vertical Y-buckets.
2. Spans aligned on baselines are tested for repetitive vertical column boundaries across multiple rows.
3. Once a region exhibits 3+ aligned columns with consistent horizontal gutters, it is extracted as a structured matrix table (`headers`, `rows`, `cells`).
4. This deterministic algorithm executes in **sub-millisecond time** on standard CPU hardware with zero hallucination risk.

---

## 5. Zero-Pixel Bloat Strategy

When documents contain high-resolution scans or photos, embedding raw image bytes directly inside JSON payloads causes catastrophic memory bloat (a 5 MB PDF can produce a 100 MB JSON string).

RasgaPDF decouples geometry from binary payloads:
- The JSON tree contains only lightweight metadata: width, height, color space, filter, byte length, SHA-256 digest, and placement `bbox`.
- Actual image streams are written to disk on-demand (`extract_images`) or returned as raw binary buffers in the native API.
- The resulting JSON remains tiny, fast to transfer over networks, and easy for LLMs to ingest.
