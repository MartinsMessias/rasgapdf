use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            width: (x1 - x0).abs(),
            height: (y1 - y0).abs(),
        }
    }
}

/// Item de coordenada 2D mais bruto e direto possível
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCoordSpan {
    pub page: usize,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub font_size: f32,
    pub font_name: String,
    pub text: String,
}

/// Documento bruto contendo exclusivamente a lista plana de coordenadas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSpatialDocument {
    pub file_name: String,
    pub total_pages: usize,
    pub total_spans: usize,
    pub parse_time_ms: f64,
    pub spans: Vec<RawCoordSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialSpan {
    pub text: String,
    pub bbox: BoundingBox,
    pub font_name: String,
    pub font_size: f32,
    pub char_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialLine {
    pub text: String,
    pub bbox: BoundingBox,
    pub spans: Vec<SpatialSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialBlock {
    pub block_id: usize,
    pub block_type: String, // "text" | "table"
    pub bbox: BoundingBox,
    pub lines: Vec<SpatialLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialCell {
    pub row: usize,
    pub col: usize,
    pub text: String,
    pub bbox: BoundingBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialTable {
    pub table_id: usize,
    pub bbox: BoundingBox,
    pub col_count: usize,
    pub row_count: usize,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub cells: Vec<SpatialCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialPage {
    pub page_number: usize, // 1-indexed
    pub width: f32,
    pub height: f32,
    pub rotation: i32,
    pub text_blocks: Vec<SpatialBlock>,
    pub tables: Vec<SpatialTable>,
    pub raw_span_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialDocument {
    pub file_name: String,
    pub total_pages: usize,
    pub parse_time_ms: f64,
    pub pages: Vec<SpatialPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserOptions {
    pub mode: String, // "structured" (default) ou "raw"
    pub detect_tables: bool,
    pub min_table_columns: usize,
    pub column_tolerance: f32,
    pub row_tolerance: f32,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            mode: "structured".to_string(),
            detect_tables: true,
            min_table_columns: 3,
            column_tolerance: 16.0,
            row_tolerance: 6.0,
        }
    }
}
