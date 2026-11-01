#!/usr/bin/env python3
"""Comprehensive Root Cause Breakdown of Why Table Count is 406 instead of 451."""

import sys
import json
import re
from pathlib import Path
from collections import Counter

sys.stdout.reconfigure(encoding="utf-8")

import rasgapdf

BASE_DIR = Path("C:/Users/messi/projetos/rasgapdf/benchmarks/data/parse_bench/2026-q1-tables-only")
PDF_DIR = BASE_DIR / "pdfs"
GT_DIR = BASE_DIR / "ground_truth"


def analyze_all_reasons():
    pdf_files = sorted(list(PDF_DIR.glob("*.pdf")))
    
    total_gt = 0
    total_detected = 0

    tables_1col_gt = 0
    tables_2col_gt = 0
    tables_side_by_side_merged = 0
    tables_absorbed_into_text = 0

    for pdf_path in pdf_files:
        gt_file = GT_DIR / f"{pdf_path.stem}.json"
        with open(gt_file, "r", encoding="utf-8") as f:
            gt_data = json.load(f)

        gt_tables = [item for item in gt_data if item.get("type") == "table"]
        total_gt += len(gt_tables)

        doc = rasgapdf.parse_pdf(str(pdf_path), detect_tables=True)
        detected_tables = sum(len(p.get("tables", [])) for p in doc.get("pages", []))
        total_detected += detected_tables

        # Check column specs in GT
        for gt_t in gt_tables:
            raw = gt_t.get("data", "")
            match = re.search(r"\\begin\{tabular\}\s*\{([^}]+)\}", raw)
            cols = len(re.findall(r"[lcrpmbX]", match.group(1))) if match else 1
            if cols == 1:
                tables_1col_gt += 1
            elif cols == 2:
                tables_2col_gt += 1

        # Check for wide side-by-side merged tables in RasgaPDF (col_count >= 8 or 9)
        for p in doc.get("pages", []):
            for t in p.get("tables", []):
                # A detected table with >= 8 columns in a 1-page synthetic PDF where GT had 4-5 col tables
                if t.get("col_count", 0) >= 8:
                    tables_side_by_side_merged += 1

    print(f"Total GT tables: {total_gt}")
    print(f"Total Detected by RasgaPDF (default min_cols=3): {total_detected}")
    print(f"Difference: {total_gt - total_detected}")
    print(f"\n--- Root Cause Breakdown ---")
    print(f"1. 1-Column 'Tables' in LaTeX GT: {tables_1col_gt} tables")
    print(f"   (LaTeX \\begin{{tabular}}{{c}} with single column; classified as text blocks by layout engine)")
    print(f"2. 2-Column Tables in LaTeX GT: {tables_2col_gt} tables")
    print(f"   (Classified as key-value text blocks under default min_table_cols=3)")
    print(f"3. Side-by-Side Merged Tables: ~{tables_side_by_side_merged} occurrences")
    print(f"   (Two independent tables placed in adjacent columns at identical Y positions merged into single wide matrix)")


if __name__ == "__main__":
    analyze_all_reasons()
