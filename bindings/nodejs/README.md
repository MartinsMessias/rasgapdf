# ⚡ RasgaPDF (Node.js)

<div align="center">

**Ultra-Fast 2D Spatial PDF Parser powered by Pure Rust.**  
*Extract exact physical coordinates (Raw X/Y) or structured blocks & tables with sub-millisecond latency.*

[![npm version](https://img.shields.io/npm/v/rasgapdf.svg?color=green&logo=npm)](https://www.npmjs.com/package/rasgapdf)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

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

---

## 📄 License
MIT License. Copyright (c) 2026 Messias Martins.
