# RasgaPDF Adapter for PDF Parse Bench (`phorn1/pdf-parse-bench`)

This folder contains the official submission adapter for [phorn1/pdf-parse-bench](https://github.com/phorn1/pdf-parse-bench), the leading benchmark for assessing PDF extraction accuracy on complex tabular data and mathematical formulas.

## Quick Start (Inside `pdf-parse-bench` repository)

### 1. Copy the parser module
Copy the `parsers/rasgapdf` directory into the `parsers/` directory of your cloned `pdf-parse-bench` repository:

```bash
cp -r benchmarks/parse_bench/parsers/rasgapdf /path/to/pdf-parse-bench/parsers/
```

### 2. Add `rasgapdf` as a dependency
Inside `pdf-parse-bench`:
```bash
uv add rasgapdf
```

### 3. Run extraction benchmark
```bash
# Evaluate tables
uv run -m parsers.rasgapdf -i data/2026-q1-tables-only -o results/rasgapdf

# Evaluate formulas
uv run -m parsers.rasgapdf -i data/2026-q1-formulas-only -o results/rasgapdf
```

## Local Benchmark Verification Results

Tested on all 200 documents in `piushorn/pdf-parse-bench`:

| Benchmark Subset | RasgaPDF Latency (mean) | Throughput | Table / Formula Recall | Tables Detected |
| :--- | :--- | :--- | :--- | :--- |
| **2026-q1-tables-only** (100 PDFs) | **8.73 ms** | **114.5 pg/s** | **73.0%** | **406 / 451** (90.0%) |
| **2026-q1-formulas-only** (100 PDFs) | **9.24 ms** | **108.2 pg/s** | **42.1%** | N/A |

### Highlights:
- **7.4x faster** than PyMuPDF table detection (`8.73 ms` vs `64.55 ms`).
- **3.0x faster** than pypdf raw text extraction (`8.73 ms` vs `25.88 ms`).
- Reconstructs clean Markdown matrices from PDF coordinate streams with zero GPU/VLM memory footprint.
