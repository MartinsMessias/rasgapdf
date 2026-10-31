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
        let x0 = if x0.is_finite() { x0 } else { 0.0 };
        let y0 = if y0.is_finite() { y0 } else { 0.0 };
        let x1 = if x1.is_finite() { x1 } else { 0.0 };
        let y1 = if y1.is_finite() { y1 } else { 0.0 };
        let width = (x1 - x0).abs();
        let height = (y1 - y0).abs();
        Self {
            x0,
            y0,
            x1,
            y1,
            width: if width.is_finite() { width } else { 0.0 },
            height: if height.is_finite() { height } else { 0.0 },
        }
    }

    /// Rotates the bounding box to match a rotated page view (0, 90, 180, 270 degrees clockwise).
    /// `page_width` and `page_height` are the dimensions of the unrotated page.
    pub fn rotate(&self, rotation: i32, page_width: f32, page_height: f32) -> Self {
        let pw = if page_width.is_finite() { page_width } else { 0.0 };
        let ph = if page_height.is_finite() { page_height } else { 0.0 };
        let norm_rot = ((rotation % 360) + 360) % 360;
        match norm_rot {
            90 => {
                let rx0 = ph - self.y1;
                let rx1 = ph - self.y0;
                let ry0 = self.x0;
                let ry1 = self.x1;
                BoundingBox::new(rx0.min(rx1), ry0.min(ry1), rx0.max(rx1), ry0.max(ry1))
            }
            180 => {
                let rx0 = pw - self.x1;
                let rx1 = pw - self.x0;
                let ry0 = ph - self.y1;
                let ry1 = ph - self.y0;
                BoundingBox::new(rx0.min(rx1), ry0.min(ry1), rx0.max(rx1), ry0.max(ry1))
            }
            270 => {
                let rx0 = self.y0;
                let rx1 = self.y1;
                let ry0 = pw - self.x1;
                let ry1 = pw - self.x0;
                BoundingBox::new(rx0.min(rx1), ry0.min(ry1), rx0.max(rx1), ry0.max(ry1))
            }
            _ => self.clone(),
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
    /// Bounding box do MediaBox bruto [x0, y0, x1, y1] em pontos PostScript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_box: Option<[f32; 4]>,
    /// Bounding box do CropBox visível [x0, y0, x1, y1] em pontos PostScript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_box: Option<[f32; 4]>,
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
    /// Normalizar bboxes de acordo com a rotação da página (/Rotate 90, 180, 270) (padrão: false).
    #[serde(default)]
    pub apply_rotation: bool,
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
            apply_rotation: false,
        }
    }
}


/// Chunk semântico espacial para RAG e citação visual determinística.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialChunk {
    pub chunk_id: usize,
    pub page_number: usize,
    pub text: String,
    pub bboxes: Vec<BoundingBox>,
    pub overall_bbox: BoundingBox,
    pub block_count: usize,
    pub token_estimate: usize,
}

impl SpatialPage {
    /// Normaliza todos os elementos geométricos da página de acordo com sua rotação nativa.
    /// Para 90 e 270 graus, a largura e altura são permutadas.
    pub fn rotated(&self) -> SpatialPage {
        if self.rotation == 0 || self.rotation % 360 == 0 {
            return self.clone();
        }
        let rot = self.rotation;
        let pw = self.width;
        let ph = self.height;
        let (new_w, new_h) = if (rot % 360).abs() == 90 || (rot % 360).abs() == 270 { (ph, pw) } else { (pw, ph) };

        let text_blocks = self.text_blocks.iter().map(|block| {
            let mut b = block.clone();
            b.bbox = b.bbox.rotate(rot, pw, ph);
            b.lines = b.lines.iter().map(|line| {
                let mut l = line.clone();
                l.bbox = l.bbox.rotate(rot, pw, ph);
                l.spans = l.spans.iter().map(|span| {
                    let mut s = span.clone();
                    s.bbox = s.bbox.rotate(rot, pw, ph);
                    s
                }).collect();
                l
            }).collect();
            b
        }).collect();

        let tables = self.tables.iter().map(|tbl| {
            let mut t = tbl.clone();
            t.bbox = t.bbox.rotate(rot, pw, ph);
            t.cells = t.cells.iter().map(|c| {
                let mut cell = c.clone();
                cell.bbox = cell.bbox.rotate(rot, pw, ph);
                cell
            }).collect();
            t
        }).collect();

        let images = self.images.iter().map(|img| {
            let mut i = img.clone();
            if let Some(ref bb) = i.bbox {
                i.bbox = Some(bb.rotate(rot, pw, ph));
            }
            i
        }).collect();

        let drawings = self.drawings.iter().map(|drw| {
            let mut d = drw.clone();
            d.bbox = d.bbox.rotate(rot, pw, ph);
            d
        }).collect();

        SpatialPage {
            page_number: self.page_number,
            width: new_w,
            height: new_h,
            rotation: 0,
            text_blocks,
            tables,
            raw_span_count: self.raw_span_count,
            images,
            drawings,
            media_box: self.media_box,
            crop_box: self.crop_box,
        }
    }

    /// Agrupa blocos de texto contíguos em chunks de até `max_chars`,
    /// preservando integralmente o array de bboxes de cada bloco constituinte.
    pub fn to_spatial_chunks(&self, max_chars: usize, overlap_blocks: usize) -> Vec<SpatialChunk> {
        let max_chars = if max_chars == 0 { 1500 } else { max_chars };
        let mut chunks = Vec::new();
        if self.text_blocks.is_empty() {
            return chunks;
        }

        let mut current_blocks: Vec<&SpatialBlock> = Vec::new();
        let mut current_len: usize = 0;
        let mut chunk_id = 1;

        for block in &self.text_blocks {
            let block_text_len: usize = block.lines.iter().map(|l| l.text.len() + 1).sum();
            if !current_blocks.is_empty() && (current_len + block_text_len > max_chars) {
                chunks.push(Self::build_chunk_from_blocks(self.page_number, chunk_id, &current_blocks));
                chunk_id += 1;

                if overlap_blocks > 0 && current_blocks.len() > overlap_blocks {
                    let keep_from = current_blocks.len() - overlap_blocks;
                    current_blocks = current_blocks[keep_from..].to_vec();
                    current_len = current_blocks.iter().map(|b| b.lines.iter().map(|l| l.text.len() + 1).sum::<usize>()).sum();
                } else if overlap_blocks == 0 {
                    current_blocks.clear();
                    current_len = 0;
                }
            }

            current_blocks.push(block);
            current_len += block_text_len;
        }

        if !current_blocks.is_empty() {
            chunks.push(Self::build_chunk_from_blocks(self.page_number, chunk_id, &current_blocks));
        }

        chunks
    }

    fn build_chunk_from_blocks(page_number: usize, chunk_id: usize, blocks: &[&SpatialBlock]) -> SpatialChunk {
        let mut text_parts = Vec::new();
        let mut bboxes = Vec::new();
        let mut min_x0 = f32::MAX;
        let mut min_y0 = f32::MAX;
        let mut max_x1 = f32::MIN;
        let mut max_y1 = f32::MIN;

        for b in blocks {
            let block_str: String = b.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
            if !block_str.trim().is_empty() {
                text_parts.push(block_str);
            }
            bboxes.push(b.bbox.clone());
            min_x0 = min_x0.min(b.bbox.x0);
            min_y0 = min_y0.min(b.bbox.y0);
            max_x1 = max_x1.max(b.bbox.x1);
            max_y1 = max_y1.max(b.bbox.y1);
        }

        let full_text = text_parts.join("\n\n");
        let token_estimate = (full_text.len() + 3) / 4;
        let overall_bbox = if min_x0 <= max_x1 {
            BoundingBox::new(min_x0, min_y0, max_x1, max_y1)
        } else {
            BoundingBox::new(0.0, 0.0, 0.0, 0.0)
        };

        SpatialChunk {
            chunk_id,
            page_number,
            text: full_text,
            bboxes,
            overall_bbox,
            block_count: blocks.len(),
            token_estimate,
        }
    }
}

impl SpatialDocument {
    /// Converte todos os blocos do documento em chunks espaciais contínuos.
    pub fn to_spatial_chunks(&self, max_chars: usize, overlap_blocks: usize) -> Vec<SpatialChunk> {
        let mut out = Vec::new();
        for page in &self.pages {
            out.extend(page.to_spatial_chunks(max_chars, overlap_blocks));
        }
        for (i, chunk) in out.iter_mut().enumerate() {
            chunk.chunk_id = i + 1;
        }
        out
    }

    /// Retorna uma cópia do documento com todas as páginas e coordenadas normalizadas pós-rotação.
    pub fn with_applied_rotation(&self) -> SpatialDocument {
        SpatialDocument {
            file_name: self.file_name.clone(),
            total_pages: self.total_pages,
            parse_time_ms: self.parse_time_ms,
            pages: self.pages.iter().map(|p| p.rotated()).collect(),
        }
    }
}
