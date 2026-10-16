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
 *   "row_tolerance": float
 * }
 * 
 * @param path_ptr Caminho do arquivo PDF
 * @param options_json_ptr String JSON de opções, ou NULL para padrão
 * @return String JSON alocada na heap
 */
char* rasgapdf_parse_file_options(const char* path_ptr, const char* options_json_ptr);

/**
 * Libera a memória da string alocada pelo RasgaPDF na heap do Rust.
 * 
 * @param ptr Ponteiro retornado por rasgapdf_parse_*
 */
void rasgapdf_free_string(char* ptr);

/**
 * Retorna a versão da biblioteca.
 * 
 * @return String estática terminada em null (ex: "0.1.0")
 */
const char* rasgapdf_version(void);

#ifdef __cplusplus
}
#endif

#endif /* RASGAPDF_H */
