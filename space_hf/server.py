#!/usr/bin/env python3
import os
import sys
import tempfile
import time
import base64
from pathlib import Path

import fitz
from fastapi import FastAPI, UploadFile, File
from fastapi.responses import HTMLResponse, FileResponse, JSONResponse
from fastapi.middleware.cors import CORSMiddleware
import uvicorn

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rasgapdf

app = FastAPI(title="RasgaPDF Native Server", version=rasgapdf.version())

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.get("/", response_class=HTMLResponse)
def get_index():
    index_path = HERE / "index.html"
    return HTMLResponse(content=index_path.read_text(encoding="utf-8"))

@app.get("/llms.txt")
def get_llms_txt():
    path = HERE / "llms.txt"
    if path.exists():
        return FileResponse(str(path), media_type="text/plain; charset=utf-8")
    return JSONResponse(status_code=404, content={"error": "llms.txt not found"})

@app.get("/llms-full.txt")
def get_llms_full_txt():
    path = HERE / "llms-full.txt"
    if path.exists():
        return FileResponse(str(path), media_type="text/plain; charset=utf-8")
    return JSONResponse(status_code=404, content={"error": "llms-full.txt not found"})

@app.get("/robots.txt")
def get_robots_txt():
    path = HERE / "robots.txt"
    if path.exists():
        return FileResponse(str(path), media_type="text/plain; charset=utf-8")
    return JSONResponse(status_code=404, content={"error": "robots.txt not found"})

@app.get("/sitemap.xml")
def get_sitemap_xml():
    path = HERE / "sitemap.xml"
    if path.exists():
        return FileResponse(str(path), media_type="application/xml")
    return JSONResponse(status_code=404, content={"error": "sitemap.xml not found"})

@app.get("/sample.pdf")
def get_sample():
    sample_path = HERE / "sample.pdf"
    if sample_path.exists():
        return FileResponse(str(sample_path), media_type="application/pdf")
    return JSONResponse(status_code=404, content={"error": "sample.pdf not found"})

@app.get("/api/status")
def get_status():
    return {
        "status": "ok",
        "engine": "rust",
        "version": rasgapdf.version(),
        "lib_loaded": bool(rasgapdf._lib),
    }

@app.post("/api/parse")
async def parse_pdf_endpoint(file: UploadFile = File(...)):
    start_time = time.perf_counter()
    
    with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp_file:
        tmp_path = tmp_file.name
        content = await file.read()
        tmp_file.write(content)

    img_tmp_dir = tempfile.mkdtemp(prefix="rasga_imgs_")

    try:
        raw_res = rasgapdf.parse_raw(tmp_path)
        struct_res = rasgapdf.parse_pdf(
            tmp_path,
            detect_tables=True,
            extract_images=True,
            extract_drawings=True,
            keep_background_fills=False,
        )
        images_manifest = rasgapdf.extract_images(tmp_path, img_tmp_dir)
        
        data_uris = {}
        pdf_doc = None
        try:
            pdf_doc = fitz.open(tmp_path)
        except Exception:
            pass

        for entry in images_manifest.get("images", []):
            page_num = entry.get("page")
            img_data = entry.get("image", {})
            img_id = img_data.get("image_id")
            xref = img_data.get("xref")

            uri = None
            if pdf_doc and xref:
                try:
                    pix = fitz.Pixmap(pdf_doc, xref)
                    if pix.n >= 5 or pix.colorspace == fitz.csCMYK:
                        pix = fitz.Pixmap(fitz.csRGB, pix)
                    png_bytes = pix.tobytes("png")
                    b64 = base64.b64encode(png_bytes).decode("ascii")
                    uri = f"data:image/png;base64,{b64}"
                except Exception:
                    pass

            if not uri:
                rel_file = img_data.get("file")
                if rel_file:
                    abs_file = os.path.join(img_tmp_dir, rel_file)
                    if os.path.exists(abs_file):
                        mime = img_data.get("mime_type") or "image/jpeg"
                        with open(abs_file, "rb") as f:
                            b64 = base64.b64encode(f.read()).decode("ascii")
                            uri = f"data:{mime};base64,{b64}"

            if uri:
                if img_id is not None:
                    data_uris[str(img_id)] = uri
                    data_uris[img_id] = uri
                if xref is not None:
                    data_uris[f"xref_{xref}"] = uri
                if page_num is not None and img_id is not None:
                    data_uris[f"{page_num}_{img_id}"] = uri

        total_elapsed_ms = (time.perf_counter() - start_time) * 1000.0
        rust_time_ms = struct_res.get("parse_time_ms") or raw_res.get("parse_time_ms") or total_elapsed_ms

        return {
            "status": "ok",
            "engine": "Native Rust (librasgapdf.so)",
            "version": rasgapdf.version(),
            "parse_time_ms": rust_time_ms,
            "total_server_time_ms": total_elapsed_ms,
            "raw": raw_res,
            "structured": struct_res,
            "images_manifest": images_manifest,
            "image_data_uris": data_uris,
        }
    except Exception as e:
        import logging
        logging.error("Error processing PDF: %s", e)
        return JSONResponse(
            status_code=500,
            content={"status": "error", "message": "Failed to process PDF document"}
        )
    finally:
        try:
            if os.path.exists(tmp_path):
                os.remove(tmp_path)
        except Exception:
            pass
        try:
            import shutil
            if os.path.exists(img_tmp_dir):
                shutil.rmtree(img_tmp_dir)
        except Exception:
            pass

if __name__ == "__main__":
    port = int(os.environ.get("PORT", 7860))
    uvicorn.run(app, host="0.0.0.0", port=port)
