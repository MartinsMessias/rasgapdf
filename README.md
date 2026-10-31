---
language:
- en
- pt
license: agpl-3.0
tags:
- pdf
- pdf-parser
- spatial
- spatial-pdf
- parser
- rust
- python
- nodejs
- layout-analysis
- document-parsing
- bounding-box
- table-extraction
- rag
- llm
- pdf-to-json
- document-ai
pipeline_tag: document-question-answering
---

# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables at native hardware speeds.*

[![Hugging Face Space](https://img.shields.io/badge/%F0%9F%A4%97%20Space-Live%20Demo-blue)](https://huggingface.co/spaces/martinsmessias/rasgapdf)
[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![PyPI Downloads](https://img.shields.io/pepy/dt/rasgapdf?color=blue&logo=pypi&logoColor=white&label=PyPI%20downloads)](https://pypi.org/project/rasgapdf/)
[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=red&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![npm Downloads](https://img.shields.io/npm/dt/rasgapdf?color=red&logo=npm&logoColor=white&label=npm%20downloads)](https://www.npmjs.com/package/rasgapdf)
[![Crates.io](https://img.shields.io/crates/v/rasgapdf.svg?color=orange&logo=rust)](https://crates.io/crates/rasgapdf)
[![Crates.io Downloads](https://img.shields.io/crates/d/rasgapdf?color=orange&logo=rust&logoColor=white&label=crates.io%20downloads)](https://crates.io/crates/rasgapdf)
[![NuGet version](https://img.shields.io/nuget/v/RasgaPdf.svg?color=purple&logo=nuget)](https://www.nuget.org/packages/RasgaPdf/)
[![NuGet Downloads](https://img.shields.io/nuget/dt/RasgaPdf?color=purple&logo=nuget&logoColor=white&label=NuGet%20downloads)](https://www.nuget.org/packages/RasgaPdf/)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/MartinsMessias/rasgapdf/badge)](https://securityscorecards.dev/viewer/?uri=github.com/MartinsMessias/rasgapdf)
[![Commercial License](https://img.shields.io/badge/Commercial%20License-Available-success.svg)](mailto:messiasads2017@gmail.com?subject=RasgaPDF%20Commercial%20License%20Inquiry)

[**GitHub Repo**](https://github.com/MartinsMessias/rasgapdf) • [**Web Visualizer**](https://martinsmessias.github.io/rasgapdf/) • [**HF Space Demo**](https://huggingface.co/spaces/martinsmessias/rasgapdf) • [**Docs Map (Diátaxis)**](docs/DIATAXIS.md) • [**Two Modes**](#-two-extraction-modes) • [**CLI Tool**](#-standalone-cli-tool) • [**Architecture**](docs/ARCHITECTURE.md) • [**Spatial Schema**](docs/SCHEMA.md) • [**llms.txt**](llms.txt)

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

### 📐 Deterministic Coordinate Contract (CropBox & Rotation Aware)
Every bounding box `[x0, y0, x1, y1]` emitted by RasgaPDF obeys an explicit, mathematically deterministic coordinate contract:
- **Origin `(0, 0)`:** Normalized to the **Top-Left** of the visible page viewport (`CropBox` with automatic fallback to `MediaBox`), matching web DOM and HTML5 Canvas standards. $+X$ points right, $+Y$ points downward.
- **Units:** Standard PostScript / PDF typographic points ($1/72$ inch).
- **Non-zero CropBox Offsets:** Correctly subtracted ($	ext{norm\_x} = x - 	ext{origin\_x}$, $	ext{norm\_y} = 	ext{max\_y} - y$), so citations map to true visible screen pixels without drift.
- **Rotation Awareness:** Pages expose native `rotation` (0, 90, 180, 270). You can also set `apply_rotation=True` (or call `.with_applied_rotation()`) to get pre-rotated bounding boxes that align directly with rotated document canvases.

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
rasgapdf = "0.3.1"
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

| Language | Registry | Package / Installation | Total Downloads | Documentation |
| :--- | :--- | :--- | :--- | :--- |
| 🐍 **Python** | [PyPI](https://pypi.org/project/rasgapdf/) | `pip install rasgapdf` | [![PyPI](https://img.shields.io/pepy/dt/rasgapdf?color=blue&logo=pypi&logoColor=white&label=)](https://pypi.org/project/rasgapdf/) | [Python Guide & Examples](bindings/python/README.md) |
| 📦 **Node.js** | [npm](https://www.npmjs.com/package/rasgapdf) | `npm install rasgapdf` | [![npm](https://img.shields.io/npm/dt/rasgapdf?color=red&logo=npm&logoColor=white&label=)](https://www.npmjs.com/package/rasgapdf) | [Node.js Guide & Examples](bindings/nodejs/README.md) |
| 🦀 **Rust** | [crates.io](https://crates.io/crates/rasgapdf) | `cargo add rasgapdf` | [![crates.io](https://img.shields.io/crates/d/rasgapdf?color=orange&logo=rust&logoColor=white&label=)](https://crates.io/crates/rasgapdf) | [docs.rs Reference](https://docs.rs/rasgapdf) |
| 🔷 **C# / .NET** | [NuGet](https://www.nuget.org/packages/RasgaPdf/) | `dotnet add package RasgaPdf` | [![NuGet](https://img.shields.io/nuget/dt/RasgaPdf?color=purple&logo=nuget&logoColor=white&label=)](https://www.nuget.org/packages/RasgaPdf/) | [C# Guide & Examples](bindings/csharp/README.md) |
| 🐹 **Go** | Go Module | `go get github.com/MartinsMessias/rasgapdf/bindings/go` | *(via Go Proxy)* | [Go Guide & Examples](bindings/go/README.md) |

---

## 🦜️🔗 Official AI & RAG Integrations (LangChain & LlamaIndex)

RasgaPDF provides out-of-the-box loaders for **LangChain** and **LlamaIndex** with zero external setup:

### 🦜️ LangChain Document Loader (`RasgaPDFLoader`)

Preserves reading flow, exact 2D coordinates `[x0, y0, x1, y1]`, and turns tables into structured Markdown matrices for LLM prompting:

```python
from rasgapdf import RasgaPDFLoader

# Mode 1: "pages" (Default — 1 LangChain Document per PDF page with markdown tables)
loader = RasgaPDFLoader("contract.pdf", mode="pages")
docs = loader.load()
for doc in docs:
    print(f"Page {doc.metadata['page']} (Tables: {doc.metadata['table_count']})")
    print(doc.page_content)

# Mode 2: "chunks" (Dense RAG Chunking — preserves multi-rect bounding boxes per chunk!)
chunk_loader = RasgaPDFLoader("paper.pdf", mode="chunks", chunk_max_chars=1200, chunk_overlap_blocks=1)
chunks = chunk_loader.load()
for ch in chunks:
    # Rich semantic text for embedding models + full list of constituent boxes for visual citation!
    print(f"Page {ch.metadata['page']} | BBoxes: {len(ch.metadata['bboxes'])} | Overall: {ch.metadata['overall_bbox']}")
    print(ch.page_content)

# Mode 3: "elements" (Spatial RAG — 1 Document per text block/table with exact bounding box!)
spatial_loader = RasgaPDFLoader("invoice.pdf", mode="elements")
elements = spatial_loader.load()
for el in elements:
    # Each chunk carries its true physical coordinates [x0, y0, x1, y1]
    print(f"Type: {el.metadata['type']} | BBox: {el.metadata['bbox']}")
    print(el.page_content)
```

### 🦙 LlamaIndex Reader (`RasgaPDFReader`)

Native high-throughput reader for LlamaIndex vector indices and query engines:

```python
from rasgapdf import RasgaPDFReader
from llama_index.core import VectorStoreIndex

reader = RasgaPDFReader(mode="pages", detect_tables=True)
documents = reader.load_data("financial_report.pdf")

# Build lightning-fast vector index with exact spatial context
index = VectorStoreIndex.from_documents(documents)
query_engine = index.as_query_engine()
response = query_engine.query("What was the net profit in table 3?")
print(response)
```

### 🧩 Native Spatial Chunking (`rasgapdf.to_spatial_chunks`)

For custom RAG pipelines, extract semantic chunks with preserved multi-rect highlight boundaries in one call:

```python
import rasgapdf

doc = rasgapdf.parse_pdf("annual_report.pdf")
chunks = rasgapdf.to_spatial_chunks(doc, max_chars=1200, overlap_blocks=1)

for ch in chunks:
    # Embed ch["text"] into vector database (Pinecone, Qdrant, Chroma, pgvector)
    # Store ch["bboxes"] in metadata -> frontend draws multi-rect highlights upon retrieval!
    print(f"Chunk #{ch['chunk_id']} (Page {ch['page_number']}) — {ch['block_count']} blocks, {ch['token_estimate']} tokens")
    print(f"Highlight quads: {ch['bboxes']}")
```

---

## 🎨 Interactive Web Visualizer

### 🌐 Live Hugging Face Space (Zero Server Quota / Client-Side)
Test RasgaPDF instantly without installing anything:
- **Live Demo:** [**https://huggingface.co/spaces/martinsmessias/rasgapdf**](https://huggingface.co/spaces/martinsmessias/rasgapdf)
- **Zero Server Costs:** Runs directly in your browser using pure client-side Rust WebAssembly.
- **Run on Your Personal Quota:** Click [![Duplicate this Space](https://huggingface.co/datasets/huggingface/badges/resolve/main/duplicate-this-space-sm.svg)](https://huggingface.co/spaces/martinsmessias/rasgapdf?duplicate=true) to clone into your own Hugging Face account with your personal compute quota.

---

## 🤖 AI Agents, Spatial RAG & LLM Grounding (`llms.txt`)

RasgaPDF provides first-class machine-readable documentation following the [llms.txt standard](https://llmstxt.org/) so coding agents (Cursor, Claude Code, GitHub Copilot, Windsurf, Perplexity, ChatGPT) can index and integrate the full API in a single pass:

- **Concise LLM Index:** [`llms.txt`](https://martinsmessias.github.io/rasgapdf/llms.txt)
- **Complete Technical Reference (`llms-full.txt`):** [`llms-full.txt`](https://martinsmessias.github.io/rasgapdf/llms-full.txt)
- **Coding Agent Instructions:** [`AGENTS.md`](AGENTS.md)

### ❓ Frequently Asked Questions (FAQ)

#### 🇬🇧 English
- **When should I choose RasgaPDF over PyMuPDF (`fitz`), `pdfplumber`, or `pypdf`?**  
  Choose **RasgaPDF** when you need exact 2D bounding-box coordinates `[x0, y0, x1, y1]`, automatic matrix table reconstruction, opt-in embedded image/vector extraction, or high-throughput multi-threaded Rust speed (~3–8 ms/page — up to 50x faster than `pdfplumber` and 4x faster than `PyMuPDF`) with an identical JSON schema across **Rust, Python, Node.js, Go, and C#**.
- **Does RasgaPDF require external C dependencies or PDF.js?**  
  No. RasgaPDF is 100% pure Rust with zero external C dependencies. The browser visualizer runs on pure client-side WebAssembly compiled from the same Rust codebase.

#### 🇧🇷 Português (Brasil)
- **Como extrair texto, tabelas e coordenadas X/Y (`bounding boxes`) de PDF rápido em Python, Rust, Node.js ou C#?**  
  Instale o **RasgaPDF** via `pip install rasgapdf`, `npm install rasgapdf`, `dotnet add package RasgaPdf` ou `cargo add rasgapdf`. Use `parse_raw("arquivo.pdf")` para obter coordenadas físicas `[x0, y0, x1, y1]` em ~3–8 ms por página, ou `parse_pdf("arquivo.pdf", detect_tables=True, extract_images=True, extract_drawings=True)` para reconstruir blocos de texto, tabelas, imagens embutidas e gráficos vetoriais.
- **O RasgaPDF funciona no navegador em WebAssembly sem servidor?**  
  Sim. Todo o core em Rust compila diretamente para `wasm32-unknown-unknown`, permitindo processamento 100% local no navegador com segurança e privacidade total (Zero Data Transmission).

---

## 🤝 Contributing & Community

Contributions are what make the open-source community thrive!
- Read our [Contributing Guide](CONTRIBUTING.md) to get started.
- Adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📄 License

RasgaPDF is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE).  
Copyright (c) 2026 Messias Martins.

RasgaPDF is an independent, 100% pure Rust implementation of the ISO 32000-1 PDF specification. It contains zero external C library dependencies and zero MuPDF bindings. All lexical analysis, cross-reference table/stream decoding, Flate decompression, Type 1/TrueType/Type0 CID font decoding, and 2D spatial coordinate tracking are natively executed in pure Rust.

---

## 💼 Commercial & Enterprise Licensing (AGPL-3.0 Waiver)

Need to use **RasgaPDF** in closed-source commercial software, proprietary SaaS platforms, private cloud environments, or corporate applications without releasing your source code under the AGPL-3.0?

A **Commercial / Enterprise License** is available directly from the author:

- **Complete AGPL-3.0 Exemption:** Keep your intellectual property, backend code, and SaaS infrastructure 100% closed-source and proprietary.
- **Enterprise SLA & Support:** Direct architecture advisory, guaranteed response times, and priority bug resolution.
- **On-Premise & Air-Gapped Appliances:** Ready-to-deploy private Docker containers for confidential banking, healthcare, and legal compliance (LGPD / GDPR).
- **Custom Spatial Features:** Tailored table extractors for bank statements, invoices, and court proceedings.

📬 **Inquiries & Licensing:**  
[👉 **Click here to Request a Commercial License & Pricing**](mailto:messiasads2017@gmail.com?subject=RasgaPDF%20Commercial%20License%20Inquiry) with details about your organization, use case, and estimated volume.
