import sys
import json
import rasgapdf

sys.stdout.reconfigure(encoding='utf-8')

doc = rasgapdf.parse_pdf("C:/Users/messi/projetos/rasgapdf/benchmarks/data/parse_bench/2026-q1-tables-only/pdfs/000.pdf", detect_tables=True)
page = doc["pages"][0]
print(f"Page 000: detected {len(page['tables'])} tables:")
for i, t in enumerate(page["tables"]):
    bbox = t.get("bbox", {})
    rows = t.get("rows", [])
    print(f"  Detected Table {i}: bbox y0={bbox.get('y0',0):.1f}, y1={bbox.get('y1',0):.1f} | rows={len(rows)}, cols={t.get('col_count')}")
    if rows:
        print(f"    Sample Row: {rows[0]}")

print("\n--- Text blocks on page 000 ---")
for b_idx, b in enumerate(page["text_blocks"]):
    lines = [l.get("text") for l in b.get("lines", [])]
    bbox = b.get("bbox", {})
    print(f"  Block {b_idx} [y0={bbox.get('y0',0):.1f}, y1={bbox.get('y1',0):.1f}]: {' // '.join(lines[:2])}")

gt = json.load(open("C:/Users/messi/projetos/rasgapdf/benchmarks/data/parse_bench/2026-q1-tables-only/ground_truth/000.json", encoding="utf-8"))
gt_tables = [x for x in gt if x.get("type") == "table"]
print(f"\nGround truth has {len(gt_tables)} tables:")
for j, gt_t in enumerate(gt_tables):
    lines = [l for l in gt_t["data"].split("\n") if l.strip()]
    print(f"  GT Table {j} ({gt_t.get('complexity')}): {lines[0]} -> {lines[1] if len(lines)>1 else ''}")
