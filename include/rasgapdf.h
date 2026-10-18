/**
 * RasgaPDF - C API Header
 * Ultra-fast 2D Spatial PDF Parser
 * Copyright (c) 2026 Messias Martins (AGPL-3.0-or-later)
 */

#ifndef RASGAPDF_H
#define RASGAPDF_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

/**
 * Modo 1 (Bruto): Retorna exclusivamente a lista plana de coordenadas X/Y
 * no formato {"file_name": "...", "spans": [{"page": 1, "x0": ..., "y0": ..., "x1": ..., "y1": ..., "text": "..."}]}.
 * O chamador DEVE liberar a memória chamando `rasgapdf_free_string`.
 * 
 * @param path_ptr Caminho do arquivo PDF (UTF-8 terminado em null)
 * @return String JSON alocada na heap
 */
char* rasgapdf_parse_raw(const char* path_ptr);

/**
 * Modo 2 (Estruturado): Processa o PDF e retorna o JSON espacial hierárquico
 * com blocos de texto, linhas, spans e detecção automática de tabelas.
 * O chamador DEVE liberar a memória chamando `rasgapdf_free_string`.
 * 
 * @param path_ptr Caminho do arquivo PDF (UTF-8 terminado em null)
 * @return String JSON alocada na heap
 */
char* rasgapdf_parse_file(const char* path_ptr);

/**
 * Modo 2 (Estruturado com Opções): Processa um PDF com opções JSON customizadas.
 * Exemplo de opções:
 * {
 *   "mode": "structured" | "raw",
 *   "detect_tables": bool,
 *   "min_table_columns": int,
 *   "column_tolerance": float,
 *   "row_tolerance": float,
 *   "extract_images": bool,
 *   "extract_drawings": bool,
 *   "filter_background_fills": bool
 * }
 * Campos ausentes usam o padrão (extract_images/extract_drawings = false).
 *
 * @param path_ptr Caminho do arquivo PDF
 * @param options_json_ptr String JSON de opções, ou NULL para padrão
 * @return String JSON alocada na heap
 */
char* rasgapdf_parse_file_options(const char* path_ptr, const char* options_json_ptr);

/**
 * Extrai os streams originais das imagens embutidas para `dir_ptr` e grava um
 * `manifest.json` na mesma pasta. Nenhum pixel é decodificado: o stream codificado
 * é copiado como está (ex.: `DCTDecode` -> `.jpg`).
 *
 * Retorna um manifesto JSON na heap:
 * {
 *   "file_name": "...",
 *   "total_images": int,
 *   "written": int,
 *   "images": [ { "page": int, "image": { ..., "file": "page_001_img_001_xref7.jpg" } } ]
 * }
 * O chamador DEVE liberar a memória chamando `rasgapdf_free_string`.
 *
 * @param path_ptr Caminho do arquivo PDF (UTF-8 terminado em null)
 * @param dir_ptr Pasta destino (criada se não existir)
 * @return String JSON alocada na heap
 */
char* rasgapdf_extract_images(const char* path_ptr, const char* dir_ptr);

/**
 * Libera a memória da string alocada pelo RasgaPDF na heap do Rust.
 * 
 * @param ptr Ponteiro retornado por rasgapdf_parse_*
 */
void rasgapdf_free_string(char* ptr);

/**
 * Retorna a versão da biblioteca.
 * 
 * @return String estática terminada em null (ex: "0.1.3")
 */
const char* rasgapdf_version(void);

#ifdef __cplusplus
}
#endif

#endif /* RASGAPDF_H */
