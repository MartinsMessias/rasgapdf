import os
import sys
import json
import ctypes
from typing import Dict, Any, List, Optional

def _find_library() -> str:
    lib_name = "librasgapdf.so" if sys.platform != "win32" else "rasgapdf.dll"
    if sys.platform == "darwin":
        lib_name = "librasgapdf.dylib"

    search_paths = [
        # Relative to installed package
        os.path.join(os.path.dirname(__file__), lib_name),
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "target", "release", lib_name),
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "target", "debug", lib_name),
        os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", lib_name),
        os.path.join(os.path.expanduser("~"), "rasgapdf", "target", "release", lib_name),
        "/usr/local/lib/" + lib_name,
        "/usr/lib/" + lib_name,
    ]

    for p in search_paths:
        abs_p = os.path.abspath(p)
        if os.path.exists(abs_p):
            return abs_p

    # Fallback to system loader
    return lib_name

_lib_path = _find_library()
try:
    _lib = ctypes.CDLL(_lib_path)
except Exception as e:
    _lib = None

if _lib:
    _lib.rasgapdf_parse_raw.argtypes = [ctypes.c_char_p]
    _lib.rasgapdf_parse_raw.restype = ctypes.c_void_p

    _lib.rasgapdf_parse_file.argtypes = [ctypes.c_char_p]
    _lib.rasgapdf_parse_file.restype = ctypes.c_void_p

    _lib.rasgapdf_parse_file_options.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    _lib.rasgapdf_parse_file_options.restype = ctypes.c_void_p

    _lib.rasgapdf_free_string.argtypes = [ctypes.c_void_p]
    _lib.rasgapdf_free_string.restype = None

    _lib.rasgapdf_version.argtypes = []
    _lib.rasgapdf_version.restype = ctypes.c_char_p

    # Disponível a partir da 0.1.4 (extração de imagens embutidas em disco).
    if hasattr(_lib, "rasgapdf_extract_images"):
        _lib.rasgapdf_extract_images.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
        _lib.rasgapdf_extract_images.restype = ctypes.c_void_p

    # Funções em memória (bytes diretos em RAM, zero I/O de disco)
    if hasattr(_lib, "rasgapdf_parse_raw_bytes"):
        _lib.rasgapdf_parse_raw_bytes.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
        _lib.rasgapdf_parse_raw_bytes.restype = ctypes.c_void_p

    if hasattr(_lib, "rasgapdf_extract_text_bytes"):
        _lib.rasgapdf_extract_text_bytes.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
        _lib.rasgapdf_extract_text_bytes.restype = ctypes.c_void_p

    if hasattr(_lib, "rasgapdf_extract_images_bytes"):
        _lib.rasgapdf_extract_images_bytes.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p]
        _lib.rasgapdf_extract_images_bytes.restype = ctypes.c_void_p

def version() -> str:
    if not _lib:
        return "0.2.5 (unlinked)"
    return _lib.rasgapdf_version().decode("utf-8")

def parse_raw(file_path: str) -> Dict[str, Any]:
    """
    ⚡ Modo 1 (Bruto): Retorna exclusivamente a lista plana de coordenadas X/Y
    sem overhead de blocos hierárquicos ou detecção de tabelas.

    Args:
        file_path: Caminho do arquivo PDF.

    Returns:
        dict: {
            "file_name": str,
            "total_pages": int,
            "total_spans": int,
            "parse_time_ms": float,
            "spans": [
                {
                    "page": int,
                    "x0": float,
                    "y0": float,
                    "x1": float,
                    "y1": float,
                    "font_size": float,
                    "font_name": str,
                    "text": str
                }, ...
            ]
        }
    """
    if not _lib:
        raise RuntimeError(f"RasgaPDF native library ({_lib_path}) not loaded.")

    if not os.path.exists(file_path):
        raise FileNotFoundError(f"PDF file not found: {file_path}")

    path_bytes = os.path.abspath(file_path).encode("utf-8")
    res_ptr = _lib.rasgapdf_parse_raw(path_bytes)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native rasgapdf library.")

    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        parsed = json.loads(raw_json)
        if isinstance(parsed, dict) and parsed.get("error"):
            raise RuntimeError(f"RasgaPDF Error: {parsed.get('message')}")
        return parsed
    finally:
        _lib.rasgapdf_free_string(res_ptr)

def parse_pdf(
    file_path: str,
    detect_tables: bool = True,
    min_table_cols: int = 3,
    col_tolerance: float = 16.0,
    row_tolerance: float = 6.0,
    extract_images: bool = False,
    extract_drawings: bool = False,
    keep_background_fills: bool = False,
) -> Dict[str, Any]:
    """
    🎯 Modo 2 (Estruturado): Processa o PDF e retorna o layout espacial hierárquico
    com blocos de texto, linhas, spans e tabelas detectadas.

    Args:
        file_path: Caminho do arquivo PDF.
        detect_tables: Se deve detectar matrizes de tabelas automaticamente.
        min_table_cols: Quantidade mínima de colunas para agrupar em tabela.
        col_tolerance: Tolerância horizontal de alinhamento em pontos.
        row_tolerance: Tolerância vertical de linha em pontos.
        extract_images: Preenche `page["images"]` com os metadados das imagens embutidas.
        extract_drawings: Preenche `page["drawings"]` com os caminhos vetoriais.
        keep_background_fills: Mantém preenchimentos que cobrem a página inteira (fundos).

    Returns:
        dict: SpatialDocument contendo páginas, blocos, linhas, tabelas,
              imagens (opcional) e vetores (opcional).
    """
    if not _lib:
        raise RuntimeError(f"RasgaPDF native library ({_lib_path}) not loaded.")

    if not os.path.exists(file_path):
        raise FileNotFoundError(f"PDF file not found: {file_path}")

    opts = {
        "mode": "structured",
        "detect_tables": detect_tables,
        "min_table_columns": min_table_cols,
        "column_tolerance": col_tolerance,
        "row_tolerance": row_tolerance,
        "extract_images": extract_images,
        "extract_drawings": extract_drawings,
        "filter_background_fills": not keep_background_fills,
    }

    path_bytes = os.path.abspath(file_path).encode("utf-8")
    opts_bytes = json.dumps(opts).encode("utf-8")

    res_ptr = _lib.rasgapdf_parse_file_options(path_bytes, opts_bytes)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native rasgapdf library.")

    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        parsed = json.loads(raw_json)
        if isinstance(parsed, dict) and parsed.get("error"):
            raise RuntimeError(f"RasgaPDF Error: {parsed.get('message')}")
        return parsed
    finally:
        _lib.rasgapdf_free_string(res_ptr)

def extract_images(file_path: str, output_dir: str) -> Dict[str, Any]:
    """
    🖼️ Extrai os streams originais das imagens embutidas para `output_dir`.

    Nenhum pixel é decodificado: o stream codificado é copiado como está
    (ex.: `DCTDecode` → `.jpg`). Um `manifest.json` é gravado junto.

    Args:
        file_path: Caminho do arquivo PDF.
        output_dir: Pasta destino (criada se não existir).

    Returns:
        dict: {
            "total_images": int,
            "written": int,
            "images": [
                {"page": int, "image": {..., "file": str}},
                ...
            ]
        }
    """
    if not _lib:
        raise RuntimeError(f"RasgaPDF native library ({_lib_path}) not loaded.")

    if not hasattr(_lib, "rasgapdf_extract_images"):
        raise RuntimeError(
            "A biblioteca nativa carregada não expõe `rasgapdf_extract_images`. "
            "Recompile/atualize o RasgaPDF."
        )

    if not os.path.exists(file_path):
        raise FileNotFoundError(f"PDF file not found: {file_path}")

    path_bytes = os.path.abspath(file_path).encode("utf-8")
    dir_bytes = os.path.abspath(output_dir).encode("utf-8")

    res_ptr = _lib.rasgapdf_extract_images(path_bytes, dir_bytes)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native rasgapdf library.")

    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        parsed = json.loads(raw_json)
        if isinstance(parsed, dict) and parsed.get("error"):
            raise RuntimeError(f"RasgaPDF Error: {parsed.get('message')}")
        return parsed
    finally:
        _lib.rasgapdf_free_string(res_ptr)


def parse_raw_bytes(data: bytes) -> Dict[str, Any]:
    """
    ⚡ Modo 1 (Em Memória): Extrai coordenadas físicas [x0, y0, x1, y1] direto de `bytes` em RAM.
    """
    if not _lib or not hasattr(_lib, "rasgapdf_parse_raw_bytes"):
        raise RuntimeError("RasgaPDF native library with `rasgapdf_parse_raw_bytes` not loaded.")
    res_ptr = _lib.rasgapdf_parse_raw_bytes(data, len(data))
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native rasgapdf library.")
    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        parsed = json.loads(raw_json)
        if isinstance(parsed, dict) and parsed.get("error"):
            raise RuntimeError(f"RasgaPDF Error: {parsed.get('message')}")
        return parsed
    finally:
        _lib.rasgapdf_free_string(res_ptr)


def extract_text(data_or_path: bytes | str) -> str:
    """
    🚀 Extrai o texto completo do PDF em ordem natural de blocos de leitura
    diretamente de `bytes` ou caminho de arquivo.
    """
    if isinstance(data_or_path, (bytes, bytearray)):
        data = bytes(data_or_path)
        is_bytes = True
    else:
        data = None
        is_bytes = False

    # 1. Caminho ultra-rápido nativo (se a biblioteca expor rasgapdf_extract_text_bytes)
    if _lib and hasattr(_lib, "rasgapdf_extract_text_bytes"):
        b_data = data if is_bytes else Path(data_or_path).read_bytes()
        res_ptr = _lib.rasgapdf_extract_text_bytes(b_data, len(b_data))
        if res_ptr:
            try:
                res_str = ctypes.string_at(res_ptr).decode("utf-8", errors="replace")
                if res_str.strip():
                    return res_str
            finally:
                _lib.rasgapdf_free_string(res_ptr)

    # 2. Fallback robusto via parse_pdf estruturado (preserva blocos e colunas)
    tmp_path = None
    try:
        if is_bytes:
            import tempfile
            with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tf:
                tf.write(data)
                tmp_path = tf.name
            path_to_parse = tmp_path
        else:
            path_to_parse = str(data_or_path)

        doc = parse_pdf(path_to_parse, detect_tables=False)
        pages_texts = []
        for page in doc.get("pages", []):
            lines = []
            for block in page.get("text_blocks", []):
                for line in block.get("lines", []):
                    text = line.get("text", "")
                    if text:
                        lines.append(text)
            pages_texts.append("\n".join(lines))
        return "\n".join(pages_texts)
    finally:
        if tmp_path and os.path.exists(tmp_path):
            try:
                os.remove(tmp_path)
            except Exception:
                pass


def extract_images_bytes(data: bytes, output_dir: str) -> Dict[str, Any]:
    """
    🖼️ Extrai streams originais de imagens embutidas direto de `bytes` em RAM para `output_dir`.
    """
    if not _lib or not hasattr(_lib, "rasgapdf_extract_images_bytes"):
        raise RuntimeError("RasgaPDF native library with `rasgapdf_extract_images_bytes` not loaded.")
    dir_bytes = os.path.abspath(output_dir).encode("utf-8")
    res_ptr = _lib.rasgapdf_extract_images_bytes(data, len(data), dir_bytes)
    if not res_ptr:
        raise RuntimeError("Null pointer returned from native rasgapdf library.")
    try:
        raw_json = ctypes.string_at(res_ptr).decode("utf-8")
        parsed = json.loads(raw_json)
        if isinstance(parsed, dict) and parsed.get("error"):
            raise RuntimeError(f"RasgaPDF Error: {parsed.get('message')}")
        return parsed
    finally:
        _lib.rasgapdf_free_string(res_ptr)

__all__ = ["parse_raw", "parse_raw_bytes", "parse_pdf", "extract_text", "extract_images", "extract_images_bytes", "version"]
