# 📐 RasgaPDF Spatial Schema Specification

RasgaPDF provides **two distinct extraction modes**:
1. **Mode 1: Raw Physical Coordinates (`RawSpatialDocument`)** — Flat, zero-overhead physical coordinates list `[page, x0, y0, x1, y1, text]` designed for maximum throughput (~3-8 ms).
2. **Mode 2: Hierarchical Structured Document (`SpatialDocument`)** — Geometric tree containing visual blocks, lines, spans, and automatic matrix table detection.

Both modes optionally include **embedded images** (`extract_images`) and **vector graphics** (`extract_drawings`). These two extras are **off by default**, so the JSON shape stays byte-for-byte compatible when they are not requested.

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
  "initial_rotation": 90,
  "coordinate_system": "viewer_oriented",
  "media_box": [ 0.0, 0.0, 595.0, 842.0 ],
  "crop_box": [ 36.0, 54.0, 576.0, 756.0 ],
  "text_blocks": [ ... ],
  "tables": [ ... ],
  "raw_span_count": 142,
  "images": [ ... ],
  "drawings": [ ... ]
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `page_number` | `integer` | 1-based page index |
| `width` | `float` | Page width in typographical points |
| `height` | `float` | Page height in typographical points |
| `rotation` | `integer` | Normalized viewer rotation (0 when `apply_rotation: true` to prevent double-rotation) |
| `initial_rotation` | `integer` | Original page `/Rotate` entry in the PDF catalog (0, 90, 180, 270) |
| `coordinate_system` | `string` | Coordinate space state: `"source"` or `"viewer_oriented"` |
| `media_box` | `array<float>?` | Declared physical MediaBox rectangle in points [x0, y0, x1, y1] |
| `crop_box` | `array<float>?` | Active visible CropBox rectangle in points [x0, y0, x1, y1] (takes precedence per ISO 32000-1) |
| `is_complex_layout` | `boolean` | Indicates visual/geometric layout complexity (diagrams, borderless grids) for VLM triage |
| `text_blocks` | `array<SpatialBlock>` | Visual paragraphs |
| `tables` | `array<SpatialTable>` | Auto-detected matrix tables |
| `raw_span_count` | `integer` | Number of raw text spans used for table detection |
| `images` | `array<SpatialImage>` | Embedded images. Empty unless the parser is created with `extract_images` |
| `drawings` | `array<SpatialDrawing>` | Vector paths. Empty unless the parser is created with `extract_drawings` |

---

### Semantic RAG Level: `SpatialChunk`

When building RAG pipelines, chunks group semantic blocks while preserving constituent bounding boxes for accurate visual citation highlighting:

```json
{
  "chunk_id": 1,
  "page_number": 1,
  "text": "Full concatenated semantic chunk text...",
  "bboxes": [
    { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
    { "x0": 50.0, "y0": 90.0, "x1": 310.0, "y1": 106.0, "width": 260.0, "height": 16.0 }
  ],
  "overall_bbox": { "x0": 50.0, "y0": 67.0, "x1": 310.0, "y1": 106.0, "width": 260.0, "height": 39.0 },
  "block_count": 2,
  "token_estimate": 128,
  "coordinate_system": "viewer_oriented"
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `chunk_id` | `integer` | 1-based sequential chunk identifier |
| `page_number` | `integer` | 1-based page index |
| `text` | `string` | Concatenated semantic text of all blocks in the chunk |
| `bboxes` | `array<BoundingBox>` | Exact individual bounding box per constituent text block (multi-quad highlight) |
| `overall_bbox` | `BoundingBox` | Enclosing bounding rectangle covering all blocks in the chunk |
| `block_count` | `integer` | Number of blocks in this chunk |
| `token_estimate` | `integer` | Approximate token count (`(chars + 3) // 4`) |
| `coordinate_system` | `string` | Coordinate space state matching the page (`"source"` or `"viewer_oriented"`) |
| `is_complex_layout` | `boolean` | Indicates visual/geometric layout complexity for selective VLM vision inspection |
| `complexity_score` | `float?` | Geometric complexity score [0.0..=1.0] (triage threshold: >= 0.45) |

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

---

### Image Level: `SpatialImage`

Only populated when the parser is created with `extract_images` (CLI: `--images`).

```json
{
  "image_id": 1,
  "name": "Im0",
  "xref": 7,
  "bbox": { "x0": 330.0, "y0": 442.0, "x1": 530.0, "y1": 592.0, "width": 200.0, "height": 150.0 },
  "pixel_width": 400,
  "pixel_height": 300,
  "bits_per_component": 8,
  "color_space": "/DeviceRGB",
  "filter": "DCTDecode",
  "mime_type": "image/jpeg",
  "byte_length": 8328,
  "sha256": "4488a57e35370bfa...",
  "file": "page_001_img_001_xref7.jpg"
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `image_id` | `integer` | 1-based index inside the page |
| `name` | `string` | XObject resource name of the image on the page |
| `xref` | `integer` | Cross-reference object number of the image |
| `bbox` | `BoundingBox \| null` | Placement rectangle in page coordinates (same system as text) |
| `pixel_width` | `integer` | Image width in pixels |
| `pixel_height` | `integer` | Image height in pixels |
| `bits_per_component` | `integer \| null` | Bit depth per colour component |
| `color_space` | `string \| null` | Raw PDF colour space name |
| `filter` | `string \| null` | Last stream filter (e.g. `DCTDecode`, `FlateDecode`) |
| `mime_type` | `string \| null` | MIME type when the stream is a self-contained image file |
| `byte_length` | `integer \| null` | Size in bytes of the original encoded stream |
| `sha256` | `string \| null` | SHA-256 (hex) of the original encoded stream |
| `file` | `string \| null` | File name written on disk (only filled by the CLI / `write_images_to_dir`) |

> **Pixel data is never embedded in the JSON.** Use the Rust API (`rasgapdf::extract_images`), the CLI (`--extract-images DIR`) or the FFI symbol (`rasgapdf_extract_images`) to get the original encoded streams.

> **Placement caveat:** the `bbox` comes from the text page's image block and is matched positionally against the page's image resources. Pages that paint the same image repeatedly, or that hide images behind clipping, may report an approximate `bbox`. The resource metadata (`xref`, size, filter, hash) is always exact.

---

### Vector Level: `SpatialDrawing` & `SpatialDrawingItem`

Only populated when the parser is created with `extract_drawings` (CLI: `--drawings`). Mirrors PyMuPDF's `Page.get_drawings()`.

```json
{
  "drawing_id": 2,
  "bbox": { "x0": 100.0, "y0": 292.0, "x1": 500.0, "y1": 292.0, "width": 400.0, "height": 0.0 },
  "drawing_type": "stroke",
  "item_count": 1,
  "items": [
    { "kind": "line", "p1": [100.0, 292.0], "p2": [500.0, 292.0] }
  ],
  "stroke_color": [1.0, 0.0, 0.0],
  "fill_color": null,
  "width": 4.0,
  "fill_opacity": null,
  "stroke_opacity": null,
  "even_odd": null,
  "dash_pattern": null,
  "dash_phase": null,
  "line_join": "miter",
  "line_cap": "butt",
  "layer": null,
  "seqno": 5
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `drawing_id` | `integer` | 1-based index inside the page (sequential after filtering) |
| `bbox` | `BoundingBox` | Union rectangle of all items |
| `drawing_type` | `string` | `fill`, `stroke` or `fill_stroke` |
| `item_count` | `integer` | Number of items in `items` |
| `items` | `array<SpatialDrawingItem>` | Geometry of the path, tagged by `kind` |
| `stroke_color` | `[float; 3] \| null` | Normalised RGB stroke colour |
| `fill_color` | `[float; 3] \| null` | Normalised RGB fill colour |
| `width` | `float \| null` | Stroke width in points |
| `fill_opacity` | `float \| null` | Fill alpha |
| `stroke_opacity` | `float \| null` | Stroke alpha |
| `even_odd` | `bool \| null` | Even-odd fill rule |
| `dash_pattern` | `array<float> \| null` | Dash lengths |
| `dash_phase` | `float \| null` | Dash phase |
| `line_join` | `string \| null` | `miter`, `round`, `bevel`, `miter_xps` |
| `line_cap` | `string \| null` | `butt`, `round`, `square`, `triangle` (or `start/end`) |
| `layer` | `string \| null` | Optional content group (OCG) name |
| `seqno` | `integer` | Paint order inside the page |

`SpatialDrawingItem` variants (discriminated by `kind`):

| `kind` | Payload |
| :--- | :--- |
| `line` | `p1`, `p2` |
| `curve` | `p1`, `p4` (endpoints) and `p2`, `p3` (control points) |
| `rect` | `bbox`, `orientation` (`1` anti-clockwise, `-1` clockwise) |
| `quad` | `points` (4 points: ul, ur, ll, lr) |

---

### Parser Options (`ParserOptions`)

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `mode` | `string` | `"structured"` | `structured` or `raw` |
| `reading_order` | `string` | `"natural"` | Reading order strategy: `natural` (Recursive XY-Cut) or `raw` |
| `apply_rotation` | `bool` | `false` | Normalize bboxes to rotated viewer orientation |
| `detect_tables` | `bool` | `true` | Enable matrix table detection |
| `min_table_columns` | `integer` | `3` | Minimum columns to form a table |
| `column_tolerance` | `float` | `16.0` | Horizontal alignment tolerance (points) |
| `row_tolerance` | `float` | `6.0` | Vertical row tolerance (points) |
| `extract_images` | `bool` | `false` | Collect embedded image metadata |
| `extract_drawings` | `bool` | `false` | Collect vector graphics |
| `filter_background_fills` | `bool` | `true` | Drop fills covering ≥95% of the page area |

The struct is `#[serde(default)]`, so partial JSON is accepted and unknown fields are ignored.
