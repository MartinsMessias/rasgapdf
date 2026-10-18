# ⚡ RasgaPDF (Node.js)

<div align="center">

**Ultra-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables with sub-millisecond latency.*

[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=green&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

</div>

---

## 🚀 Installation

```bash
npm install rasgapdf
```

*Note: Bundles pre-compiled native Rust C-ABI binaries. Works out-of-the-box with optional `koffi` fast FFI and automatic CLI fallback.*

---

## ⚡ Two Extraction Modes

### ⚡ Mode 1: Flat Physical Coordinates (Raw X/Y)
Zero-overhead extraction returning exact physical 2D coordinates `[x0, y0, x1, y1]` for every text span (~3-8 ms):

```javascript
const { parseRaw } = require('rasgapdf');

const raw = parseRaw('sample.pdf');
console.log(`Parsed ${raw.total_spans} spans in ${raw.parse_time_ms.toFixed(2)} ms`);

// Iterate through raw physical coordinates
for (const span of raw.spans) {
  console.log(`[Page ${span.page}] (${span.x0}, ${span.y0}, ${span.x1}, ${span.y1}) -> "${span.text}"`);
}
```

### 🎯 Mode 2: Hierarchical Structured Document
Extracts visual text blocks, lines, font sizes, and automatically detects matrix tables via column alignment:

```javascript
const { parsePdf } = require('rasgapdf');

const doc = parsePdf('invoice.pdf', {
  detect_tables: true,
  min_table_columns: 3
});

console.log(`Total Pages: ${doc.total_pages}`);
for (const page of doc.pages) {
  // Access text blocks
  for (const block of page.text_blocks) {
    console.log(`Block #${block.block_id}: ${block.lines[0].text}`);
  }

  // Access detected tables
  for (const table of page.tables) {
    console.log('Table Headers:', table.headers);
    console.log('Table Rows:', table.rows);
  }
}
```

---

### 🖼️ Embedded Images & Vector Graphics (opt-in)

Both are **off by default**, so the JSON shape is unchanged unless you ask for them. Image bytes are never inlined: the tree carries metadata (dimensions, colour space, filter, SHA-256, placement `bbox`) and `extractImages()` writes the original streams to disk.

```javascript
const { parsePdf, extractImages } = require('rasgapdf');

const doc = parsePdf('report.pdf', {
  extract_images: true,
  extract_drawings: true
});

const page = doc.pages[0];

for (const image of page.images) {
  console.log(image.name, image.pixel_width, 'x', image.pixel_height, image.filter, image.bbox);
}

for (const drawing of page.drawings) {
  console.log(drawing.drawing_id, drawing.drawing_type, drawing.stroke_color);
}

// Writes page_001_img_001_xref7.jpg (etc.) plus manifest.json
const manifest = extractImages('report.pdf', './out');
console.log(`${manifest.written} of ${manifest.total_images} images written`);
```

---

## 📐 Schema

Returns a deterministic JSON structure representing the exact 2D canvas geometry of the PDF.

### `RawSpatialDocument`
```typescript
interface RawSpatialDocument {
  file_name: string;
  total_pages: number;
  total_spans: number;
  parse_time_ms: number;
  spans: Array<{
    page: number;
    x0: number;
    y0: number;
    x1: number;
    y1: number;
    font_size: number;
    font_name: string;
    text: string;
  }>;
}
```

### `SpatialDocument`
```typescript
interface SpatialDocument {
  file_name: string;
  total_pages: number;
  parse_time_ms: number;
  pages: Array<{
    page_number: number;
    width: number;
    height: number;
    rotation: number;          // real page /Rotate (0, 90, 180, 270)
    text_blocks: SpatialBlock[];
    tables: SpatialTable[];
    raw_span_count: number;
    images: SpatialImage[];    // empty unless extract_images: true
    drawings: SpatialDrawing[];// empty unless extract_drawings: true
  }>;
}

interface SpatialImage {
  image_id: number;
  name: string;
  xref: number;
  bbox: { x0: number; y0: number; x1: number; y1: number; width: number; height: number } | null;
  pixel_width: number;
  pixel_height: number;
  bits_per_component: number | null;
  color_space: string | null;
  filter: string | null;       // e.g. "DCTDecode"
  mime_type: string | null;    // e.g. "image/jpeg"
  byte_length: number | null;
  sha256: string | null;
  file: string | null;         // filled only by extractImages()
}

interface SpatialDrawing {
  drawing_id: number;
  bbox: { x0: number; y0: number; x1: number; y1: number; width: number; height: number };
  drawing_type: 'fill' | 'stroke' | 'fill_stroke';
  item_count: number;
  items: Array<{               // discriminated by `kind`
    kind: 'line' | 'curve' | 'rect' | 'quad';
    p1?: [number, number]; p2?: [number, number];
    p3?: [number, number]; p4?: [number, number];
    bbox?: { x0: number; y0: number; x1: number; y1: number; width: number; height: number };
    orientation?: number;      // 1 anti-clockwise, -1 clockwise
    points?: Array<[number, number]>;
  }>;
  stroke_color: [number, number, number] | null;
  fill_color: [number, number, number] | null;
  width: number | null;
  fill_opacity: number | null;
  stroke_opacity: number | null;
  even_odd: boolean | null;
  dash_pattern: number[] | null;
  dash_phase: number | null;
  line_join: string | null;
  line_cap: string | null;
  layer: string | null;
  seqno: number;
}
```

---

## 📄 License
GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later). Copyright (c) 2026 Messias Martins.

RasgaPDF links against [MuPDF](https://mupdf.com/) (Artifex Software), licensed
under the AGPL-3.0 — hence the same license here.
