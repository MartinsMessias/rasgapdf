# Quickstart Tutorial: Your First 5 Minutes with RasgaPDF

Welcome to **RasgaPDF**! In this guided 5-minute tutorial, you will learn how to install RasgaPDF, parse a PDF document, and extract exact 2D physical bounding boxes (`[x0, y0, x1, y1]`) and structured tables.

By the end of this tutorial, you will have successfully extracted spatial layout coordinates in **Python**, **Node.js**, **Rust**, or the **CLI**.

---

## Prerequisites

You will need at least one of the following runtimes:
- **Python**: 3.8+ (`python3` and `pip`)
- **Node.js**: 18+ (`node` and `npm`)
- **Rust**: 1.75+ (`cargo`)
- Or pre-compiled CLI binary for Windows, Linux, or macOS.

Have a sample PDF handy (e.g. `sample.pdf`).

---

## 1. Python Walkthrough

### Step 1: Install `rasgapdf`
Run in your terminal:
```bash
pip install rasgapdf
```

### Step 2: Mode 1 — Extract Flat Physical Coordinates (Raw X/Y)
Create a file named `demo_raw.py`:

```python
import rasgapdf

# Mode 1: Fast flat physical coordinates [x0, y0, x1, y1] in typographics points (~3-8 ms)
raw_doc = rasgapdf.parse_raw("sample.pdf")

print(f"File: {raw_doc['file_name']} (Total Pages: {raw_doc['total_pages']})")
print(f"Execution time: {raw_doc['parse_time_ms']:.2f} ms")
print(f"Total Spans: {raw_doc['total_spans']}\n")

# Inspect the first 5 text spans
for span in raw_doc["spans"][:5]:
    print(f"Page {span['page']} | BBox: [{span['x0']:.1f}, {span['y0']:.1f}, {span['x1']:.1f}, {span['y1']:.1f}] | "
          f"Font: {span['font_name']} ({span['font_size']}pt) | Text: '{span['text']}'")
```

Run it:
```bash
python demo_raw.py
```

### Step 3: Mode 2 — Extract Structured Paragraphs & Tables
Create `demo_structured.py`:

```python
import rasgapdf

# Mode 2: Reconstruct visual hierarchy and auto-detect matrix tables
doc = rasgapdf.parse_pdf(
    "sample.pdf",
    detect_tables=True,
    extract_images=False,
    extract_drawings=False
)

for page in doc["pages"]:
    print(f"--- Page {page['page_number']} ({page['width']} x {page['height']} pt) ---")
    
    # Iterate through visual text blocks (paragraphs)
    for block in page["text_blocks"]:
        print(f"Block #{block['block_id']} at {block['bbox']}:")
        for line in block["lines"]:
            print(f"  Line: {line['text']}")
            
    # Iterate through detected tables
    for table in page["tables"]:
        print(f"\nFound Table #{table['table_id']} with {table['col_count']} columns:")
        print(f"Headers: {table['headers']}")
        for row in table["rows"]:
            print(f"Row: {row}")
```

Run it:
```bash
python demo_structured.py
```

---

## 2. Node.js Walkthrough

### Step 1: Install `rasgapdf`
```bash
npm install rasgapdf
```

### Step 2: Extract Coordinates and Tables
Create `index.js`:

```javascript
const { parseRaw, parsePdf, version } = require('rasgapdf');

console.log(`RasgaPDF Version: ${version()}`);

// Mode 1: Fast Raw Coordinates
const raw = parseRaw('sample.pdf');
console.log(`Parsed ${raw.total_spans} spans in ${raw.parse_time_ms} ms`);
console.log('First span:', raw.spans[0]);

// Mode 2: Structured Document
const doc = parsePdf('sample.pdf', {
  detect_tables: true,
  extract_images: false,
  extract_drawings: false
});

for (const page of doc.pages) {
  console.log(`Page ${page.page_number} has ${page.text_blocks.length} blocks and ${page.tables.length} tables.`);
  for (const table of page.tables) {
    console.log(`Table #${table.table_id} headers:`, table.headers);
  }
}
```

Run it:
```bash
node index.js
```

---

## 3. Rust Walkthrough

### Step 1: Add Dependency
In your `Cargo.toml`:
```toml
[dependencies]
rasgapdf = "0.3.1"
```

### Step 2: Parse File in Pure Rust
In your `src/main.rs`:

```rust
use rasgapdf::{parse_file, parse_raw_file, ParserOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Mode 1: Raw Physical Spans
    let raw = parse_raw_file("sample.pdf")?;
    println!("Total spans extracted: {}", raw.total_spans);
    if let Some(first) = raw.spans.first() {
        println!("First span: [{:.1}, {:.1}, {:.1}, {:.1}] '{}'", 
                 first.x0, first.y0, first.x1, first.y1, first.text);
    }

    // Mode 2: Structured Hierarchy with Default Options
    let options = ParserOptions {
        detect_tables: true,
        min_table_columns: 3,
        column_tolerance: 16.0,
        row_tolerance: 6.0,
        extract_images: false,
        extract_drawings: false,
        keep_background_fills: false,
    };

    let doc = parse_file("sample.pdf", &options)?;
    println!("Processed {} pages in {:.2} ms", doc.total_pages, doc.parse_time_ms);
    for page in &doc.pages {
        println!("Page {}: {} text blocks, {} tables", 
                 page.page_number, page.text_blocks.len(), page.tables.len());
    }

    Ok(())
}
```

Run it:
```bash
cargo run --release
```

---

## 4. Standalone CLI Binary

If you prefer the command line without writing code:

```bash
# Output flat coordinates to CSV
rasgapdf sample.pdf --raw --format csv -o coordinates.csv

# Output flat coordinates to TSV
rasgapdf sample.pdf --raw --format tsv

# Output structured hierarchical JSON with tables
rasgapdf sample.pdf --pretty -o output.json

# Opt-in embedded image and vector drawing extraction
rasgapdf sample.pdf --images --drawings --extract-images ./extracted_images/
```

---

## What Next?

Congratulations! You have completed the first run with RasgaPDF.

- **Need to convert detected tables into pandas DataFrames?** See [How-to: Extract Tables to CSV and DataFrames](../how-to/extract-tables-to-csv.md).
- **Building a Retrieval-Augmented Generation pipeline?** See [How-to: Spatial RAG Coordinates & Bounding Boxes](../how-to/spatial-rag-coordinates.md).
- **Want to run parsing directly in the browser with WebAssembly?** See [How-to: Pure Rust WebAssembly in Browser](../how-to/pure-rust-wasm.md).
- **Looking for exact schema and field specifications?** See [Reference: Spatial Schema Specification](../reference/README.md).
