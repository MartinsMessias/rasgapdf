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

def version() -> str:
    if not _lib:
        return "0.1.0 (unlinked)"
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

    Returns:
        dict: SpatialDocument contendo páginas, blocos, linhas e tabelas.
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

__all__ = ["parse_raw", "parse_pdf", "version"]
