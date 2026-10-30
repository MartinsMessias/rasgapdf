# How-to: Spatial RAG & Visual Bounding Box Citations

Standard Retrieval-Augmented Generation (RAG) pipelines extract flat, unformatted text from PDFs, discarding physical layout coordinates. Consequently:
1. LLM citations are limited to vague page numbers without pinpointing the exact passage.
2. Web UIs cannot render visual highlight overlays on top of the original PDF.
3. Multi-column documents suffer from scrambled reading orders and context contamination.

This recipe demonstrates how to use **RasgaPDF** to build **Spatial RAG Chunks** containing exact 2D physical bounding boxes (`[x0, y0, x1, y1]`) for ground-truth visual verification in Python and Node.js.

---

## 1. Understanding 2D Spatial Coordinates

RasgaPDF reports coordinates in PDF typographical points (1/72 inch):
- `x0`: Left boundary coordinate.
- `y0`: Top boundary coordinate (measured from the top edge of the page canvas).
- `x1`: Right boundary coordinate.
- `y1`: Bottom boundary coordinate.
- `page`: 1-indexed page number.

---

## 2. Python Recipe: Spatial Chunking with Bounding Boxes

### Step 1: Extract Paragraphs with Bounding Boxes
```python
import rasgapdf

def extract_spatial_chunks(pdf_path: str) -> list[dict]:
    """Extracts visual text blocks as RAG chunks with exact bounding boxes."""
    doc = rasgapdf.parse_pdf(pdf_path, detect_tables=True)
    chunks = []

    for page in doc["pages"]:
        page_num = page["page_number"]
        page_width = page["width"]
        page_height = page["height"]

        # 1. Text Paragraph Blocks
        for block in page["text_blocks"]:
            full_text = " ".join(line["text"] for line in block["lines"]).strip()
            if not full_text:
                continue

            chunks.append({
                "type": "text",
                "page": page_num,
                "text": full_text,
                "bbox": block["bbox"],  # {'x0', 'y0', 'x1', 'y1', 'width', 'height'}
                "normalized_bbox": [
                    block["bbox"]["x0"] / page_width,
                    block["bbox"]["y0"] / page_height,
                    block["bbox"]["x1"] / page_width,
                    block["bbox"]["y1"] / page_height,
                ],
                "page_dimensions": {"width": page_width, "height": page_height}
            })

        # 2. Table Blocks
        for table in page["tables"]:
            table_md = "| " + " | ".join(table["headers"]) + " |\n"
            table_md += "| " + " | ".join(["---"] * len(table["headers"])) + " |\n"
            for row in table["rows"]:
                table_md += "| " + " | ".join(row) + " |\n"

            chunks.append({
                "type": "table",
                "page": page_num,
                "text": table_md,
                "bbox": table["bbox"],
                "normalized_bbox": [
                    table["bbox"]["x0"] / page_width,
                    table["bbox"]["y0"] / page_height,
                    table["bbox"]["x1"] / page_width,
                    table["bbox"]["y1"] / page_height,
                ],
                "page_dimensions": {"width": page_width, "height": page_height}
            })

    return chunks
```

### Step 2: Indexing in Vector Databases (LangChain / LlamaIndex / Qdrant)
Store the spatial bounding box inside the vector metadata payload:

```python
from langchain_core.documents import Document

def to_langchain_documents(spatial_chunks: list[dict]) -> list[Document]:
    documents = []
    for chunk in spatial_chunks:
        doc = Document(
            page_content=chunk["text"],
            metadata={
                "page": chunk["page"],
                "chunk_type": chunk["type"],
                "bbox": chunk["bbox"],
                "normalized_bbox": chunk["normalized_bbox"],
            }
        )
        documents.append(doc)
    return documents

# Example
chunks = extract_spatial_chunks("sample.pdf")
docs = to_langchain_documents(chunks)
print(f"Created {len(docs)} spatial chunks ready for vector indexing.")
```

---

## 3. Highlighting Citations on the Web Frontend

When your LLM generates an answer referencing a retrieved chunk, pass `chunk.metadata.bbox` or `chunk.metadata.normalized_bbox` to your web frontend.

### Frontend Rendering Recipe (HTML5 Canvas / CSS Overlay):
```html
<div class="pdf-page-container" style="position: relative; width: 595px; height: 842px;">
  <!-- Render PDF page image or PDF.js canvas here -->
  <canvas id="pdf-render" width="595" height="842"></canvas>

  <!-- Bounding box highlight citation overlay -->
  <div id="citation-highlight" style="
    position: absolute;
    left: 50px;
    top: 67px;
    width: 194px;
    height: 16px;
    background-color: rgba(255, 235, 59, 0.4);
    border: 2px solid #fbc02d;
    border-radius: 2px;
    pointer-events: none;
    box-shadow: 0 0 8px rgba(251, 192, 45, 0.6);
  "></div>
</div>
```

If using normalized bounding boxes `[nx0, ny0, nx1, ny1]`:
```javascript
function applyNormalizedHighlight(overlayEl, [nx0, ny0, nx1, ny1], containerWidth, containerHeight) {
  overlayEl.style.left = `${nx0 * containerWidth}px`;
  overlayEl.style.top = `${ny0 * containerHeight}px`;
  overlayEl.style.width = `${(nx1 - nx0) * containerWidth}px`;
  overlayEl.style.height = `${(ny1 - ny0) * containerHeight}px`;
}
```

---

## 4. Ultra-Fast High-Throughput Token-Level Grounding (Mode 1)

For token-level or span-level citation precision, use Mode 1 (`parse_raw`), which parses documents in ~3–8 ms per page:

```python
import rasgapdf

# Flat list of text spans with exact bounds
raw_doc = rasgapdf.parse_raw("sample.pdf")

# Search for exact target query or keyword
query = "Revenue"
matches = [
    span for span in raw_doc["spans"]
    if query.lower() in span["text"].lower()
]

for m in matches:
    print(f"Match on Page {m['page']} at [{m['x0']}, {m['y0']}, {m['x1']}, {m['y1']}]: '{m['text']}'")
```

---

## Summary
By combining RasgaPDF's pure Rust speed with spatial coordinates, your RAG pipeline gains:
- **Pinpoint visual citations**: Users can click citations to jump directly to highlighted passages.
- **Zero hallucinations in tables**: Tabular structures are preserved as structured matrices, not broken strings.
- **Microsecond processing**: Native Rust speed keeps retrieval latency virtually unnoticeable.
