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
go get github.com/MartinsMessias/rasgapdf/bindings/go
```

*Requires the compiled `librasgapdf.so` / `rasgapdf.dll` in your library path or project root.*

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

### 🎯 Mode 2: Hierarchical Structured Document

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

	doc, err := rasgapdf.ParsePdf("invoice.pdf", options)
	if err != nil {
		log.Fatalf("Error: %v", err)
	}

	fmt.Printf("Parsed %d pages in %.2f ms\n", doc.TotalPages, doc.ParseTimeMs)
	for _, page := range doc.Pages {
		for _, block := range page.TextBlocks {
			for _, line := range block.Lines {
				fmt.Println(line.Text)
			}
		}
	}
}
```
