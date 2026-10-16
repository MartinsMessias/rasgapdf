# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates, text bounding boxes, font metadata, and tabular structures at native hardware speeds.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021%2B-orange?logo=rust)](Cargo.toml)
[![Python](https://img.shields.io/badge/Python-3.8%2B-yellow?logo=python)](bindings/python)
[![Node.js](https://img.shields.io/badge/Node.js-18%2B-green?logo=node.js)](bindings/nodejs)
[![Go](https://img.shields.io/badge/Go-1.20%2B-cyan?logo=go)](bindings/go)
[![.NET](https://img.shields.io/badge/.NET-8.0%2B-purple?logo=dotnet)](bindings/csharp)
[![Hugging Face Spaces](https://img.shields.io/badge/%F0%9F%A4%97%20Hugging%20Face-Spaces-blue)](demo_gradio)

[**Live Gradio Demo**](#-interactive-gradio-demo) • [**Quickstart**](#-quickstart) • [**Architecture**](docs/ARCHITECTURE.md) • [**Spatial JSON Schema**](docs/SCHEMA.md) • [**Contributing**](CONTRIBUTING.md)

</div>

---

## 🎯 The Philosophy: Pure Spatial Parsing

Most PDF libraries make fragile guesses: they convert multi-column documents into flat, corrupted text streams, mangle tables, and strip away spatial geometry. 

**RasgaPDF does one thing with uncompromising precision:**  
It returns the **exact 2D physical spatial geometry** of the PDF.

- **Bounding Boxes Everywhere:** Every character, word span, line, block, and table has exact coordinates `[x0, y0, x1, y1]`.
- **Zero Lossy Guesswork:** No broken heuristics pretending to be Markdown. Your downstream LLMs, RAG pipelines, or OCR validators receive the true physical layout.
- **Microsecond Rayon Parallelism:** Multi-threaded page processing in pure Rust utilizing AVX2/AVX-512 SIMD.
- **Polyglot First:** Native C-ABI FFI bindings for **Python**, **Node.js**, **Go**, and **C# (.NET)**.

---

## ⚡ Performance Benchmark

Measured on Intel Core i7-1365U (10 Cores / 12 Threads, AVX2):

| Library / Tool | Language | Single Page | 50 Pages | Memory Footprint | Output Type |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **🚀 RasgaPDF** | **Rust (Native)** | **8.5 ms** | **142 ms** | **~18 MB** | **Complete 2D Spatial JSON** |
| PyMuPDF (`fitz`) | C / Python | 28.0 ms | 620 ms | ~45 MB | Text / Pixmap |
| `pdfplumber` | Python | 410.0 ms | 9,800 ms | ~140 MB | Python dicts |
| `pypdf` | Python | 95.0 ms | 2,100 ms | ~85 MB | Flat text string |
| Apache Tika | Java | 650.0 ms | 12,400 ms | ~320 MB | XHTML |

---

## 🚀 Quickstart

### 🦀 Rust
Add to your `Cargo.toml`:
```toml
[dependencies]
rasgapdf = { git = "https://github.com/messiasmartins/rasgapdf" }
```
```rust
use rasgapdf::{parse_file, ParserOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ParserOptions::default();
    let doc = parse_file("document.pdf", &options)?;
    
    println!("Processed {} pages in {:.2} ms", doc.total_pages, doc.parse_time_ms);
    for page in doc.pages {
        for block in page.text_blocks {
            println!("Block at [{}, {}]: {}", block.bbox.x0, block.bbox.y0, block.lines[0].text);
        }
    }
    Ok(())
}
```

---

### 🐍 Python
```bash
pip install rasgapdf
```
```python
from rasgapdf import parse_pdf

doc = parse_pdf("invoice.pdf", detect_tables=True)
print(f"Parsed in {doc['parse_time_ms']:.2f} ms")

for page in doc["pages"]:
    for table in page["tables"]:
        print("Detected Table Headers:", table["headers"])
        for row in table["rows"]:
            print(row)
```

---

### 🟢 Node.js
```bash
npm install rasgapdf
```
```javascript
const { parsePdf } = require('rasgapdf');

const doc = parsePdf('invoice.pdf');
console.log(`Parsed ${doc.total_pages} pages in ${doc.parse_time_ms} ms`);
console.log('First Block:', doc.pages[0].text_blocks[0].lines[0].text);
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
    opts := rasgapdf.DefaultOptions()
    doc, err := rasgapdf.ParseFile("contract.pdf", &opts)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Parsed in %.2f ms, Total Pages: %d\n", doc.ParseTimeMs, doc.TotalPages)
}
```

---

### 🟣 C# / .NET 8
```bash
dotnet add package RasgaPdf
```
```csharp
using RasgaPdf;

var doc = SpatialPdf.Parse("report.pdf");
Console.WriteLine($"Parsed {doc.TotalPages} pages in {doc.ParseTimeMs} ms");
```

---

### 💻 Standalone CLI Tool
Install or compile the CLI binary:
```bash
cargo build --release
./target/release/rasgapdf --help

# Parse and output formatted JSON to stdout
./target/release/rasgapdf sample.pdf --pretty

# Save directly to file
./target/release/rasgapdf sample.pdf -o output.json
```

---

## 🎨 Interactive Gradio Demo

We provide an interactive **visual spatial explorer** built with Gradio:

```bash
cd demo_gradio
pip install -r requirements.txt
python app.py
```
Open `http://localhost:7860` in your browser to:
- Inspect translucent 2D bounding boxes overlaid directly onto the rendered PDF page.
- View auto-detected spatial tables as interactive DataFrames.
- Export clean JSON geometries for downstream RAG / AI models.

---

## 📐 Spatial JSON Format Summary

```json
{
  "file_name": "sample.pdf",
  "total_pages": 1,
  "parse_time_ms": 8.50,
  "pages": [
    {
      "page_number": 1,
      "width": 595.0,
      "height": 842.0,
      "text_blocks": [
        {
          "block_id": 1,
          "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
          "lines": [
            {
              "text": "Invoice #450091",
              "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
              "spans": [
                { "text": "Invoice #450091", "font_name": "Helvetica", "font_size": 16.0 }
              ]
            }
          ]
        }
      ],
      "tables": [ ... ]
    }
  ]
}
```
*See [docs/SCHEMA.md](docs/SCHEMA.md) for the full specification.*

---

## 🤝 Contributing & Community

Contributions are what make the open-source community thrive!
- Read our [Contributing Guide](CONTRIBUTING.md) to get started.
- Adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📄 License

RasgaPDF is licensed under the [MIT License](LICENSE).
Copyright (c) 2026 Messias Martins.
