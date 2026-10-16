# ⚡ RasgaPDF

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser for Python.**  
*Extract exact physical coordinates `[x0, y0, x1, y1]`, word spans, text blocks, and tabular structures at native hardware speeds.*

[![PyPI version](https://img.shields.io/pypi/v/rasgapdf.svg?color=blue&logo=pypi&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![Python versions](https://img.shields.io/pypi/pyversions/rasgapdf.svg?logo=python&logoColor=white)](https://pypi.org/project/rasgapdf/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)

</div>

---

## 🚀 Why RasgaPDF?

Most Python PDF extraction libraries make fragile guesses: they flatten multi-column layouts into corrupted text streams, mangle tables, and strip away 2D positional geometry.

**RasgaPDF does one thing with uncompromising precision:**  
It returns the **exact 2D physical spatial geometry** of the PDF directly to Python:

- **Exact Bounding Boxes:** Every character, word span, line, block, and table has exact coordinates `[x0, y0, x1, y1]`.
- **Zero Lossy Heuristics:** No broken markdown assumptions. Downstream LLMs, RAG pipelines, or vision-language models receive the true physical layout.
- **Pure Native Speed:** Powered by a SIMD-parallelized Rust core. Extracts typical pages in **~3 to 8 ms**.
- **Self-Contained Wheel:** Pre-compiled native binaries bundled directly in the wheel. Zero external runtime dependencies.

---

## ⚡ Quick Benchmarks (Python)

Measured on Intel Core i7-1365U (Single-thread latency, 1-page complex PDF):

| Library | Parse Time | Output Type | Spatial Accuracy |
| :--- | :--- | :--- | :--- |
| **⚡ RasgaPDF (`parse_raw`)** | **~7.7 ms** | **Flat Coordinates `[x0, y0, x1, y1]`** | **Exact Physical Canvas** |
| **🚀 RasgaPDF (`parse_pdf`)** | **~8.5 ms** | **Hierarchical Blocks & Tables** | **Exact Physical Canvas** |
| PyMuPDF (`fitz`) | ~28.0 ms | Text / Pixmaps | Bounding Boxes |
| `pdfplumber` | ~410.0 ms | Python Dicts | High Overhead |
| `pypdf` | ~95.0 ms | Plain String | No Geometry |

---

## 📦 Installation

```bash
pip install --upgrade rasgapdf
```

---

## 💡 Usage

RasgaPDF offers two distinct modes for Python engineers:

### ⚡ Mode 1: Raw Physical Coordinates (`parse_raw`)
The fastest and most direct extraction possible. Returns a flat list of physical 2D coordinates for every text span on the page, with zero block/table overhead (~3-8 ms):

```python
import rasgapdf

# Parse raw physical coordinates
doc = rasgapdf.parse_raw("document.pdf")

print(f"Extracted {doc['total_spans']} spans in {doc['parse_time_ms']:.2f} ms")

for span in doc["spans"]:
    print(
        f"[Page {span['page']}] "
        f"bbox=({span['x0']:.2f}, {span['y0']:.2f}, {span['x1']:.2f}, {span['y1']:.2f}) "
        f"font={span['font_name']} ({span['font_size']}pt) "
        f"text=\"{span['text']}\""
    )
```

#### Output Structure:
```json
{
  "file_name": "document.pdf",
  "total_pages": 1,
  "total_spans": 20,
  "parse_time_ms": 7.78,
  "spans": [
    {
      "page": 1,
      "x0": 50.0,
      "y0": 67.0,
      "x1": 244.0,
      "y1": 83.0,
      "font_size": 12.0,
      "font_name": "Helvetica",
      "text": "The Future of Artificial Intelligence"
    }
  ]
}
```

---

### 🎯 Mode 2: Hierarchical Structured Document (`parse_pdf`)
Parses the visual hierarchy: paragraphs (`text_blocks`), text lines (`lines`), spans, and auto-detects matrix tables via column alignment:

```python
import rasgapdf

doc = rasgapdf.parse_pdf("invoice.pdf", detect_tables=True)

for page in doc["pages"]:
    print(f"--- Page {page['page_number']} ({page['width']}x{page['height']} pt) ---")
    
    # 1. Inspect Text Blocks
    for block in page["text_blocks"]:
        print(f"Block #{block['block_id']} at {block['bbox']}:")
        for line in block["lines"]:
            print(f"  Line: {line['text']}")

    # 2. Inspect Detected Tables
    for table in page["tables"]:
        print(f"Table ({table['col_count']} cols x {table['row_count']} rows):")
        print("Headers:", table["headers"])
        for row in table["rows"]:
            print("Row:", row)
```

---

## ⚙️ Parameters for `parse_pdf`

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `file_path` | `str` | *required* | Path to the PDF file on disk |
| `detect_tables` | `bool` | `True` | Automatically detect and reconstruct tables |
| `min_table_cols` | `int` | `3` | Minimum number of columns required to classify as a table |
| `col_tolerance` | `float` | `16.0` | Horizontal alignment threshold in typographical points |
| `row_tolerance` | `float` | `6.0` | Vertical line clustering threshold in points |

---

## 📄 License

MIT License. Copyright (c) 2026 Messias Martins.
