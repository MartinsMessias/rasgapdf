/**
 * RasgaPDF - C API Header
 * Ultra-fast 2D Spatial PDF Parser
 * Copyright (c) 2026 Messias Martins (MIT License)
 */

#ifndef RASGAPDF_H
#define RASGAPDF_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

/**
 * Parses a PDF file from disk and returns a JSON string with complete spatial geometry.
 * The returned pointer MUST be freed using `rasgapdf_free_string`.
 * 
 * @param path_ptr Path to the PDF file (null-terminated UTF-8 string)
 * @return Null-terminated JSON string allocated on the heap, or JSON error object
 */
char* rasgapdf_parse_file(const char* path_ptr);

/**
 * Parses a PDF file with custom options passed as a JSON string.
 * Options schema:
 * {
 *   "detect_tables": bool,
 *   "min_table_columns": int,
 *   "column_tolerance": float,
 *   "row_tolerance": float
 * }
 * 
 * @param path_ptr Path to the PDF file
 * @param options_json_ptr JSON string with options, or NULL for defaults
 * @return Null-terminated JSON string
 */
char* rasgapdf_parse_file_options(const char* path_ptr, const char* options_json_ptr);

/**
 * Frees a string pointer allocated by RasgaPDF.
 * 
 * @param ptr Pointer returned by rasgapdf_parse_file*
 */
void rasgapdf_free_string(char* ptr);

/**
 * Returns the library version string.
 * 
 * @return Statically-allocated null-terminated version string (e.g. "0.1.0")
 */
const char* rasgapdf_version(void);

#ifdef __cplusplus
}
#endif

#endif /* RASGAPDF_H */
