# 📐 RasgaPDF Spatial JSON Schema Specification

RasgaPDF focuses strictly on **2D Spatial Layout and Geometric Analysis**. Instead of discarding positional coordinates or returning lossy text streams, it outputs a deterministic, strongly-typed JSON document representing the physical reality of the PDF canvas.

---

## Top-Level: `SpatialDocument`

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
| `parse_time_ms` | `float` | Exact native parsing latency in milliseconds |
| `pages` | `array<SpatialPage>` | Array of processed page objects |

---

## Page Level: `SpatialPage`

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

| Field | Type | Description |
| :--- | :--- | :--- |
| `page_number` | `integer` | 1-based index of the page |
| `width` | `float` | Physical page width in typographical points (1/72 inch) |
| `height` | `float` | Physical page height in points |
| `rotation` | `integer` | Page rotation angle (0, 90, 180, 270) |
| `text_blocks` | `array<SpatialBlock>` | Paragraphs and visual text groupings |
| `tables` | `array<SpatialTable>` | Tables detected via 2D spatial column alignment |
| `raw_span_count` | `integer` | Total number of distinct text spans on this page |

---

## Bounding Box: `BoundingBox`

All spatial coordinates use standard PDF coordinate system units (points):
- `(0, 0)` is the **top-left corner**.
- `x` increases to the right.
- `y` increases downward.

```json
{
  "x0": 50.0,
  "y0": 120.5,
  "x1": 320.0,
  "y1": 136.5,
  "width": 270.0,
  "height": 16.0
}
```

---

## Block Level: `SpatialBlock` & `SpatialLine`

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

## Table Level: `SpatialTable` & `SpatialCell`

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
