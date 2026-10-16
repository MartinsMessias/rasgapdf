# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables at native hardware speeds.*

[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Crates.io](https://img.shields.io/crates/v/rasgapdf.svg?color=orange&logo=rust)](https://crates.io/crates/rasgapdf)
[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=red&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![Hugging Face](https://img.shields.io/badge/%F0%9F%A4%97%20Hugging%20Face-martinsmessias%2Frasgapdf-blue)](https://huggingface.co/martinsmessias/rasgapdf)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

[**Live Gradio Demo**](#-interactive-gradio-demo) • [**Two Modes**](#-two-extraction-modes) • [**CLI Tool**](#-standalone-cli-tool) • [**Quickstart**](#-polyglot-quickstart) • [**Architecture**](docs/ARCHITECTURE.md) • [**Spatial Schema**](docs/SCHEMA.md)

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
```

---

## 🚀 Polyglot Quickstart

### 🦀 Rust
Add to your `Cargo.toml`:
```toml
[dependencies]
rasgapdf = "0.1.1"
```
```rust
use rasgapdf::{parse_raw_file, parse_file, ParserOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ⚡ Mode 1: Raw Physical Coordinates
    let raw = parse_raw_file("sample.pdf")?;
    println!("Extracted {} spans in {:.2} ms", raw.total_spans, raw.parse_time_ms);

    // 🎯 Mode 2: Hierarchical Structured Document
    let options = ParserOptions::default();
    let doc = parse_file("sample.pdf", &options)?;
    println!("Processed {} pages", doc.total_pages);
    Ok(())
}
```

---

### 🐍 Python
```bash
pip install --upgrade rasgapdf
```
```python
import rasgapdf

# ⚡ Mode 1: Raw Physical Coordinates
raw = rasgapdf.parse_raw("document.pdf")
print(f"Extracted {raw['total_spans']} spans in {raw['parse_time_ms']:.2f} ms")
for s in raw["spans"][:5]:
    print(f"[Page {s['page']}] ({s['x0']:.2f}, {s['y0']:.2f}) -> {s['text']}")

# 🎯 Mode 2: Hierarchical Structured Document
doc = rasgapdf.parse_pdf("document.pdf", detect_tables=True)
for page in doc["pages"]:
    for table in page["tables"]:
        print("Detected Table:", table["headers"], table["rows"])
```

---

### 🟢 Node.js
```bash
npm install rasgapdf
```
```javascript
const { parseRaw, parsePdf } = require('rasgapdf');

// ⚡ Mode 1: Raw Physical Coordinates
const raw = parseRaw('document.pdf');
console.log(`Raw: ${raw.total_spans} spans in ${raw.parse_time_ms.toFixed(2)} ms`);

// 🎯 Mode 2: Hierarchical Structured Document
const doc = parsePdf('document.pdf');
console.log(`Text Blocks: ${doc.pages[0].text_blocks.length}`);
```

---

### 🔵 Go
```bash
go get github.com/messiasmartins/rasgapdf/bindings/go
```
```go
package main

import (
    "fmt"
    "github.com/messiasmartins/rasgapdf/bindings/go"
)

func main() {
    // ⚡ Mode 1: Raw Physical Coordinates
    raw, _ := rasgapdf.ParseRaw("document.pdf")
    fmt.Printf("Raw: %d spans in %.2f ms\n", raw.TotalSpans, raw.ParseTimeMs)

    // 🎯 Mode 2: Hierarchical Structured Document
    opts := rasgapdf.DefaultOptions()
    doc, _ := rasgapdf.ParseFile("document.pdf", &opts)
    fmt.Printf("Structured: %d pages\n", doc.TotalPages)
}
```

---

### 🟣 C# / .NET 8
```csharp
using RasgaPdf;

// ⚡ Mode 1: Raw Physical Coordinates
var raw = SpatialPdf.ParseRaw("document.pdf");
Console.WriteLine($"Raw: {raw.TotalSpans} spans in {raw.ParseTimeMs:F2} ms");

// 🎯 Mode 2: Hierarchical Structured Document
var doc = SpatialPdf.Parse("document.pdf");
Console.WriteLine($"Structured: {doc.TotalPages} pages");
```

---

## 🎨 Interactive Gradio Demo

We provide an interactive **visual spatial explorer** built with Gradio:

```bash
cd demo_gradio
pip install -r requirements.txt
python app.py
```
Open `http://localhost:7860` to:
- Inspect the **Responsive Physical Coordinates Table (Raw X/Y)**.
- Read and instantly copy the **Pure Plain Text Stream & Coordinates Dump**.
- Visualize **translucent 2D bounding boxes** overlaid onto the PDF page.
- Analyze **detected tables** rendered in interactive DataFrames.
- Export raw or structured JSON geometries.

---

## 🤝 Contributing & Community

Contributions are what make the open-source community thrive!
- Read our [Contributing Guide](CONTRIBUTING.md) to get started.
- Adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📄 License

RasgaPDF is licensed under the [MIT License](LICENSE).  
Copyright (c) 2026 Messias Martins.
