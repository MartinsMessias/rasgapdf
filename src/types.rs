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

/// Item geométrico individual de um caminho vetorial (paridade com `Page.get_drawings()` do PyMuPDF).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpatialDrawingItem {
    /// Segmento de reta entre dois pontos.
    Line { p1: [f32; 2], p2: [f32; 2] },
    /// Curva de Bézier cúbica: `p1`/`p4` são as extremidades, `p2`/`p3` os controles.
    Curve {
        p1: [f32; 2],
        p2: [f32; 2],
        p3: [f32; 2],
        p4: [f32; 2],
    },
    /// Retângulo alinhado aos eixos. `orientation` é `1` (anti-horário) ou `-1` (horário).
    Rect { bbox: BoundingBox, orientation: i32 },
    /// Quadrilátero arbitrário (4 pontos na ordem ul, ur, ll, lr).
    Quad { points: [[f32; 2]; 4] },
}

/// Caminho vetorial pintado em uma página (fill, stroke ou ambos).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialDrawing {
    pub drawing_id: usize,
    pub bbox: BoundingBox,
    /// "fill" | "stroke" | "fill_stroke"
    pub drawing_type: String,
    pub item_count: usize,
    pub items: Vec<SpatialDrawingItem>,
    /// Cor de traço RGB normalizada (`0.0..=1.0`), quando houver.
    pub stroke_color: Option<[f32; 3]>,
    /// Cor de preenchimento RGB normalizada (`0.0..=1.0`), quando houver.
    pub fill_color: Option<[f32; 3]>,
    pub width: Option<f32>,
    pub fill_opacity: Option<f32>,
    pub stroke_opacity: Option<f32>,
    pub even_odd: Option<bool>,
    /// Padrão de tracejado (ex.: `[3.0, 3.0]`).
    pub dash_pattern: Option<Vec<f32>>,
    /// Fase do tracejado (offset inicial).
    pub dash_phase: Option<f32>,
    pub line_join: Option<String>,
    pub line_cap: Option<String>,
    /// Nome da camada de conteúdo opcional (OCG), quando presente.
    pub layer: Option<String>,
    /// Ordem de pintura dentro da página (`seqno`).
    pub seqno: usize,
}

/// Imagem embutida (`/Subtype /Image`) referenciada pelos recursos da página.
///
/// Os bytes **não** são incluídos aqui: use `rasgapdf::extract_images` (API Rust) ou
/// a flag `--extract-images <DIR>` (CLI) para gravar o stream original em disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialImage {
    pub image_id: usize,
    /// Nome do recurso XObject na página (ex.: `"Im0"`).
    pub name: String,
    /// Xref do objeto de imagem no PDF (0 quando não aplicável).
    pub xref: i32,
    /// Retângulo de posicionamento na página, quando conhecido (ver ressalva no README).
    pub bbox: Option<BoundingBox>,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub bits_per_component: Option<i32>,
    pub color_space: Option<String>,
    /// Filtro de codificação do stream (ex.: `"DCTDecode"`, `"FlateDecode"`).
    pub filter: Option<String>,
    /// MIME type derivado do filtro, quando o stream é um arquivo de imagem direto.
    pub mime_type: Option<String>,
    /// Tamanho em bytes do stream codificado original.
    pub byte_length: Option<usize>,
    /// SHA-256 (hex) do stream codificado original.
    pub sha256: Option<String>,
    /// Arquivo gerado em disco (preenchido apenas pela CLI com `--extract-images`).
    pub file: Option<String>,
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
    /// Imagens embutidas (vazio salvo quando `extract_images` está ativo).
    #[serde(default)]
    pub images: Vec<SpatialImage>,
    /// Caminhos vetoriais (vazio salvo quando `extract_drawings` está ativo).
    #[serde(default)]
    pub drawings: Vec<SpatialDrawing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialDocument {
    pub file_name: String,
    pub total_pages: usize,
    pub parse_time_ms: f64,
    pub pages: Vec<SpatialPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParserOptions {
    pub mode: String, // "structured" (default) ou "raw"
    pub detect_tables: bool,
    pub min_table_columns: usize,
    pub column_tolerance: f32,
    pub row_tolerance: f32,
    /// Extrair imagens embutidas referenciadas pelas páginas (desligado por padrão).
    pub extract_images: bool,
    /// Extrair caminhos vetoriais preenchidos/contornados (desligado por padrão).
    pub extract_drawings: bool,
    /// Descartar preenchimentos que cobrem praticamente a página inteira (fundos).
    pub filter_background_fills: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            mode: "structured".to_string(),
            detect_tables: true,
            min_table_columns: 3,
            column_tolerance: 16.0,
            row_tolerance: 6.0,
            extract_images: false,
            extract_drawings: false,
            filter_background_fills: true,
        }
    }
}
