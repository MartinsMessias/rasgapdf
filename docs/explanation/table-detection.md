# Deterministic 2D Table Reconstruction Algorithm

This document explains the geometric projection profile algorithm used by RasgaPDF to detect and reconstruct physical tables without neural networks or GPU latency.

---

## 1. Motivation: Why Pure Geometric Detection?
Vision-Language Models (VLMs) and deep learning detectors (such as Table-Transformer) suffer from:
1. **Severe Latency**: 500 ms to 3,000 ms per page.
2. **GPU Dependency**: Ingestion pipelines require CUDA hardware.
3. **Hallucination Risk**: Text tokens and decimal numbers can be scrambled or subtly altered.

RasgaPDF uses **deterministic 2D spatial clustering** operating in under **1 millisecond per page** on CPU.

---

## 2. Algorithmic Workflow

### Step 1: Vertical Row Clustering (Y-Band Splitting)
1. Spans on a page are ordered by baseline $Y_0$.
2. Spans sharing overlapping vertical intervals (within `row_tolerance`, default $6.0\text{ pt}$) are merged into candidate rows.

### Step 2: Horizontal Column Projection Profiles
1. For each candidate table region, column boundaries are calculated by projecting bounding boxes along the horizontal $X$ axis.
2. Spans whose horizontal gaps exceed `column_tolerance` (default $16.0\text{ pt}$) define column gutter boundaries.

### Step 3: Multi-Row Consistency & Filtering
1. Regions must exhibit at least `min_table_columns` (default 3 columns) to avoid false positives on simple two-column key-value lists or page margins.
2. Rows with matching column gutters are grouped into a `SpatialTable`.
3. Header rows and data cells are assigned discrete grid indexes $(r, c)$ and individual `SpatialCell` bounding boxes.

### Step 4: Markdown Matrix Generation
Tables are serialized into standard GitHub-flavored Markdown matrices:
```markdown
| Col 1 | Col 2 | Col 3 |
| --- | --- | --- |
| Val 1 | Val 2 | Val 3 |
```
This enables downstream LLMs and vector embeddings to ingest tabular relationships with structural clarity.
