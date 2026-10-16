# 📐 RasgaPDF Spatial Schema Specification

RasgaPDF provides **two distinct extraction modes**:
1. **Mode 1: Raw Physical Coordinates (`RawSpatialDocument`)** — Flat, zero-overhead physical coordinates list `[page, x0, y0, x1, y1, text]` designed for maximum throughput (~3-8 ms).
2. **Mode 2: Hierarchical Structured Document (`SpatialDocument`)** — Geometric tree containing visual blocks, lines, spans, and automatic matrix table detection.

---

## Mode 1: Raw Physical Coordinates (`RawSpatialDocument`)

Returns a flat, unnested list of every physical text element rendered on the PDF canvas, without paragraph clustering or table heuristics overhead. Ideal for spatial RAG pipelines, 2D visualizers, OCR alignment, and Unix pipelines (`awk`/`grep`).

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
      "text": "The Revolution of Artificial Intelligence"
    }
  ]
}
```

### `RawCoordSpan` Fields:
| Field | Type | Description |
| :--- | :--- | :--- |
| `page` | `integer` | 1-based page index |
| `x0` | `float` | Left horizontal boundary (typographical points, 1/72 inch) |
| `y0` | `float` | Top vertical boundary (typographical points) |
| `x1` | `float` | Right horizontal boundary |
| `y1` | `float` | Bottom vertical boundary |
| `font_size` | `float` | Font size in points |
| `font_name` | `string` | Embedded PDF font name |
| `text` | `string` | Raw extracted text |

---

## Mode 2: Hierarchical Structured Document (`SpatialDocument`)

Reconstructs the visual hierarchy into visual paragraphs (`text_blocks`), text lines (`lines`), spans, and auto-detects matrix tables via column alignment.

```json
{
  "file_name": "sample_document.pdf",
  "total_pages": 3,
  "parse_time_ms": 12.45,
  "pages": [ ... ]
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `file_name` | `string` | Base name of the processed PDF file |
| `total_pages` | `integer` | Total number of pages processed |
| `parse_time_ms` | `float` | Native execution time in milliseconds |
| `pages` | `array<SpatialPage>` | List of processed page objects |

---

### Page Level: `SpatialPage`

```json
{
  "page_number": 1,
  "width": 595.0,
  "height": 842.0,
  "rotation": 0,
  "text_blocks": [ ... ],
  "tables": [ ... ],
  "raw_span_count": 142
}
```

---

### Block & Line Level: `SpatialBlock` & `SpatialLine`

```json
{
  "block_id": 1,
  "block_type": "text",
  "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
  "lines": [
    {
      "text": "Invoice #450091",
      "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
      "spans": [
        {
          "text": "Invoice #450091",
          "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
          "font_name": "Helvetica-Bold",
          "font_size": 16.0,
          "char_count": 15
        }
      ]
    }
  ]
}
```

---

### Table Level: `SpatialTable` & `SpatialCell`

```json
{
  "table_id": 1,
  "bbox": { "x0": 45.0, "y0": 200.0, "x1": 550.0, "y1": 380.0, "width": 505.0, "height": 180.0 },
  "col_count": 4,
  "row_count": 5,
  "headers": ["Item", "Quantity", "Unit Price", "Total"],
  "rows": [
    ["Item", "Quantity", "Unit Price", "Total"],
    ["Coffee Beans 1kg", "10", "$15.00", "$150.00"],
    ["Filter Paper", "5", "$4.00", "$20.00"]
  ],
  "cells": [
    {
      "row": 0,
      "col": 0,
      "text": "Item",
      "bbox": { "x0": 45.0, "y0": 200.0, "x1": 180.0, "y1": 220.0, "width": 135.0, "height": 20.0 }
    }
  ]
}
```
