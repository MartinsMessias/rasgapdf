#!/usr/bin/env python3
"""Forensic Analysis of Table Detection Gaps in RasgaPDF (406 detected vs 451 ground truth).

Identifies:
1. Which PDFs and ground-truth tables were not detected as tables.
2. The LaTeX structural attributes of missed tables:
   - Column count (e.g. 2 columns vs min_table_columns)
   - Row count (e.g. <= 2 rows)
   - Layout complexity (simple, moderate, complex)
   - LaTeX constructs (\multicolumn, \multirow, math symbols, table without borders)
3. How RasgaPDF processed the text of those missed tables (e.g., extracted as text_blocks).
"""

import sys
import json
import re
from pathlib import Path
from collections import Counter

# Ensure UTF-8 output
sys.stdout.reconfigure(encoding="utf-8")

import rasgapdf

BASE_DIR = Path("C:/Users/messi/projetos/rasgapdf/benchmarks/data/parse_bench/2026-q1-tables-only")
PDF_DIR = BASE_DIR / "pdfs"
GT_DIR = BASE_DIR / "ground_truth"


def count_latex_columns(tabular_code: str) -> int:
    """Estimates the column count from LaTeX \\begin{tabular}{...} definition."""
    match = re.search(r"\\begin\{tabular\}\s*\{([^}]+)\}", tabular_code)
    if not match:
        # Fallback: check maximum number of & in rows
        rows = tabular_code.split(r"\\")
        max_amps = max((r.count("&") for r in rows), default=0)
        return max_amps + 1 if max_amps > 0 else 1

    spec = match.group(1)
    # Strip optional column formatting like @{}, |*, etc.
    cols = re.findall(r"[lcrpmbX]", spec)
    return len(cols) if cols else 1


def count_latex_rows(tabular_code: str) -> int:
    """Counts rows by occurrences of \\."""
    rows = [r.strip() for r in tabular_code.split(r"\\") if r.strip() and not r.strip().startswith(r"\end")]
    return len(rows)


def analyze_misses():
    pdf_files = sorted(list(PDF_DIR.glob("*.pdf")))
    total_gt_tables = 0
    total_detected = 0

    pdf_diffs = []
    all_gt_tables = []
    
    # We will also test varying min_table_columns (e.g. min_table_cols=2)
    detected_with_min_cols_2 = 0

    for pdf_path in pdf_files:
        gt_file = GT_DIR / f"{pdf_path.stem}.json"
        with open(gt_file, "r", encoding="utf-8") as f:
            gt_data = json.load(f)

        gt_tables = [item for item in gt_data if item.get("type") == "table"]
        total_gt_tables += len(gt_tables)

        # Parse with default params
        doc = rasgapdf.parse_pdf(str(pdf_path), detect_tables=True)
        detected_count = sum(len(p.get("tables", [])) for p in doc.get("pages", []))
        total_detected += detected_count

        # Also test with min_table_cols=2
        doc_c2 = rasgapdf.parse_pdf(str(pdf_path), detect_tables=True, min_table_cols=2)
        c2_count = sum(len(p.get("tables", [])) for p in doc_c2.get("pages", []))
        detected_with_min_cols_2 += c2_count

        if detected_count != len(gt_tables):
            pdf_diffs.append({
                "pdf": pdf_path.name,
                "gt_count": len(gt_tables),
                "detected": detected_count,
                "diff": len(gt_tables) - detected_count,
                "gt_tables": gt_tables,
                "doc": doc
            })

    print(f"============================================================")
    print(f"  TABLE DETECTION FORENSIC AUDIT")
    print(f"============================================================")
    print(f"Total Ground Truth Tables: {total_gt_tables}")
    print(f"Detected (Default, min_cols=3): {total_detected} ({(total_detected/total_gt_tables)*100:.1f}%)")
    print(f"Detected (min_cols=2): {detected_with_min_cols_2} ({(detected_with_min_cols_2/total_gt_tables)*100:.1f}%)")
    print(f"Number of PDFs with count discrepancies: {len(pdf_diffs)}")

    # Let's inspect the ground truth tables across ALL 451 tables to see column distributions
    col_counts = Counter()
    row_counts = Counter()
    complexity_counts = Counter()

    missed_table_col_counts = Counter()
    missed_table_row_counts = Counter()
    missed_table_complexity = Counter()
    missed_table_samples = []

    # Let's check which specific GT tables were missed
    # For each PDF with discrepancy, we check if table tokens were absorbed into text_blocks
    absorbed_into_text_blocks = 0
    merged_tables = 0

    for item in pdf_diffs:
        pdf_name = item["pdf"]
        doc = item["doc"]
        page = doc.get("pages", [])[0] if doc.get("pages") else {}
        detected_tables = page.get("tables", [])
        text_blocks = page.get("text_blocks", [])

        # Extract text from detected tables
        detected_table_texts = []
        for t in detected_tables:
            t_str = " ".join(str(cell) for row in t.get("rows", []) for cell in (row if isinstance(row, list) else [row]))
            detected_table_texts.append(t_str)

        all_text_block_text = " ".join(
            line.get("text", "")
            for b in text_blocks
            for line in b.get("lines", [])
        )

        for gt_t in item["gt_tables"]:
            raw_latex = gt_t.get("data", "")
            cols = count_latex_columns(raw_latex)
            rows = count_latex_rows(raw_latex)
            comp = gt_t.get("complexity", "unknown")

            col_counts[cols] += 1
            row_counts[rows] += 1
            complexity_counts[comp] += 1

            # Check if this table was captured in one of the detected tables
            # Simple heuristic: find sample distinctive words from this GT table
            clean_words = [w for w in re.findall(r"[A-Za-z0-9]{4,}", raw_latex) if w not in {"begin", "tabular", "hline", "textbf", "midrule", "toprule", "bottomrule", "multicolumn", "multirow", "cline"}]
            
            matched_in_table = False
            for t_text in detected_table_texts:
                if clean_words:
                    matches = sum(1 for w in clean_words[:10] if w in t_text)
                    if matches >= max(2, len(clean_words[:10]) * 0.5):
                        matched_in_table = True
                        break

            if not matched_in_table:
                # This GT table was not in detected_tables!
                missed_table_col_counts[cols] += 1
                missed_table_row_counts[rows] += 1
                missed_table_complexity[comp] += 1

                # Check if it was absorbed into text blocks
                in_text_blocks = False
                if clean_words:
                    tb_matches = sum(1 for w in clean_words[:10] if w in all_text_block_text)
                    if tb_matches >= max(2, len(clean_words[:10]) * 0.5):
                        in_text_blocks = True
                        absorbed_into_text_blocks += 1

                if len(missed_table_samples) < 10:
                    missed_table_samples.append({
                        "pdf": pdf_name,
                        "cols": cols,
                        "rows": rows,
                        "complexity": comp,
                        "absorbed_into_text": in_text_blocks,
                        "latex_snippet": raw_latex[:250].replace("\n", " ")
                    })

    print(f"\n--- Distribution of ALL 451 GT Tables by Column Count ---")
    for c in sorted(col_counts.keys()):
        print(f"  {c} columns: {col_counts[c]} tables")

    print(f"\n--- Characteristics of Missed GT Tables ---")
    print(f"Total missed table instances analyzed: {sum(missed_table_col_counts.values())}")
    print(f"Missed by Column Count:")
    for c in sorted(missed_table_col_counts.keys()):
        print(f"  {c} columns: {missed_table_col_counts[c]} missed")

    print(f"\nMissed by Row Count:")
    for r in sorted(missed_table_row_counts.keys()):
        print(f"  {r} rows: {missed_table_row_counts[r]} missed")

    print(f"\nMissed by Complexity:")
    for comp, cnt in missed_table_complexity.items():
        print(f"  {comp}: {cnt} missed")

    print(f"\nAbsorbed into Text Blocks (Text preserved, but not tagged as table): {absorbed_into_text_blocks}")

    print(f"\n--- Sample Missed Tables ---")
    for s in missed_table_samples:
        print(f"PDF: {s['pdf']} | Cols: {s['cols']} | Rows: {s['rows']} | Complexity: {s['complexity']} | In Text Blocks: {s['absorbed_into_text']}")
        print(f"   LaTeX: {s['latex_snippet']}...")


if __name__ == "__main__":
    analyze_misses()
