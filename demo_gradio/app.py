import os
import sys
import json
import ctypes
import fitz
import gradio as gr
import pandas as pd
from PIL import Image, ImageDraw

# Load native RasgaPDF shared library
def get_lib():
    candidates = [
        os.path.join(os.path.dirname(__file__), "librasgapdf.so"),
        os.path.join(os.path.dirname(__file__), "..", "target", "release", "librasgapdf.so"),
        "/home/messias_martins/rasgapdf/target/release/librasgapdf.so"
    ]
    for c in candidates:
        if os.path.exists(c):
            lib = ctypes.CDLL(c)
            lib.rasgapdf_parse_raw.argtypes = [ctypes.c_char_p]
            lib.rasgapdf_parse_raw.restype = ctypes.c_void_p

            lib.rasgapdf_parse_file_options.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            lib.rasgapdf_parse_file_options.restype = ctypes.c_void_p

            lib.rasgapdf_free_string.argtypes = [ctypes.c_void_p]
            lib.rasgapdf_free_string.restype = None

            lib.rasgapdf_version.argtypes = []
            lib.rasgapdf_version.restype = ctypes.c_char_p
            return lib
    return None

_lib = get_lib()

def parse_raw_pdf(file_path):
    """Mode 1: Pure Flat Physical 2D Coordinates (Raw X/Y)"""
    if not _lib:
        raise RuntimeError("RasgaPDF native library not found")
    path_b = os.path.abspath(file_path).encode("utf-8")
    res_ptr = _lib.rasgapdf_parse_raw(path_b)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native library")
    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        return json.loads(raw_json)
    finally:
        _lib.rasgapdf_free_string(res_ptr)

def parse_spatial_pdf(file_path, detect_tables=True, min_cols=3, col_tol=16.0, row_tol=6.0):
    """Mode 2: Hierarchical Structured Document (Blocks, Lines & Tables)"""
    if not _lib:
        raise RuntimeError("RasgaPDF native library not found")
    
    opts = {
        "mode": "structured",
        "detect_tables": detect_tables,
        "min_table_columns": int(min_cols),
        "column_tolerance": float(col_tol),
        "row_tolerance": float(row_tol),
    }

    path_b = os.path.abspath(file_path).encode("utf-8")
    opts_b = json.dumps(opts).encode("utf-8")

    res_ptr = _lib.rasgapdf_parse_file_options(path_b, opts_b)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native library")

    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        return json.loads(raw_json)
    finally:
        _lib.rasgapdf_free_string(res_ptr)

def render_page_with_overlay(pdf_path, page_num, spatial_doc, show_text=True, show_tables=True):
    doc = fitz.open(pdf_path)
    if page_num < 1 or page_num > len(doc):
        page_num = 1
    
    page = doc[page_num - 1]
    pix = page.get_pixmap(dpi=150)
    img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)

    if not spatial_doc or "pages" not in spatial_doc or len(spatial_doc["pages"]) < page_num:
        return img

    page_data = spatial_doc["pages"][page_num - 1]
    scale_x = pix.width / max(page_data.get("width", 595.0), 1.0)
    scale_y = pix.height / max(page_data.get("height", 842.0), 1.0)

    # Create translucent overlay
    overlay = Image.new("RGBA", img.size, (255, 255, 255, 0))
    draw = ImageDraw.Draw(overlay)

    # 1. Text blocks & lines overlay
    if show_text and "text_blocks" in page_data:
        for b in page_data["text_blocks"]:
            bx0 = b["bbox"]["x0"] * scale_x
            by0 = b["bbox"]["y0"] * scale_y
            bx1 = b["bbox"]["x1"] * scale_x
            by1 = b["bbox"]["y1"] * scale_y

            # Cyan translucent block bounding box
            draw.rectangle([bx0, by0, bx1, by1], fill=(0, 240, 255, 25), outline=(0, 200, 240, 180), width=2)

            for l in b.get("lines", []):
                lx0 = l["bbox"]["x0"] * scale_x
                ly0 = l["bbox"]["y0"] * scale_y
                lx1 = l["bbox"]["x1"] * scale_x
                ly1 = l["bbox"]["y1"] * scale_y
                draw.rectangle([lx0, ly0, lx1, ly1], outline=(0, 150, 255, 120), width=1)

    # 2. Detected tables overlay
    if show_tables and "tables" in page_data:
        for t in page_data["tables"]:
            tx0 = t["bbox"]["x0"] * scale_x
            ty0 = t["bbox"]["y0"] * scale_y
            tx1 = t["bbox"]["x1"] * scale_x
            ty1 = t["bbox"]["y1"] * scale_y

            # Emerald green border for tables
            draw.rectangle([tx0, ty0, tx1, ty1], fill=(16, 185, 129, 35), outline=(16, 185, 129, 240), width=3)

            for cell in t.get("cells", []):
                cx0 = cell["bbox"]["x0"] * scale_x
                cy0 = cell["bbox"]["y0"] * scale_y
                cx1 = cell["bbox"]["x1"] * scale_x
                cy1 = cell["bbox"]["y1"] * scale_y
                draw.rectangle([cx0, cy0, cx1, cy1], outline=(16, 185, 129, 140), width=1)

    img = img.convert("RGBA")
    combined = Image.alpha_composite(img, overlay)
    return combined.convert("RGB")

def process_pdf(pdf_file, page_num, detect_tables, min_cols, col_tol, row_tol, show_text, show_tables):
    empty_df = pd.DataFrame()
    if not pdf_file:
        return None, empty_df, "", empty_df, {}, {}, "⚠️ Please upload a PDF file to begin analysis."

    file_path = pdf_file.name if hasattr(pdf_file, "name") else str(pdf_file)

    try:
        raw_doc = parse_raw_pdf(file_path)
        spatial_doc = parse_spatial_pdf(file_path, detect_tables, min_cols, col_tol, row_tol)
    except Exception as e:
        return None, empty_df, f"Error: {e}", empty_df, {"error": str(e)}, {}, f"❌ Spatial extraction error: {e}"

    total_pages = spatial_doc.get("total_pages", 1)
    page_idx = min(max(int(page_num), 1), total_pages)

    overlay_img = render_page_with_overlay(file_path, page_idx, spatial_doc, show_text, show_tables)

    # 1. Raw Coordinates Table (Clean & Responsive)
    raw_spans = raw_doc.get("spans", [])
    if raw_spans:
        page_spans = [s for s in raw_spans if s["page"] == page_idx]
        if not page_spans:
            page_spans = raw_spans

        raw_records = []
        for s in page_spans:
            raw_records.append({
                "Page": s["page"],
                "X0": round(s["x0"], 2),
                "Y0": round(s["y0"], 2),
                "X1": round(s["x1"], 2),
                "Y1": round(s["y1"], 2),
                "Size": round(s["font_size"], 1),
                "Font": s["font_name"],
                "Extracted Text": s["text"]
            })
        raw_df = pd.DataFrame(raw_records)
    else:
        raw_df = pd.DataFrame({"Status": ["No raw coordinate spans found."]})

    # 2. Pure Plain Text Views (Clean Stream & Coordinate Dump)
    plain_text_lines = []
    plain_text_lines.append(f"# RasgaPDF Plain Text Stream - Page {page_idx} of {total_pages}\n")
    plain_text_lines.append("=" * 70)
    plain_text_lines.append("--- [CLEAN TEXT FLOW] ---")
    if raw_spans:
        page_spans_for_text = [s for s in raw_spans if s["page"] == page_idx]
        for s in page_spans_for_text:
            plain_text_lines.append(s["text"])

        plain_text_lines.append("\n" + "=" * 70)
        plain_text_lines.append("--- [RAW COORDINATES DUMP] ---")
        for s in page_spans_for_text:
            plain_text_lines.append(
                f"[P.{s['page']:2}] (X0: {s['x0']:7.2f}, Y0: {s['y0']:7.2f}, X1: {s['x1']:7.2f}, Y1: {s['y1']:7.2f}) "
                f"[{s['font_size']:4.1f}pt | {s['font_name']}] \"{s['text']}\""
            )
    else:
        plain_text_lines.append("(No text found on this page)")
    plain_text_output = "\n".join(plain_text_lines)

    # 3. Detected Structured Tables
    table_dfs = []
    if "pages" in spatial_doc and len(spatial_doc["pages"]) >= page_idx:
        p_data = spatial_doc["pages"][page_idx - 1]
        for t in p_data.get("tables", []):
            if t.get("rows"):
                df = pd.DataFrame(t["rows"])
                if t.get("headers") and len(t["headers"]) == len(df.columns):
                    df.columns = t["headers"]
                table_dfs.append(df)

    main_table_df = table_dfs[0] if table_dfs else pd.DataFrame({"Info": ["No tabular matrices detected on this page."]})

    # 4. Performance & Summary Statistics
    stats_msg = (
        f"### ⚡ Engine Performance (Pure Rust SIMD + Rayon)\n\n"
        f"| Metric | Value |\n"
        f"| :--- | :--- |\n"
        f"| **⚡ Raw X/Y Parse Latency** | **{raw_doc.get('parse_time_ms', 0):.2f} ms** ({raw_doc.get('total_spans', 0)} spans) |\n"
        f"| **🎯 Structured Parse Latency** | **{spatial_doc.get('parse_time_ms', 0):.2f} ms** ({len(spatial_doc['pages'][page_idx - 1].get('text_blocks', []))} blocks) |\n"
        f"| **📄 Document Size** | **{total_pages} Page(s)** (Currently viewing Page {page_idx}) |\n"
        f"| **📊 Detected Tables** | **{len(spatial_doc['pages'][page_idx - 1].get('tables', []))} table(s)** |"
    )

    return overlay_img, raw_df, plain_text_output, main_table_df, raw_doc, spatial_doc, stats_msg

custom_css = """
.gradio-container {
    max-width: 96% !important;
    margin: auto !important;
}
/* Responsive Table Container */
.responsive-table {
    width: 100% !important;
    overflow-x: auto !important;
    -webkit-overflow-scrolling: touch;
}
.responsive-table table,
.responsive-table [role="table"],
.responsive-table [role="grid"] {
    width: 100% !important;
    min-width: 760px !important;
    table-layout: auto !important;
    border-collapse: separate !important;
    border-spacing: 0 !important;
}
.responsive-table th,
.responsive-table th *,
.responsive-table [role="columnheader"],
.responsive-table [role="columnheader"] * {
    background-color: #0f172a !important;
    color: #38bdf8 !important;
    font-size: 13px !important;
    font-weight: 600 !important;
    padding: 8px 10px !important;
    white-space: nowrap !important;
    word-break: keep-all !important;
    overflow: visible !important;
    text-overflow: clip !important;
}
.responsive-table th:first-child {
    min-width: 85px !important;
}
.responsive-table td {
    font-size: 13px !important;
    padding: 8px 10px !important;
    border-bottom: 1px solid #334155 !important;
    word-break: break-word !important;
}
/* Monospace typography for numerical coordinates */
.responsive-table td:nth-child(2),
.responsive-table td:nth-child(3),
.responsive-table td:nth-child(4),
.responsive-table td:nth-child(5),
.responsive-table td:nth-child(6) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace !important;
    color: #0284c7 !important;
    font-weight: 500 !important;
    white-space: nowrap !important;
    word-break: keep-all !important;
}
/* Expand extracted text column */
.responsive-table td:last-child {
    min-width: 320px !important;
    color: #f1f5f9 !important;
    word-break: break-word !important;
}
"""

demo_title = "⚡ RasgaPDF // High-Performance 2D Spatial PDF Parser"
demo_desc = (
    "Ultra-fast 2D spatial layout parser powered by **Pure Rust**. "
    "Supports both **Flat Physical Coordinates (Raw X/Y)** for maximum throughput (~3-8 ms), "
    "and **Hierarchical Structured Mode** (blocks, lines, and matrix table detection). "
    "Official polyglot bindings for **Python, Node.js, Go, and C# (.NET)**."
)

with gr.Blocks(title="RasgaPDF Spatial Viewer") as demo:
    gr.Markdown(f"# {demo_title}\n{demo_desc}")

    with gr.Row():
        with gr.Column(scale=1):
            file_input = gr.File(label="📄 Upload PDF File", file_types=[".pdf"])
            page_slider = gr.Slider(minimum=1, maximum=50, value=1, step=1, label="📖 Page Number")

            with gr.Accordion("⚙️ Extraction Parameters", open=False):
                detect_tables_check = gr.Checkbox(label="Detect Spatial Tables", value=True)
                min_cols_slider = gr.Slider(minimum=2, maximum=10, value=3, step=1, label="Minimum Columns for Table")
                col_tol_slider = gr.Slider(minimum=4.0, maximum=50.0, value=16.0, step=1.0, label="Column Alignment Tolerance (pt)")
                row_tol_slider = gr.Slider(minimum=2.0, maximum=20.0, value=6.0, step=0.5, label="Row Line Tolerance (pt)")

            with gr.Row():
                show_text_check = gr.Checkbox(label="Show Text Boxes (Cyan)", value=True)
                show_tables_check = gr.Checkbox(label="Show Tables (Emerald)", value=True)

            btn_parse = gr.Button("🚀 Run Spatial Analysis", variant="primary")
            stats_box = gr.Markdown()

        with gr.Column(scale=2):
            with gr.Tabs():
                with gr.TabItem("⚡ 1. Raw Coordinates (X/Y Table)"):
                    gr.Markdown("### Flat Physical Coordinates `[Page, X0, Y0, X1, Y1, Font, Text]`")
                    raw_df_output = gr.DataFrame(
                        label="Physical Coordinate Matrix",
                        elem_classes=["responsive-table"],
                        column_widths=["85px", "70px", "70px", "70px", "70px", "80px", "110px", "auto"],
                        wrap=True,
                        max_height=560
                    )

                with gr.TabItem("📝 2. Pure Plain Text"):
                    gr.Markdown("### Pure Text Stream & Raw Coordinate Dump (Instant Copy)")
                    plain_text_output_box = gr.Code(
                        label="Pure Text Flow & Spatial Coordinates Dump",
                        language="markdown",
                        lines=22
                    )

                with gr.TabItem("👁️ 3. 2D Bounding Box Overlay"):
                    gr.Markdown("### Physical 2D Canvas Overlay")
                    img_output = gr.Image(label="Visual 2D Overlay", type="pil")

                with gr.TabItem("📊 4. Detected Tables"):
                    gr.Markdown("### Structured Tables Extracted via Column Alignment")
                    table_df_output = gr.DataFrame(
                        label="Detected Tables",
                        elem_classes=["responsive-table"],
                        wrap=True,
                        max_height=560
                    )

                with gr.TabItem("📄 5. Flat Raw JSON"):
                    gr.Markdown("### Raw Flat Coordinates JSON (Zero Overhead)")
                    raw_json_output = gr.JSON(label="Raw Coordinates JSON")

                with gr.TabItem("🌳 6. Structured JSON"):
                    gr.Markdown("### Hierarchical Geometric Tree (Blocks, Lines, Spans, Tables)")
                    struct_json_output = gr.JSON(label="Structured Document JSON")

    inputs = [
        file_input, page_slider, detect_tables_check, min_cols_slider,
        col_tol_slider, row_tol_slider, show_text_check, show_tables_check
    ]
    outputs = [img_output, raw_df_output, plain_text_output_box, table_df_output, raw_json_output, struct_json_output, stats_box]

    btn_parse.click(fn=process_pdf, inputs=inputs, outputs=outputs)
    file_input.change(fn=process_pdf, inputs=inputs, outputs=outputs)
    page_slider.change(fn=process_pdf, inputs=inputs, outputs=outputs)

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860, css=custom_css)
