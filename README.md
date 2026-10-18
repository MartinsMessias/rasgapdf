---
language:
- en
license: agpl-3.0
tags:
- pdf
- spatial
- parser
- rust
- layout-analysis
- document-parsing
- bounding-box
pipeline_tag: document-question-answering
---

# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables at native hardware speeds.*

[![Hugging Face Space](https://img.shields.io/badge/%F0%9F%A4%97%20Space-Live%20Demo-blue)](https://huggingface.co/spaces/martinsmessias/rasgapdf)
[![Duplicate this Space](https://huggingface.co/datasets/huggingface/badges/resolve/main/duplicate-this-space-md.svg)](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true)
[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Crates.io](https://img.shields.io/crates/v/rasgapdf.svg?color=orange&logo=rust)](https://crates.io/crates/rasgapdf)
[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=red&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![GitHub](https://img.shields.io/badge/GitHub-Repository-181717?logo=github)](https://github.com/MartinsMessias/rasgapdf)
[![Hugging Face Model](https://img.shields.io/badge/%F0%9F%A4%97%20Hub-Model%20Repo-blue)](https://huggingface.co/martinsmessias/rasgapdf)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)

[**GitHub Repo**](https://github.com/MartinsMessias/rasgapdf) • [**Live Space Demo**](https://huggingface.co/spaces/martinsmessias/rasgapdf) • [**Two Modes**](#-two-extraction-modes) • [**CLI Tool**](#-standalone-cli-tool) • [**Quickstart**](#-polyglot-quickstart) • [**Architecture**](docs/ARCHITECTURE.md) • [**Spatial Schema**](docs/SCHEMA.md)

</div>

---

## 🎯 The Philosophy: Pure Spatial Parsing

Most PDF extraction tools make fragile guesses: they flatten multi-column layouts into corrupted text streams, mangle tables, and strip away 2D positional geometry.

**RasgaPDF does one thing with uncompromising precision:**  
It returns the **exact 2D physical spatial geometry** of the PDF.

- **Bounding Boxes Everywhere:** Every character, word span, line, block, and table has exact coordinates `[x0, y0, x1, y1]`.
- **Zero Lossy Heuristics:** No broken markdown assumptions. Downstream LLMs, RAG pipelines, or vision-language models receive the true physical layout.
- **Microsecond Rayon Parallelism:** Multi-threaded page processing in pure Rust utilizing AVX2/AVX-512 SIMD.
- **Polyglot First:** Native C-ABI FFI bindings for **Python**, **Node.js**, **Go**, and **C# (.NET)**.

---

## ⚡ Two Extraction Modes

RasgaPDF offers two distinct modes for different engineering needs:

### ⚡ Mode 1: Flat Physical Coordinates (Raw X/Y)
The fastest, most direct extraction possible. Returns an unnested list of physical 2D coordinates for every text span on the page, with zero block or table overhead:
- **Latency:** ~3 to 8 ms per page
- **Supported Formats:** Flat JSON, Formatted Text, TSV, or CSV
- **Best For:** Spatial RAG, 2D visualizers, vector indexing, OCR alignment, Unix pipelines with `awk`/`grep`.

```
[Page 1] (X0:  50.00, Y0:  67.00, X1: 244.00, Y1:  83.00) [12.0pt] The Revolution of Artificial Intelligence
[Page 1] (X0:  50.00, Y0: 103.00, X1: 478.00, Y1: 119.00) [12.0pt] Artificial Intelligence (AI) has transformed...
```

### 🎯 Mode 2: Hierarchical Structured Document
Reconstructs the visual hierarchy into visual paragraphs (`text_blocks`), text lines (`lines`), spans, and auto-detects matrix tables via horizontal column projection:
- **Latency:** ~8 to 15 ms per page
- **Output:** Hierarchical tree `SpatialDocument` -> `pages` -> `text_blocks` -> `lines` -> `tables`.

---

## ⚡ Performance Benchmark

Measured on Intel Core i7-1365U (10 Cores / 12 Threads, AVX2):

| Library / Tool | Language | Single Page | 50 Pages | Memory Footprint | Output Type |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **⚡ RasgaPDF (Raw X/Y)** | **Rust (Native)** | **7.7 ms** | **95 ms** | **~14 MB** | **Flat Raw Coordinates** |
| **🚀 RasgaPDF (Structured)**| **Rust (Native)** | **8.5 ms** | **142 ms** | **~18 MB** | **Spatial Blocks & Tables** |
| PyMuPDF (`fitz`) | C / Python | 28.0 ms | 620 ms | ~45 MB | Text / Pixmap |
| `pdfplumber` | Python | 410.0 ms | 9,800 ms | ~140 MB | Python dicts |
| `pypdf` | Python | 95.0 ms | 2,100 ms | ~85 MB | Flat text string |
| Apache Tika | Java | 650.0 ms | 12,400 ms | ~320 MB | XHTML |

---

## 💻 Standalone CLI Tool

Install or build the standalone CLI binary:
```bash
cargo build --release
./target/release/rasgapdf --help

# ⚡ MODE 1: Raw Physical Coordinates (Raw X/Y)
./target/release/rasgapdf sample.pdf --raw
./target/release/rasgapdf sample.pdf --raw --format text
./target/release/rasgapdf sample.pdf --raw --format tsv | awk -F'\t' '$1 == 1 {print $2, $3, $8}'
./target/release/rasgapdf sample.pdf --raw --format csv -o coordinates.csv

# 🎯 MODE 2: Hierarchical Structured Document
./target/release/rasgapdf sample.pdf --pretty
./target/release/rasgapdf sample.pdf -o structured.json

# 🖼️ GRAPHICS (opt-in): embedded images and vector paths
./target/release/rasgapdf sample.pdf --images --drawings --pretty
./target/release/rasgapdf sample.pdf --extract-images ./out   # writes ./out/*.jpg + manifest.json
```

---

## 🚀 Rust Quickstart

Add `rasgapdf` to your `Cargo.toml`:

```toml
[dependencies]
rasgapdf = "0.1.3"
```

### ⚡ Mode 1: Raw Physical Coordinates (Raw X/Y)
Extract exact 2D bounding boxes `[x0, y0, x1, y1]` for every text span on the page with zero block overhead (~3 to 8 ms):

```rust
use rasgapdf::parse_raw_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = parse_raw_file("sample.pdf")?;
    println!("Extracted {} spans in {:.2} ms", raw.total_spans, raw.parse_time_ms);

    for span in &raw.spans {
        println!(
            "[Page {}] bbox=({:.2}, {:.2}, {:.2}, {:.2}) font={} ({:.1}pt) \"{}\"",
            span.page, span.x0, span.y0, span.x1, span.y1,
            span.font_name, span.font_size, span.text
        );
    }
    Ok(())
}
```

### 🎯 Mode 2: Hierarchical Structured Document
Reconstruct the visual hierarchy into visual paragraphs (`text_blocks`), text lines (`lines`), and auto-detect matrix tables:

```rust
use rasgapdf::{parse_file, ParserOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ParserOptions {
        detect_tables: true,
        min_table_columns: 3,
        column_tolerance: 16.0,
        row_tolerance: 6.0,
        ..Default::default()
    };

    let doc = parse_file("document.pdf", &options)?;
    println!("Processed {} pages in {:.2} ms", doc.total_pages, doc.parse_time_ms);

    for page in &doc.pages {
        println!("--- Page {} ({}x{} pt) ---", page.page_number, page.width, page.height);

        // 1. Text blocks and lines
        for block in &page.text_blocks {
            println!("Block #{} at {:?}", block.block_id, block.bbox);
            for line in &block.lines {
                println!("  Line: {}", line.text);
            }
        }

        // 2. Reconstructed tables
        for table in &page.tables {
            println!("Table ({} cols x {} rows):", table.col_count, table.row_count);
            println!("  Headers: {:?}", table.headers);
            for row in &table.rows {
                println!("  Row: {:?}", row);
            }
        }
    }
    Ok(())
}
```

---

### 🖼️ Embedded Images & Vector Graphics (opt-in)

Both extras are **off by default**, so the JSON shape is byte-for-byte unchanged unless you ask for them. Image *bytes* are never embedded in the JSON: metadata (dimensions, colour space, filter, SHA-256) stays in the tree, while the original encoded stream can be written to disk.

```rust
use rasgapdf::{extract_images, parse_file, write_images_to_dir, ParserOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ParserOptions {
        extract_images: true,
        extract_drawings: true,
        ..Default::default()
    };

    let doc = parse_file("document.pdf", &options)?;

    for page in &doc.pages {
        for image in &page.images {
            println!(
                "image {} on page {}: {}x{} px, filter={:?}, bbox={:?}",
                image.name, page.page_number, image.pixel_width, image.pixel_height,
                image.filter, image.bbox
            );
        }

        for drawing in &page.drawings {
            println!(
                "path {} on page {}: {} ({} items, stroke={:?})",
                drawing.drawing_id, page.page_number, drawing.drawing_type,
                drawing.item_count, drawing.stroke_color
            );
        }
    }

    // Original encoded streams (e.g. `.jpg` for DCTDecode) plus a `manifest.json`.
    let mut images = extract_images("document.pdf")?;
    let written = write_images_to_dir(&mut images, std::path::Path::new("./out"))?;
    println!("wrote {written} image files");

    Ok(())
}
```

Equivalent CLI flags: `--images`, `--drawings`, `--extract-images DIR`, and `--keep-backgrounds` (keeps full-page background fills, which are dropped by default). See [`docs/SCHEMA.md`](docs/SCHEMA.md) for the full field reference.

---

## 📦 Native Bindings for Other Ecosystems

RasgaPDF provides native C-ABI bindings for other languages, each published with its own language-specific package and documentation:

| Language | Registry | Package / Installation | Documentation |
| :--- | :--- | :--- | :--- |
| **Python** | [PyPI](https://pypi.org/project/rasgapdf/) | `pip install rasgapdf` | [Python Guide & Examples](bindings/python/README.md) |
| **Node.js** | [npm](https://www.npmjs.com/package/rasgapdf) | `npm install rasgapdf` | [Node.js Guide & Examples](bindings/nodejs/README.md) |
| **Go** | Go Module | `bindings/go` | [Go Bindings](bindings/go/) |
| **C# / .NET** | .NET Class Library | `bindings/csharp` | [C# Bindings](bindings/csharp/) |

---

## 🎨 Interactive Web Visualizer

### 🌐 Live Hugging Face Space (Zero Server Quota / Client-Side)
Test RasgaPDF instantly without installing anything:
- **Live Demo:** [**https://huggingface.co/spaces/martinsmessias/rasgapdf**](https://huggingface.co/spaces/martinsmessias/rasgapdf)
- **Zero Server Costs:** Runs directly in your browser using client-side WebAssembly & PDF.js.
- **Run on Your Personal Quota:** Click [![Duplicate this Space](https://huggingface.co/datasets/huggingface/badges/resolve/main/duplicate-this-space-sm.svg)](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true) to clone into your own Hugging Face account with your personal compute quota.

---

## 🤝 Contributing & Community

Contributions are what make the open-source community thrive!
- Read our [Contributing Guide](CONTRIBUTING.md) to get started.
- Adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📄 License

RasgaPDF is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE).  
Copyright (c) 2026 Messias Martins.

RasgaPDF links against [MuPDF](https://mupdf.com/) (Artifex Software), which is
distributed under the AGPL-3.0. As a derivative work, the whole project is licensed
under the same terms: if you distribute RasgaPDF — or expose it as a network service —
you must make the complete corresponding source available under the AGPL-3.0.
