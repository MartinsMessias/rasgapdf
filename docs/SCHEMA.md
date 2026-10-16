# 📐 RasgaPDF Spatial Schema Specification

RasgaPDF provides **two distinct parsing modes**:
1. **Modo 1: Raw Spatial X/Y Coordinates (`RawSpatialDocument`)** — Flat, zero-overhead physical coordinates list `[page, x0, y0, x1, y1, text]` for maximum speed (~3-8 ms).
2. **Modo 2: Structured Hierarchy (`SpatialDocument`)** — Geometric tree containing visual blocks, lines, spans, and automatic table matrix detection.

---

## Modo 1: Coordenadas Brutas (`RawSpatialDocument`)

Retorna a lista plana de cada elemento físico desenhado na página, sem criar hierarquias de parágrafos ou tabelas. Ideal para indexação vetorial, spatial visualizers, OCR alignment e pipelines de parsing customizados.

```json
{
  "file_name": "document.pdf",
  "total_pages": 1,
  "total_spans": 20,
  "parse_time_ms": 7.78,
  "spans": [
    {
      "page": 1,
      "x0": 50.0,
      "y0": 67.0,
      "x1": 244.0,
      "y1": 83.0,
      "font_size": 12.0,
      "font_name": "Helvetica",
      "text": "A Revolução da Inteligência Artificial"
    }
  ]
}
```

### Campos do `RawCoordSpan`:
| Campo | Tipo | Descrição |
| :--- | :--- | :--- |
| `page` | `integer` | Número da página (iniciando em 1) |
| `x0` | `float` | Posição horizontal esquerda (pontos tipográficos) |
| `y0` | `float` | Posição vertical superior (pontos tipográficos) |
| `x1` | `float` | Posição horizontal direita |
| `y1` | `float` | Posição vertical inferior |
| `font_size` | `float` | Tamanho da fonte em pontos |
| `font_name` | `string` | Nome da fonte utilizada no PDF |
| `text` | `string` | Texto bruto extraído |

---

## Modo 2: Estruturado Hierárquico (`SpatialDocument`)

```json
{
  "file_name": "sample_document.pdf",
  "total_pages": 3,
  "parse_time_ms": 12.45,
  "pages": [ ... ]
}
```

| Campo | Tipo | Descrição |
| :--- | :--- | :--- |
| `file_name` | `string` | Nome base do arquivo PDF |
| `total_pages` | `integer` | Total de páginas processadas |
| `parse_time_ms` | `float` | Tempo nativo de execução em milissegundos |
| `pages` | `array<SpatialPage>` | Lista de páginas do documento |

---

### Nível de Página: `SpatialPage`

```json
{
  "page_number": 1,
  "width": 595.0,
  "height": 842.0,
  "rotation": 0,
  "text_blocks": [ ... ],
  "tables": [ ... ],
  "raw_span_count": 142
}
```

---

### Nível de Bloco e Linha: `SpatialBlock` & `SpatialLine`

```json
{
  "block_id": 1,
  "block_type": "text",
  "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
  "lines": [
    {
      "text": "Invoice #450091",
      "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
      "spans": [
        {
          "text": "Invoice #450091",
          "bbox": { "x0": 50.0, "y0": 67.0, "x1": 244.0, "y1": 83.0, "width": 194.0, "height": 16.0 },
          "font_name": "Helvetica-Bold",
          "font_size": 16.0,
          "char_count": 15
        }
      ]
    }
  ]
}
```

---

### Nível de Tabela: `SpatialTable` & `SpatialCell`

```json
{
  "table_id": 1,
  "bbox": { "x0": 45.0, "y0": 200.0, "x1": 550.0, "y1": 380.0, "width": 505.0, "height": 180.0 },
  "col_count": 4,
  "row_count": 5,
  "headers": ["Item", "Quantity", "Unit Price", "Total"],
  "rows": [
    ["Item", "Quantity", "Unit Price", "Total"],
    ["Coffee Beans 1kg", "10", "$15.00", "$150.00"],
    ["Filter Paper", "5", "$4.00", "$20.00"]
  ],
  "cells": [
    {
      "row": 0,
      "col": 0,
      "text": "Item",
      "bbox": { "x0": 45.0, "y0": 200.0, "x1": 180.0, "y1": 220.0, "width": 135.0, "height": 20.0 }
    }
  ]
}
```
