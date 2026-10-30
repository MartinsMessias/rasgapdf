# How-to: Extract Tables to CSV and Pandas DataFrames

This guide demonstrates how to detect matrix tables in PDF documents using RasgaPDF's deterministic 2D column-projection engine and export them to **CSV files** and **pandas DataFrames** in Python, Node.js, and Rust.

Unlike heavy ML approaches (e.g. Table-Transformer or LayoutLM), RasgaPDF uses physical geometric alignments `[x0, y0, x1, y1]`, completing extraction in milliseconds with zero GPU overhead.

---

## Python Recipe: PDF Tables to CSV & Pandas

### 1. Requirements
Ensure you have `rasgapdf` and optionally `pandas` installed:
```bash
pip install rasgapdf pandas
```

### 2. Export Every Table to CSV via Standard Library
```python
import csv
import os
import rasgapdf

def extract_tables_to_csv(pdf_path: str, output_dir: str = "./tables"):
    os.makedirs(output_dir, exist_ok=True)
    
    # Run Mode 2 extraction with table detection enabled
    doc = rasgapdf.parse_pdf(pdf_path, detect_tables=True)
    
    total_tables = 0
    for page in doc["pages"]:
        page_num = page["page_number"]
        for table in page["tables"]:
            table_id = table["table_id"]
            csv_path = os.path.join(output_dir, f"page_{page_num}_table_{table_id}.csv")
            
            with open(csv_path, mode="w", newline="", encoding="utf-8") as f:
                writer = csv.writer(f)
                # Write header row if detected
                if table.get("headers"):
                    writer.writerow(table["headers"])
                # Write data rows
                for row in table["rows"]:
                    writer.writerow(row)
                    
            print(f"Exported: {csv_path} ({table['row_count']} rows x {table['col_count']} cols)")
            total_tables += 1
            
    print(f"Done! Extracted {total_tables} table(s) to '{output_dir}'.")

if __name__ == "__main__":
    extract_tables_to_csv("sample.pdf")
```

### 3. Load Tables Directly into Pandas DataFrames
```python
import pandas as pd
import rasgapdf

def pdf_tables_to_dataframes(pdf_path: str) -> list[pd.DataFrame]:
    doc = rasgapdf.parse_pdf(pdf_path, detect_tables=True)
    dataframes = []
    
    for page in doc["pages"]:
        for table in page["tables"]:
            headers = table.get("headers")
            rows = table.get("rows", [])
            
            if headers:
                df = pd.DataFrame(rows, columns=headers)
            else:
                df = pd.DataFrame(rows)
                
            # Attach spatial metadata as attributes
            df.attrs["page"] = page["page_number"]
            df.attrs["bbox"] = table["bbox"]
            df.attrs["table_id"] = table["table_id"]
            
            dataframes.append(df)
            
    return dataframes

# Example Usage
dfs = pdf_tables_to_dataframes("sample.pdf")
for i, df in enumerate(dfs, start=1):
    print(f"\n--- DataFrame #{i} (Page {df.attrs['page']}, BBox: {df.attrs['bbox']}) ---")
    print(df.head())
```

---

## Tuning Table Detection Parameters

In documents with narrow columns, loose row spacing, or 2-column tables, tune the geometric tolerance parameters:

| Parameter | Default | Purpose & Recommendation |
| :--- | :--- | :--- |
| `min_table_columns` | `3` | Minimum number of detected columns to form a table. Set to `2` for key-value tables. |
| `column_tolerance` | `16.0` pt | Horizontal tolerance for grouping text spans into vertical columns. Increase for wide gaps. |
| `row_tolerance` | `6.0` pt | Vertical tolerance for baseline alignment. Increase for double-spaced rows. |

### Python with Custom Options:
```python
doc = rasgapdf.parse_pdf(
    "sample.pdf",
    detect_tables=True,
    min_table_columns=2,   # allow 2-column key-value tables
    column_tolerance=20.0, # looser column grouping
    row_tolerance=8.0      # looser row grouping
)
```

---

## Node.js Recipe: Export Tables to CSV

```javascript
const fs = require('fs');
const path = require('path');
const { parsePdf } = require('rasgapdf');

function exportTablesToCsv(pdfPath, outDir = './tables') {
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  const doc = parsePdf(pdfPath, { detect_tables: true });

  for (const page of doc.pages) {
    for (const table of page.tables) {
      const fileName = `page_${page.page_number}_table_${table.table_id}.csv`;
      const filePath = path.join(outDir, fileName);

      const rows = [];
      if (table.headers && table.headers.length > 0) {
        rows.push(table.headers.map(cell => `"${cell.replace(/"/g, '""')}"`).join(','));
      }
      for (const row of table.rows) {
        rows.push(row.map(cell => `"${cell.replace(/"/g, '""')}"`).join(','));
      }

      fs.writeFileSync(filePath, rows.join('\n'), 'utf-8');
      console.log(`Saved: ${filePath}`);
    }
  }
}

exportTablesToCsv('sample.pdf');
```

---

## Rust Recipe: Table Extraction

```rust
use rasgapdf::{parse_file, ParserOptions};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ParserOptions {
        detect_tables: true,
        min_table_columns: 2,
        column_tolerance: 16.0,
        row_tolerance: 6.0,
        extract_images: false,
        extract_drawings: false,
        keep_background_fills: false,
    };

    let doc = parse_file("sample.pdf", &options)?;
    for page in &doc.pages {
        for table in &page.tables {
            let file_name = format!("page_{}_table_{}.csv", page.page_number, table.table_id);
            let mut file = File::create(&file_name)?;

            if !table.headers.is_empty() {
                writeln!(file, "{}", table.headers.join(","))?;
            }
            for row in &table.rows {
                writeln!(file, "{}", row.join(","))?;
            }
            println!("Exported table {} to {}", table.table_id, file_name);
        }
    }
    Ok(())
}
```

---

## Related Guides & References
- [Spatial Schema Reference](../reference/README.md) for exact `SpatialTable` and `SpatialCell` field descriptions.
- [Spatial RAG Coordinates](../how-to/spatial-rag-coordinates.md) for linking table cells back to visual PDF citations.
