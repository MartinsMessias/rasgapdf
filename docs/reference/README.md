# RasgaPDF Reference Documentation

This section provides comprehensive, information-oriented reference specifications for RasgaPDF: data schemas, C-ABI signatures, CLI command-line options, and memory management guarantees.

---

## 1. Primary Reference Documents

- **[Spatial JSON Schema Specification](../SCHEMA.md)**: Exhaustive schema reference for both **Mode 1** (`RawSpatialDocument`) and **Mode 2** (`SpatialDocument`), including detailed field definitions for `SpatialPage`, `SpatialBlock`, `SpatialLine`, `SpatialSpan`, `SpatialTable`, `SpatialCell`, `SpatialImage`, and `SpatialDrawing`.
- **[C-ABI C99 Header (`include/rasgapdf.h`)](../../include/rasgapdf.h)**: Complete C function declarations, argument descriptions, and heap allocation contracts for native shared libraries (`librasgapdf.so`, `rasgapdf.dll`, `librasgapdf.dylib`).

---

## 2. C-ABI Specification & Memory Contracts

All foreign language bindings (Python, Node.js, Go, C#) communicate through the pure Rust C-ABI exported in `src/ffi.rs`.

### Exported Symbols

| Function Signature | Description | Allocation / Ownership |
| :--- | :--- | :--- |
| `char* rasgapdf_parse_raw(const char* path_ptr)` | Mode 1 file parse. Returns flat JSON string of physical spans. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_parse_raw_bytes(const uint8_t* bytes_ptr, size_t len)` | Mode 1 in-memory parse. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_parse_file(const char* path_ptr)` | Mode 2 file parse with default options. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_parse_file_options(const char* path_ptr, const char* opts_json)` | Mode 2 file parse with custom JSON options. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_extract_text_bytes(const uint8_t* bytes_ptr, size_t len)` | Parallel natural reading order text extraction from RAM. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_extract_images(const char* path_ptr, const char* dir_ptr)` | Extracts raw image streams to disk and writes `manifest.json`. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `char* rasgapdf_extract_images_bytes(const uint8_t* bytes, size_t len, const char* dir)` | Extracts raw image streams from RAM to disk. | Heap string. Caller **must** call `rasgapdf_free_string`. |
| `void rasgapdf_free_string(char* ptr)` | Deallocates heap string allocated by Rust. | Releases memory. Safe to call with `NULL`. |
| `const char* rasgapdf_version(void)` | Returns library version string. | Static string literal. Do **not** free. |

### Parser Options Specification (JSON)

When invoking `rasgapdf_parse_file_options` or binding options:

```json
{
  "mode": "structured",
  "detect_tables": true,
  "min_table_columns": 3,
  "column_tolerance": 16.0,
  "row_tolerance": 6.0,
  "extract_images": false,
  "extract_drawings": false,
  "keep_background_fills": false
}
```

---

## 3. CLI Command-Line Reference

The standalone binary `rasgapdf` supports the following flags:

```
Usage: rasgapdf [OPTIONS] <INPUT_PDF>

Arguments:
  <INPUT_PDF>  Path to the PDF file to process

Options:
  -r, --raw                     Mode 1: Extract flat physical coordinates [x0, y0, x1, y1]
  -f, --format <FORMAT>         Output format: json (default), text, csv, tsv [default: json]
  -p, --pretty                  Pretty-print JSON output with 2-space indentation
  -o, --output <OUTPUT_PATH>    Write output to specified file instead of stdout
      --no-tables               Disable automatic 2D matrix table detection
      --min-cols <COUNT>        Minimum columns to classify a region as a table [default: 3]
      --images                  Opt-in: Include embedded image metadata in JSON tree
      --drawings                Opt-in: Include vector paths in JSON tree
      --extract-images <DIR>    Extract original image streams (.jpg, .png) to target directory
      --keep-backgrounds        Preserve full-page background color fills in vector paths
  -h, --help                    Print help information
  -V, --version                 Print version information
```

### Exit Codes
- `0`: Success.
- `1`: I/O error, file not found, or corrupted PDF structure.
- `2`: Invalid CLI arguments or unrecognized flags.

---

## 4. Supported PDF Features (ISO 32000-1 Subset)

RasgaPDF's pure Rust engine natively implements:
- **Structure**: Cross-reference tables (`xref`), cross-reference streams (`/XRef`), linearization streams, object streams (`/ObjStm`), incremental revisions.
- **Decompression Filters**: `FlateDecode` (Deflate/zlib, RFC 1950/1951) with PNG / TIFF Predictors, `ASCIIHexDecode`, `ASCII85Decode`.
- **Fonts & Encodings**: Type 1 (Standard 14, MacRoman, WinAnsi), TrueType (`/WinAnsiEncoding`, `/MacRomanEncoding`), Type 0 CID Fonts with `/ToUnicode` CMap streams, composite horizontal/vertical glyph positioning.
- **Image Filters**: `DCTDecode` (JPEG), `FlateDecode` (lossless PNG/raw bitmaps), `JPXDecode` (JPEG 2000 metadata), `/DeviceRGB`, `/DeviceGray`, `/DeviceCMYK`, `/Indexed`.
- **Vector Paths**: Moveto (`m`), lineto (`l`), curveto (`c`, `v`, `y`), closepath (`h`), rect (`re`), stroke (`S`, `s`), fill (`f`, `F`, `f*`), setdash (`d`), setlinewidth (`w`).
