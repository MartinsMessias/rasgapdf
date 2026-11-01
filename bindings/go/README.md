# ⚡ RasgaPDF (Go)

<div align="center">

**The Blazing-Fast 2D Spatial PDF Parser powered by Pure Rust for Go.**  
*Extract exact physical coordinates `[x0, y0, x1, y1]`, word spans, text blocks, and tabular structures at native hardware speeds.*

[![Go Reference](https://pkg.go.dev/badge/github.com/MartinsMessias/rasgapdf/bindings/go.svg)](https://pkg.go.dev/github.com/MartinsMessias/rasgapdf/bindings/go)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

[**GitHub Repository**](https://github.com/MartinsMessias/rasgapdf) • [**Web Visualizer**](https://martinsmessias.github.io/rasgapdf/)

</div>

---

## 🚀 Installation

```bash
go get github.com/MartinsMessias/rasgapdf/bindings/go@v0.3.3
```

*Requires the compiled `librasgapdf.so` (Linux), `rasgapdf.dll` (Windows), or `librasgapdf.dylib` (macOS) in your system library path or executable directory.*

---

## ⚡ Quickstart

### ⚡ Mode 1: Flat Physical Coordinates (Raw X/Y)

```go
package main

import (
	"fmt"
	"log"
	"github.com/MartinsMessias/rasgapdf/bindings/go"
)

func main() {
	raw, err := rasgapdf.ParseRaw("document.pdf")
	if err != nil {
		log.Fatalf("Error: %v", err)
	}

	fmt.Printf("Extracted %d spans in %.2f ms\n", raw.TotalSpans, raw.ParseTimeMs)
	for _, span := range raw.Spans {
		fmt.Printf("[Page %d] (%.2f, %.2f, %.2f, %.2f) \"%s\"\n",
			span.Page, span.X0, span.Y0, span.X1, span.Y1, span.Text)
	}
}
```

---

### 🎯 Mode 2: Hierarchical Structured Document & Tables

```go
package main

import (
	"fmt"
	"log"
	"github.com/MartinsMessias/rasgapdf/bindings/go"
)

func main() {
	options := rasgapdf.DefaultOptions()
	options.DetectTables = true
	options.MinTableColumns = 3
	options.ApplyRotation = true

	doc, err := rasgapdf.ParsePdf("invoice.pdf", &options)
	if err != nil {
		log.Fatalf("Error: %v", err)
	}

	fmt.Printf("Parsed %d pages in %.2f ms\n", doc.TotalPages, doc.ParseTimeMs)
	for _, page := range doc.Pages {
		fmt.Printf("--- Page %d (%.1f x %.1f pt) ---\n", page.PageNumber, page.Width, page.Height)
		for _, block := range page.TextBlocks {
			for _, line := range block.Lines {
				fmt.Printf("[%s] (%.1f, %.1f): %s\n", block.BlockType, line.BBox.X0, line.BBox.Y0, line.Text)
			}
		}

		for _, table := range page.Tables {
			fmt.Printf("Table #%d (%d rows x %d cols)\n", table.TableID, table.RowCount, table.ColCount)
			for _, row := range table.Rows {
				fmt.Println(row)
			}
		}
	}
}
```

---

### 🚀 Zero I/O Memory Extraction (Raw Bytes in RAM)

```go
data, err := os.ReadFile("document.pdf")
if err != nil {
    log.Fatal(err)
}

// Extract natural reading order text in RAM
text, err := rasgapdf.ExtractTextBytes(data)

// Extract raw coordinates directly from memory buffer
rawDoc, err := rasgapdf.ParseRawBytes(data)
```

---

## 📄 License & Commercial Options

- **Open Source:** Licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](https://www.gnu.org/licenses/agpl-3.0).
- **Commercial Exemption:** A commercial proprietary license is available for closed-source software without AGPL copyleft requirements. Contact: [messiasads2017@gmail.com](mailto:messiasads2017@gmail.com).
