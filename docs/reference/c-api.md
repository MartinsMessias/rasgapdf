# C-ABI & Foreign Function Interface (FFI) Reference

This specification defines the C-ABI exported by RasgaPDF in `librasgapdf.so` (Linux), `rasgapdf.dll` (Windows), and `librasgapdf.dylib` (macOS).

---

## 1. Header & Linkage
```c
#include "rasgapdf.h"
```
Link against `-lrasgapdf` or dynamically load via `dlopen` / `LoadLibrary`.

---

## 2. Functions

### `rasgapdf_parse_raw`
```c
char* rasgapdf_parse_raw(const char* path_ptr);
```
- **Description**: Parses a PDF from a UTF-8 file path and returns flat list of physical spans (`Mode 1`).
- **Memory**: Returns a heap-allocated JSON string. The caller **must** free with `rasgapdf_free_string`.

### `rasgapdf_parse_raw_bytes`
```c
char* rasgapdf_parse_raw_bytes(const uint8_t* bytes_ptr, size_t len);
```
- **Description**: Parses a PDF directly from memory buffer (`Mode 1`). Zero disk I/O.
- **Memory**: Caller must free with `rasgapdf_free_string`.

### `rasgapdf_parse_file_options`
```c
char* rasgapdf_parse_file_options(const char* path_ptr, const char* options_json_ptr);
```
- **Description**: Parses a PDF from file path into structured blocks and matrix tables (`Mode 2`).
- **Options JSON**:
  ```json
  {
    "mode": "structured",
    "detect_tables": true,
    "min_table_columns": 3,
    "column_tolerance": 16.0,
    "row_tolerance": 6.0,
    "extract_images": false,
    "extract_drawings": false,
    "filter_background_fills": true,
    "apply_rotation": false
  }
  ```

### `rasgapdf_extract_text_bytes`
```c
char* rasgapdf_extract_text_bytes(const uint8_t* bytes_ptr, size_t len);
```
- **Description**: Extracts pure reading-order text directly from bytes in RAM.

### `rasgapdf_extract_images`
```c
char* rasgapdf_extract_images(const char* path_ptr, const char* dir_ptr);
```
- **Description**: Dumps all embedded image streams into `dir_ptr` and returns a JSON manifest.

### `rasgapdf_free_string`
```c
void rasgapdf_free_string(char* ptr);
```
- **Description**: Deallocates C strings returned by the library. Safe to call with `NULL`.

### `rasgapdf_version`
```c
const char* rasgapdf_version(void);
```
- **Description**: Returns constant version literal (e.g., `"0.3.4"`). Do **not** free.
