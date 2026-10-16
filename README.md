# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables at native hardware speeds.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PyPI](https://img.shields.io/pypi/v/rasgapdf.svg?logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Rust](https://img.shields.io/badge/Rust-2021%2B-orange?logo=rust)](Cargo.toml)
[![Node.js](https://img.shields.io/badge/Node.js-18%2B-green?logo=node.js)](bindings/nodejs)
[![Go](https://img.shields.io/badge/Go-1.20%2B-cyan?logo=go)](bindings/go)
[![.NET](https://img.shields.io/badge/.NET-8.0%2B-purple?logo=dotnet)](bindings/csharp)
[![Hugging Face](https://img.shields.io/badge/%F0%9F%A4%97%20Hugging%20Face-martinsmessias%2Frasgapdf-blue)](https://huggingface.co/martinsmessias/rasgapdf)

[**Live Gradio Demo**](#-interactive-gradio-demo) • [**Quickstart**](#-quickstart) • [**Two Modes**](#-two-parsing-modes) • [**Architecture**](docs/ARCHITECTURE.md) • [**Spatial Schema**](docs/SCHEMA.md) • [**Contributing**](CONTRIBUTING.md)

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

## ⚡ Two Parsing Modes

RasgaPDF offers two distinct modes for different engineering needs:

### ⚡ Modo 1: Coordenadas Brutas (Raw X/Y Puro)
O retorno mais direto, bruto e rápido possível. Retorna exclusivamente a lista plana de coordenadas físicas de cada elemento desenhado na página, sem criar hierarquias de blocos ou overhead de tabelas:
- **Latência:** ~3 a 8 ms por página
- **Saídas suportadas:** JSON plano, Text formatado, TSV ou CSV
- **Uso ideal:** RAG espacial, visualizadores 2D, indexação vetorial, pipelines Unix com `awk`/`grep`.

```
[Pág. 1] (X0:  50.00, Y0:  67.00, X1: 244.00, Y1:  83.00) [12.0pt] A Revolução da Inteligência Artificial
[Pág. 1] (X0:  50.00, Y0: 103.00, X1: 478.00, Y1: 119.00) [12.0pt] A Inteligência Artificial (IA) tem transformado...
```

### 🎯 Modo 2: Estruturado Hierárquico
Processa a geometria e agrupa elementos em parágrafos visuais (`blocks`), linhas (`lines`), spans de fonte e detecta matrizes de tabelas automaticamente via projeção horizontal de colunas:
- **Latência:** ~8 a 15 ms por página
- **Saída:** Árvore hierárquica `SpatialDocument` -> `pages` -> `text_blocks` -> `lines` -> `tables`.

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

```bash
cargo build --release
./target/release/rasgapdf --help

# ⚡ MODO 1: Coordenadas Brutas (Raw X/Y)
./target/release/rasgapdf sample.pdf --raw
./target/release/rasgapdf sample.pdf --raw --format text
./target/release/rasgapdf sample.pdf --raw --format csv
./target/release/rasgapdf sample.pdf --raw --format tsv

# 🎯 MODO 2: Estruturado (Hierárquico com Blocos e Tabelas)
./target/release/rasgapdf sample.pdf --pretty
./target/release/rasgapdf sample.pdf -o output.json
```

---

## 🚀 Polyglot Quickstart

### 🐍 Python
```python
import rasgapdf

# ⚡ Modo 1: Coordenadas Brutas (Raw X/Y)
raw = rasgapdf.parse_raw("document.pdf")
print(f"Extraídos {raw['total_spans']} spans em {raw['parse_time_ms']:.2f} ms")
for s in raw["spans"]:
    print(f"Pág {s['page']} | ({s['x0']}, {s['y0']}) -> {s['text']}")

# 🎯 Modo 2: Estruturado (Hierárquico)
doc = rasgapdf.parse_pdf("document.pdf", detect_tables=True)
for page in doc["pages"]:
    for table in page["tables"]:
        print("Tabela detectada:", table["rows"])
```

### 🟢 Node.js
```javascript
const rasgapdf = require('rasgapdf');

// ⚡ Modo 1: Coordenadas Brutas
const raw = rasgapdf.parseRaw('document.pdf');
console.log(`Raw: ${raw.total_spans} spans em ${raw.parse_time_ms} ms`);

// 🎯 Modo 2: Estruturado
const doc = rasgapdf.parsePdf('document.pdf');
console.log(`Blocos de texto: ${doc.pages[0].text_blocks.length}`);
```

### 🔵 Go
```go
package main

import (
    "fmt"
    "github.com/messiasmartins/rasgapdf/bindings/go"
)

func main() {
    // ⚡ Modo 1: Coordenadas Brutas
    raw, _ := rasgapdf.ParseRaw("document.pdf")
    fmt.Printf("Raw: %d spans em %.2f ms\n", raw.TotalSpans, raw.ParseTimeMs)

    // 🎯 Modo 2: Estruturado
    opts := rasgapdf.DefaultOptions()
    doc, _ := rasgapdf.ParseFile("document.pdf", &opts)
    fmt.Printf("Estruturado: %d páginas\n", doc.TotalPages)
}
```

### 🟣 C# / .NET 8
```csharp
using RasgaPdf;

// ⚡ Modo 1: Coordenadas Brutas
var raw = SpatialPdf.ParseRaw("document.pdf");
Console.WriteLine($"Raw: {raw.TotalSpans} spans em {raw.ParseTimeMs} ms");

// 🎯 Modo 2: Estruturado
var doc = SpatialPdf.Parse("document.pdf");
Console.WriteLine($"Estruturado: {doc.TotalPages} páginas");
```

---

## 🎨 Interactive Gradio Demo

We provide an interactive **visual spatial explorer** built with Gradio:

```bash
cd demo_gradio
pip install -r requirements.txt
python app.py
```
Acesse `http://localhost:7860` para:
- Inspecionar a **tabela interativa de Coordenadas Físicas (Raw X/Y)**.
- Visualizar os **bounding boxes translúcidos** sobrepostos à página do PDF.
- Analisar **tabelas detectadas** renderizadas em DataFrames.
- Exportar JSONs brutos ou estruturados.

---

## 🤝 Contributing & Community

Contributions are what make the open-source community thrive!
- Read our [Contributing Guide](CONTRIBUTING.md) to get started.
- Adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📄 License

RasgaPDF is licensed under the [MIT License](LICENSE).  
Copyright (c) 2026 Messias Martins.
