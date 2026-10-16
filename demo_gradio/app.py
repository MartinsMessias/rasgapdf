import os
import sys
import json
import ctypes
import fitz
import gradio as gr
import pandas as pd
from PIL import Image, ImageDraw, ImageFont

# Carrega a biblioteca nativa RasgaPDF
def get_lib():
    candidates = [
        os.path.join(os.path.dirname(__file__), "librasgapdf.so"),
        os.path.join(os.path.dirname(__file__), "..", "target", "release", "librasgapdf.so"),
        "/home/messias_martins/rasgapdf/target/release/librasgapdf.so"
    ]
    for c in candidates:
        if os.path.exists(c):
            lib = ctypes.CDLL(c)
            lib.rasgapdf_parse_file_options.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            lib.rasgapdf_parse_file_options.restype = ctypes.c_void_p
            lib.rasgapdf_free_string.argtypes = [ctypes.c_void_p]
            lib.rasgapdf_free_string.restype = None
            lib.rasgapdf_version.argtypes = []
            lib.rasgapdf_version.restype = ctypes.c_char_p
            return lib
    return None

_lib = get_lib()

def parse_spatial_pdf(file_path, detect_tables=True, min_cols=3, col_tol=16.0, row_tol=6.0):
    if not _lib:
        raise RuntimeError("RasgaPDF native library not found")
    
    opts = {
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

    # Cria overlay translúcido
    overlay = Image.new("RGBA", img.size, (255, 255, 255, 0))
    draw = ImageDraw.Draw(overlay)

    # 1. Desenha blocos e linhas de texto
    if show_text and "text_blocks" in page_data:
        for b in page_data["text_blocks"]:
            bx0 = b["bbox"]["x0"] * scale_x
            by0 = b["bbox"]["y0"] * scale_y
            bx1 = b["bbox"]["x1"] * scale_x
            by1 = b["bbox"]["y1"] * scale_y

            # Bloco (ciano translúcido com borda)
            draw.rectangle([bx0, by0, bx1, by1], fill=(0, 240, 255, 25), outline=(0, 200, 240, 180), width=2)

            for l in b.get("lines", []):
                lx0 = l["bbox"]["x0"] * scale_x
                ly0 = l["bbox"]["y0"] * scale_y
                lx1 = l["bbox"]["x1"] * scale_x
                ly1 = l["bbox"]["y1"] * scale_y
                draw.rectangle([lx0, ly0, lx1, ly1], outline=(0, 150, 255, 120), width=1)

    # 2. Desenha tabelas espaciais detectadas
    if show_tables and "tables" in page_data:
        for t in page_data["tables"]:
            tx0 = t["bbox"]["x0"] * scale_x
            ty0 = t["bbox"]["y0"] * scale_y
            tx1 = t["bbox"]["x1"] * scale_x
            ty1 = t["bbox"]["y1"] * scale_y

            # Borda verde esmeralda para tabelas
            draw.rectangle([tx0, ty0, tx1, ty1], fill=(0, 223, 137, 40), outline=(0, 223, 137, 240), width=3)

            # Células internas
            for cell in t.get("cells", []):
                cx0 = cell["bbox"]["x0"] * scale_x
                cy0 = cell["bbox"]["y0"] * scale_y
                cx1 = cell["bbox"]["x1"] * scale_x
                cy1 = cell["bbox"]["y1"] * scale_y
                draw.rectangle([cx0, cy0, cx1, cy1], outline=(16, 185, 129, 140), width=1)

    # Mescla overlay com a imagem original
    img = img.convert("RGBA")
    combined = Image.alpha_composite(img, overlay)
    return combined.convert("RGB")

def process_pdf(pdf_file, page_num, detect_tables, min_cols, col_tol, row_tol, show_text, show_tables):
    if not pdf_file:
        return None, None, {}, "⚠️ Por favor, faça upload de um arquivo PDF."

    file_path = pdf_file.name if hasattr(pdf_file, "name") else str(pdf_file)

    try:
        spatial_doc = parse_spatial_pdf(file_path, detect_tables, min_cols, col_tol, row_tol)
    except Exception as e:
        return None, None, {"error": str(e)}, f"❌ Erro na extração espacial: {e}"

    total_pages = spatial_doc.get("total_pages", 1)
    page_idx = min(max(int(page_num), 1), total_pages)

    overlay_img = render_page_with_overlay(file_path, page_idx, spatial_doc, show_text, show_tables)

    # Extrai tabelas para DataFrame
    table_dfs = []
    if "pages" in spatial_doc and len(spatial_doc["pages"]) >= page_idx:
        p_data = spatial_doc["pages"][page_idx - 1]
        for t in p_data.get("tables", []):
            if t.get("rows"):
                df = pd.DataFrame(t["rows"])
                if t.get("headers") and len(t["headers"]) == len(df.columns):
                    df.columns = t["headers"]
                table_dfs.append(df)

    main_df = table_dfs[0] if table_dfs else pd.DataFrame({"Mensagem": ["Nenhuma tabela detectada nesta página."]})

    stats_msg = (
        f"⚡ **RasgaPDF Engine (Rust 100% Nativo)**\n"
        f"- ⏱️ **Tempo de Parse:** {spatial_doc.get('parse_time_ms', 0):.2f} ms\n"
        f"- 📄 **Páginas:** {total_pages} (Exibindo Página {page_idx})\n"
        f"- 📦 **Blocos de Texto:** {len(spatial_doc['pages'][page_idx - 1].get('text_blocks', []))}\n"
        f"- 📊 **Tabelas Detectadas:** {len(spatial_doc['pages'][page_idx - 1].get('tables', []))}"
    )

    return overlay_img, main_df, spatial_doc, stats_msg

# Interface Gradio Moderna
demo_title = "⚡ RasgaPDF // High-Performance 2D Spatial PDF Parser"
demo_desc = (
    "Biblioteca aberta de alta performance para **extração espacial 2D de PDFs** em **Rust nativo**, "
    "com bindings oficiais para **Python, Node.js, Go e C#**. Preserva bounding boxes exatas `[x0, y0, x1, y1]`, "
    "linhas, palavras e agrupamento matricial de tabelas com latência sub-milissegundo."
)

with gr.Blocks(title="RasgaPDF Spatial Viewer", theme=gr.themes.Soft(primary_hue="cyan", neutral_hue="slate")) as demo:
    gr.Markdown(f"# {demo_title}\n{demo_desc}")

    with gr.Row():
        with gr.Column(scale=1):
            file_input = gr.File(label="📄 Arquivo PDF", file_types=[".pdf"])
            page_slider = gr.Slider(minimum=1, maximum=50, value=1, step=1, label="📖 Número da Página")

            with gr.Accordion("⚙️ Parâmetros Espaciais", open=False):
                detect_tables_check = gr.Checkbox(label="Detectar Tabelas Espaciais", value=True)
                min_cols_slider = gr.Slider(minimum=2, maximum=10, value=3, step=1, label="Mínimo de Colunas")
                col_tol_slider = gr.Slider(minimum=4.0, maximum=50.0, value=16.0, step=1.0, label="Tolerância de Coluna (pt)")
                row_tol_slider = gr.Slider(minimum=2.0, maximum=20.0, value=6.0, step=0.5, label="Tolerância de Linha (pt)")

            with gr.Row():
                show_text_check = gr.Checkbox(label="Exibir Caixas de Texto (Ciano)", value=True)
                show_tables_check = gr.Checkbox(label="Exibir Tabelas (Verde)", value=True)

            btn_parse = gr.Button("🚀 Analisar Geometria Espacial", variant="primary")
            stats_box = gr.Markdown()

        with gr.Column(scale=2):
            with gr.Tabs():
                with gr.TabItem("👁️ Visual 2D Bounding Boxes"):
                    img_output = gr.Image(label="Overlay Espacial 2D", type="pil")

                with gr.TabItem("📊 Tabelas Detectadas"):
                    df_output = gr.DataFrame(label="Tabelas Estruturadas")

                with gr.TabItem("🌳 Árvore JSON Espacial"):
                    json_output = gr.JSON(label="JSON Estruturado com Coordenadas Físicas")

    inputs = [
        file_input, page_slider, detect_tables_check, min_cols_slider,
        col_tol_slider, row_tol_slider, show_text_check, show_tables_check
    ]
    outputs = [img_output, df_output, json_output, stats_box]

    btn_parse.click(fn=process_pdf, inputs=inputs, outputs=outputs)
    file_input.change(fn=process_pdf, inputs=inputs, outputs=outputs)
    page_slider.change(fn=process_pdf, inputs=inputs, outputs=outputs)

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860)
