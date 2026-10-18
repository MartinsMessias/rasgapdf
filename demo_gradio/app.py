import os
import sys
import json
import ctypes
import tempfile
import fitz
import gradio as gr
import pandas as pd
from PIL import Image, ImageDraw

OVERLAY_MAGENTA = (236, 72, 153)
OVERLAY_ORANGE = (249, 115, 22)


def _shared_lib_names():
    """Nome do binário nativo por plataforma (o HF Space roda em Linux)."""
    if sys.platform.startswith("win"):
        return ["rasgapdf.dll"]
    if sys.platform == "darwin":
        return ["librasgapdf.dylib"]
    return ["librasgapdf.so"]


# Load native RasgaPDF shared library
def get_lib():
    here = os.path.dirname(os.path.abspath(__file__))

    candidates = []
    for name in _shared_lib_names():
        candidates += [
            os.path.join(here, name),
            os.path.join(here, "..", "target", "release", name),
            os.path.join(here, "..", "target", "debug", name),
        ]

    for c in candidates:
        if os.path.exists(c):
            lib = ctypes.CDLL(c)

            lib.rasgapdf_parse_raw.argtypes = [ctypes.c_char_p]
            lib.rasgapdf_parse_raw.restype = ctypes.c_void_p

            lib.rasgapdf_parse_file_options.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            lib.rasgapdf_parse_file_options.restype = ctypes.c_void_p

            lib.rasgapdf_extract_images.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            lib.rasgapdf_extract_images.restype = ctypes.c_void_p

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

def parse_spatial_pdf(file_path, detect_tables=True, min_cols=3, col_tol=16.0, row_tol=6.0,
                      extract_images=False, extract_drawings=False, keep_backgrounds=False):
    """Mode 2: Hierarchical Structured Document (Blocks, Lines, Tables, Images & Vectors)"""
    if not _lib:
        raise RuntimeError("RasgaPDF native library not found")

    opts = {
        "mode": "structured",
        "detect_tables": detect_tables,
        "min_table_columns": int(min_cols),
        "column_tolerance": float(col_tol),
        "row_tolerance": float(row_tol),
        # Opt-in graphics: omitir estas chaves preserva o formato antigo.
        "extract_images": bool(extract_images),
        "extract_drawings": bool(extract_drawings),
        "filter_background_fills": not bool(keep_backgrounds),
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

def extract_embedded_images(file_path, output_dir):
    """🖼️ Writes every embedded image stream to `output_dir`; returns the manifest.

    Bytes are copied verbatim (no pixel decoding), so a DCTDecode image lands as a
    `.jpg`. The JSON tree itself never carries pixels — only metadata + SHA-256.
    """
    if not _lib:
        raise RuntimeError("RasgaPDF native library not found")

    path_b = os.path.abspath(file_path).encode("utf-8")
    out_b = os.path.abspath(output_dir).encode("utf-8")

    res_ptr = _lib.rasgapdf_extract_images(path_b, out_b)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native library")

    try:
        return json.loads(ctypes.string_at(res_ptr).decode("utf-8"))
    finally:
        _lib.rasgapdf_free_string(res_ptr)

def _rgb255(color, fallback=None):
    """Normalized 0.0..=1.0 RGB (RasgaPDF) -> 0..255 tuple (Pillow)."""
    if color and len(color) >= 3:
        return tuple(max(0, min(255, int(round(float(c) * 255)))) for c in color[:3])
    return fallback

def render_page_with_overlay(pdf_path, page_num, spatial_doc, show_text=True, show_tables=True,
                             show_images=True, show_drawings=True):
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

    def sx(value):
        return value * scale_x

    def sy(value):
        return value * scale_y

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

    # 3. Vector graphics overlay (magenta) — real geometry, not only bounding boxes
    if show_drawings:
        for d in page_data.get("drawings", []):
            d_type = d.get("drawing_type", "stroke")
            stroke_rgb = _rgb255(d.get("stroke_color"), fallback=OVERLAY_MAGENTA)
            fill_rgb = _rgb255(d.get("fill_color"))

            bb = d["bbox"]
            box = [sx(bb["x0"]), sy(bb["y0"]), sx(bb["x1"]), sy(bb["y1"])]

            # Filled paths: translucent bbox so large shapes read at a glance
            if d_type in ("fill", "fill_stroke") and fill_rgb:
                box_outline = stroke_rgb + (220,) if d_type == "fill_stroke" else None
                draw.rectangle(box, fill=fill_rgb + (45,), outline=box_outline)

            if d_type == "fill" and not d.get("items"):
                continue

            line_color = stroke_rgb + (235,)
            width_px = max(1, int(round((d.get("width") or 1.0) * scale_x)))

            for item in d.get("items", []):
                kind = item.get("kind")

                if kind == "line":
                    p1, p2 = item["p1"], item["p2"]
                    draw.line([sx(p1[0]), sy(p1[1]), sx(p2[0]), sy(p2[1])],
                              fill=line_color, width=width_px)

                elif kind == "rect":
                    ibb = item["bbox"]
                    draw.rectangle([sx(ibb["x0"]), sy(ibb["y0"]), sx(ibb["x1"]), sy(ibb["y1"])],
                                   outline=line_color, width=width_px)

                elif kind == "quad":
                    pts = [(sx(p[0]), sy(p[1])) for p in item["points"]]
                    draw.polygon(pts, outline=line_color)

                elif kind == "curve":
                    # p1/p4 = endpoints, p2/p3 = controls (no per-curve bbox in the schema)
                    xs = [item[k][0] for k in ("p1", "p2", "p3", "p4")]
                    ys = [item[k][1] for k in ("p1", "p2", "p3", "p4")]
                    draw.rectangle([sx(min(xs)), sy(min(ys)), sx(max(xs)), sy(max(ys))],
                                   outline=OVERLAY_MAGENTA + (190,), width=1)

    # 4. Embedded images overlay (orange)
    if show_images:
        for embedded in page_data.get("images", []):
            bb = embedded.get("bbox")
            if not bb:
                # Placement unknown (repeated/clipped image) — resource metadata is still exact
                continue
            draw.rectangle([sx(bb["x0"]), sy(bb["y0"]), sx(bb["x1"]), sy(bb["y1"])],
                           fill=OVERLAY_ORANGE + (30,), outline=OVERLAY_ORANGE + (240,), width=3)

    img = img.convert("RGBA")
    combined = Image.alpha_composite(img, overlay)
    return combined.convert("RGB")

def process_pdf(pdf_file, page_num, detect_tables, min_cols, col_tol, row_tol,
                extract_images, extract_drawings, keep_backgrounds,
                show_text, show_tables, show_images, show_drawings):
    empty_df = pd.DataFrame()
    if not pdf_file:
        return None, empty_df, "", empty_df, {}, {}, [], "", "⚠️ Please upload a PDF file to begin analysis."

    file_path = pdf_file.name if hasattr(pdf_file, "name") else str(pdf_file)

    try:
        raw_doc = parse_raw_pdf(file_path)
        spatial_doc = parse_spatial_pdf(
            file_path, detect_tables, min_cols, col_tol, row_tol,
            extract_images, extract_drawings, keep_backgrounds
        )
    except Exception as e:
        return None, empty_df, f"Error: {e}", empty_df, {"error": str(e)}, {}, [], f"⚠️ {e}", f"❌ Spatial extraction error: {e}"

    total_pages = spatial_doc.get("total_pages", 1)
    page_idx = min(max(int(page_num), 1), total_pages)

    overlay_img = render_page_with_overlay(file_path, page_idx, spatial_doc, show_text, show_tables,
                                           show_images, show_drawings)

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

    # 4. Graphics summary (page-level counts)
    p_data = spatial_doc["pages"][page_idx - 1]
    n_images = len(p_data.get("images", []))
    n_drawings = len(p_data.get("drawings", []))
    n_image_blocks = sum(1 for b in p_data.get("text_blocks", []) if b.get("block_type") == "image")

    # 5. Embedded images sidecar extraction (gallery)
    gallery_items = []
    images_note = ""
    if extract_images:
        try:
            out_dir = tempfile.mkdtemp(prefix="rasgapdf_images_")
            manifest = extract_embedded_images(file_path, out_dir)

            for rec in manifest.get("images", []):
                meta = rec.get("image", {})
                fname = meta.get("file")
                if not fname:
                    continue
                fpath = os.path.join(out_dir, fname)
                if not os.path.exists(fpath):
                    continue

                caption = (
                    f"p{rec.get('page')} · {meta.get('name')} · "
                    f"{meta.get('pixel_width')}x{meta.get('pixel_height')} px · "
                    f"{meta.get('filter') or 'raw'} · {meta.get('byte_length')} B"
                )
                gallery_items.append((fpath, caption))

            images_note = (
                f"### 🖼️ Embedded Images Written to Disk\n\n"
                f"| Metric | Value |\n"
                f"| :--- | :--- |\n"
                f"| **Total images** | **{manifest.get('total_images', 0)}** |\n"
                f"| **Written** | **{manifest.get('written', 0)}** |\n"
                f"| **Placement mode** | Original encoded streams (DCTDecode → `.jpg`), pixels **never** inlined in JSON |\n"
            )
        except Exception as e:
            images_note = f"⚠️ Image extraction error: {e}"
    else:
        images_note = (
            "### 🖼️ Embedded Images\n\n"
            "Enable **Extract Embedded Images** in the sidebar to write the original "
            "encoded streams and preview them here."
        )

    # 6. Performance & Summary Statistics
    stats_msg = (
        f"### ⚡ Engine Performance (Pure Rust SIMD + Rayon)\n\n"
        f"| Metric | Value |\n"
        f"| :--- | :--- |\n"
        f"| **⚡ Raw X/Y Parse Latency** | **{raw_doc.get('parse_time_ms', 0):.2f} ms** ({raw_doc.get('total_spans', 0)} spans) |\n"
        f"| **🎯 Structured Parse Latency** | **{spatial_doc.get('parse_time_ms', 0):.2f} ms** ({len(p_data.get('text_blocks', []))} blocks) |\n"
        f"| **📄 Document Size** | **{total_pages} Page(s)** (Currently viewing Page {page_idx}) |\n"
        f"| **📊 Detected Tables** | **{len(p_data.get('tables', []))} table(s)** |\n"
        f"| **🖼️ Embedded Images** | **{n_images}** (metadata; {n_image_blocks} placed block(s) on this page) |\n"
        f"| **📐 Vector Drawings** | **{n_drawings} path(s)** on this page |"
    )

    return overlay_img, raw_df, plain_text_output, main_table_df, raw_doc, spatial_doc, gallery_items, images_note, stats_msg

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
    "and **Hierarchical Structured Mode** (blocks, lines, tables, **embedded images** and "
    "**vector graphics**). "
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

            with gr.Accordion("🖼️ Graphics Extraction (opt-in)", open=False):
                extract_images_check = gr.Checkbox(label="Extract Embedded Images (metadata + sidecar)", value=False)
                extract_drawings_check = gr.Checkbox(label="Extract Vector Drawings (paths)", value=False)
                keep_backgrounds_check = gr.Checkbox(label="Keep Full-Page Background Fills", value=False)

            with gr.Row():
                show_text_check = gr.Checkbox(label="Show Text Boxes (Cyan)", value=True)
                show_tables_check = gr.Checkbox(label="Show Tables (Emerald)", value=True)

            with gr.Row():
                show_images_check = gr.Checkbox(label="Show Images (Orange)", value=True)
                show_drawings_check = gr.Checkbox(label="Show Vectors (Magenta)", value=True)

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
                    gr.Markdown("### Physical 2D Canvas Overlay\nText (cyan) · tables (emerald) · vectors (magenta) · images (orange)")
                    img_output = gr.Image(label="Visual 2D Overlay", type="pil")

                with gr.TabItem("📊 4. Detected Tables"):
                    gr.Markdown("### Structured Tables Extracted via Column Alignment")
                    table_df_output = gr.DataFrame(
                        label="Detected Tables",
                        elem_classes=["responsive-table"],
                        wrap=True,
                        max_height=560
                    )

                with gr.TabItem("🖼️ 5. Embedded Images"):
                    gr.Markdown("### Original Image Streams (sidecar extraction)")
                    image_gallery_output = gr.Gallery(label="Extracted Images", columns=3, height=420, allow_preview=True, buttons=["fullscreen"])
                    images_note_output = gr.Markdown()

                with gr.TabItem("📄 6. Flat Raw JSON"):
                    gr.Markdown("### Raw Flat Coordinates JSON (Zero Overhead)")
                    raw_json_output = gr.JSON(label="Raw Coordinates JSON")

                with gr.TabItem("🌳 7. Structured JSON"):
                    gr.Markdown("### Hierarchical Geometric Tree (Blocks, Lines, Spans, Tables, Images, Drawings)")
                    struct_json_output = gr.JSON(label="Structured Document JSON")

    inputs = [
        file_input, page_slider, detect_tables_check, min_cols_slider,
        col_tol_slider, row_tol_slider,
        extract_images_check, extract_drawings_check, keep_backgrounds_check,
        show_text_check, show_tables_check, show_images_check, show_drawings_check
    ]
    outputs = [
        img_output, raw_df_output, plain_text_output_box, table_df_output,
        raw_json_output, struct_json_output, image_gallery_output, images_note_output, stats_box
    ]

    btn_parse.click(fn=process_pdf, inputs=inputs, outputs=outputs)
    file_input.change(fn=process_pdf, inputs=inputs, outputs=outputs)
    page_slider.change(fn=process_pdf, inputs=inputs, outputs=outputs)

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860, css=custom_css)
