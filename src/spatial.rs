use std::path::Path;
use std::time::Instant;

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Document, TextPageFlags};
use rayon::prelude::*;

use crate::drawing;
use crate::image;
use crate::table::{RawSpan, TableDetector};
use crate::types::*;

/// DTOs mínimos do JSON de texto estruturado produzido pelo MuPDF.
///
/// Substituem `mupdf::page::StextPage`: o `Block` daquele crate exige o campo `lines`,
/// que **não existe** nos blocos de imagem (`{"type":"image","bbox":{...}}`) nem nos de
/// estrutura. Com o tipo do crate, a desserialização falhava e a página inteira era
/// descartada — apagando todo o texto de qualquer página que contivesse uma imagem.
#[derive(serde::Deserialize)]
struct RawStextPage {
    #[serde(default)]
    blocks: Vec<RawBlock>,
}

#[derive(serde::Deserialize)]
struct RawBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    bbox: Option<RawBBox>,
    #[serde(default)]
    lines: Vec<RawLine>,
}

#[derive(serde::Deserialize)]
struct RawBBox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl RawBBox {
    #[inline]
    fn x0(&self) -> f32 {
        self.x as f32
    }

    #[inline]
    fn y0(&self) -> f32 {
        self.y as f32
    }

    #[inline]
    fn x1(&self) -> f32 {
        (self.x + self.w) as f32
    }

    #[inline]
    fn y1(&self) -> f32 {
        (self.y + self.h) as f32
    }

    #[inline]
    fn to_bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.x0(), self.y0(), self.x1(), self.y1())
    }
}

#[derive(serde::Deserialize)]
struct RawLine {
    #[serde(default)]
    text: String,
    #[serde(default)]
    bbox: Option<RawBBox>,
    #[serde(default)]
    font: Option<RawFont>,
}

#[derive(serde::Deserialize)]
struct RawFont {
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: f64,
}

/// União dos bounding boxes das linhas, usada quando o bloco não traz `bbox`.
fn lines_bounding_box(lines: &[SpatialLine]) -> BoundingBox {
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y1 = f32::MIN;

    for line in lines {
        x0 = x0.min(line.bbox.x0);
        y0 = y0.min(line.bbox.y0);
        x1 = x1.max(line.bbox.x1);
        y1 = y1.max(line.bbox.y1);
    }

    if x0 > x1 {
        BoundingBox::new(0.0, 0.0, 0.0, 0.0)
    } else {
        BoundingBox::new(x0, y0, x1, y1)
    }
}

/// Modo 1: Parse Bruto Espacial (Raw X/Y)
/// Retorna exclusivamente a lista plana e direta de coordenadas físicas de cada elemento
pub fn parse_raw_file(pdf_path: &str) -> Result<RawSpatialDocument, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", pdf_path).into());
    }

    let file_name = path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.pdf".to_string());

    let doc = Document::open(pdf_path)?;
    let total_pages = doc.page_count()? as usize;

    let pages_spans: Vec<Vec<RawCoordSpan>> = (0..total_pages)
        .into_par_iter()
        .map(|page_idx| {
            let local_doc = match Document::open(pdf_path) {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };
            let page = match local_doc.load_page(page_idx as i32) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            let text_page = match page.to_text_page(TextPageFlags::empty()) {
                Ok(tp) => tp,
                Err(_) => return Vec::new(),
            };
            let json_str = match text_page.to_json(1.0) {
                Ok(j) => j,
                Err(_) => return Vec::new(),
            };
            let stext: RawStextPage = match serde_json::from_str(&json_str) {
                Ok(st) => st,
                Err(_) => return Vec::new(),
            };

            let mut spans = Vec::new();
            for block in stext.blocks {
                if block.block_type != "text" {
                    continue;
                }
                for line in block.lines {
                    let text = line.text.trim().to_string();
                    if !text.is_empty() {
                        let Some(line_bbox) = line.bbox.as_ref() else {
                            continue;
                        };
                        spans.push(RawCoordSpan {
                            page: page_idx + 1,
                            x0: line_bbox.x0(),
                            y0: line_bbox.y0(),
                            x1: line_bbox.x1(),
                            y1: line_bbox.y1(),
                            font_size: line.font.as_ref().map(|f| f.size as f32).unwrap_or(0.0),
                            font_name: line.font.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
                            text,
                        });
                    }
                }
            }

            // Ordenação espacial estrita por linha (Y) e depois horizontal (X)
            spans.sort_by(|a, b| {
                let y_bucket_a = (a.y0 / 4.0).round() as i32;
                let y_bucket_b = (b.y0 / 4.0).round() as i32;
                if y_bucket_a != y_bucket_b {
                    y_bucket_a.cmp(&y_bucket_b)
                } else {
                    a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            spans
        })
        .collect();

    let mut all_spans = Vec::new();
    for page_spans in pages_spans {
        all_spans.extend(page_spans);
    }

    let parse_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total_spans = all_spans.len();

    Ok(RawSpatialDocument {
        file_name,
        total_pages,
        total_spans,
        parse_time_ms,
        spans: all_spans,
    })
}

/// Modo 2: Parse Estruturado (Hierárquico com Blocos e Tabelas)
pub fn parse_file(pdf_path: &str, options: &ParserOptions) -> Result<SpatialDocument, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", pdf_path).into());
    }

    let file_name = path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.pdf".to_string());

    let doc = Document::open(pdf_path)?;
    let total_pages = doc.page_count()? as usize;

    let detector = TableDetector::new(
        options.min_table_columns,
        options.column_tolerance,
        options.row_tolerance,
    );

    // Paralelização por página via Rayon
    let pages: Vec<SpatialPage> = (0..total_pages)
        .into_par_iter()
        .map(|page_idx| {
            let local_doc = match Document::open(pdf_path) {
                Ok(d) => d,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let page = match local_doc.load_page(page_idx as i32) {
                Ok(p) => p,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let bounds = page.bounds().unwrap_or(mupdf::Rect::new(0.0, 0.0, 595.0, 842.0));
            let page_width = (bounds.x1 - bounds.x0).abs();
            let page_height = (bounds.y1 - bounds.y0).abs();

            // `PRESERVE_IMAGES` faz o MuPDF emitir blocos `{"type":"image"}` no JSON de texto,
            // permitindo correlacionar os metadados da imagem com seu retângulo de pintura.
            let text_flags = if options.extract_images {
                TextPageFlags::PRESERVE_IMAGES
            } else {
                TextPageFlags::empty()
            };

            let text_page = match page.to_text_page(text_flags) {
                Ok(tp) => tp,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let json_str = match text_page.to_json(1.0) {
                Ok(j) => j,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let stext: RawStextPage = match serde_json::from_str(&json_str) {
                Ok(st) => st,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let mut raw_spans: Vec<RawSpan> = Vec::new();
            let mut text_blocks: Vec<SpatialBlock> = Vec::new();
            let mut image_block_boxes: Vec<BoundingBox> = Vec::new();
            let mut block_counter = 1;

            for block in stext.blocks {
                // Blocos de imagem: sem `lines`, apenas o retângulo de posicionamento.
                // Usados como aproximação do bbox das imagens embutidas.
                if block.block_type == "image" {
                    if let Some(block_bbox) = block.bbox.as_ref() {
                        image_block_boxes.push(block_bbox.to_bounding_box());
                    }
                    continue;
                }

                if block.block_type != "text" {
                    continue;
                }

                let block_bbox = block.bbox.as_ref().map(RawBBox::to_bounding_box);
                let mut lines: Vec<SpatialLine> = Vec::new();

                for line in block.lines {
                    let Some(line_bbox) = line.bbox.as_ref() else {
                        continue;
                    };

                    let text = line.text.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }

                    let x0 = line_bbox.x0();
                    let y0 = line_bbox.y0();
                    let x1 = line_bbox.x1();
                    let y1 = line_bbox.y1();
                    let font_size = line.font.as_ref().map(|f| f.size as f32).unwrap_or(0.0);
                    let font_name = line.font.as_ref().map(|f| f.name.clone()).unwrap_or_default();

                    raw_spans.push(RawSpan {
                        x0,
                        y0,
                        x1,
                        y1,
                        font_size,
                        text: text.clone(),
                    });

                    let span = SpatialSpan {
                        text: text.clone(),
                        bbox: BoundingBox::new(x0, y0, x1, y1),
                        font_name,
                        font_size,
                        char_count: text.chars().count(),
                    };

                    lines.push(SpatialLine {
                        text,
                        bbox: BoundingBox::new(x0, y0, x1, y1),
                        spans: vec![span],
                    });
                }

                if !lines.is_empty() {
                    let bbox = block_bbox.unwrap_or_else(|| lines_bounding_box(&lines));

                    text_blocks.push(SpatialBlock {
                        block_id: block_counter,
                        block_type: "text".to_string(),
                        bbox,
                        lines,
                    });
                    block_counter += 1;
                }
            }

            // Ordena spans espacialmente (Top-to-Bottom, Left-to-Right)
            raw_spans.sort_by(|a, b| {
                let y_bucket_a = (a.y0 / options.row_tolerance).round() as i32;
                let y_bucket_b = (b.y0 / options.row_tolerance).round() as i32;
                if y_bucket_a != y_bucket_b {
                    y_bucket_a.cmp(&y_bucket_b)
                } else {
                    a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            // Extração de tabelas espaciais
            let (tables, _) = if options.detect_tables {
                detector.extract_tables(&raw_spans, 1)
            } else {
                (Vec::new(), raw_spans.clone())
            };

            // --- Gráficos (opcional): vetores e imagens embutidas ---
            let mut drawings: Vec<SpatialDrawing> = Vec::new();
            let mut images: Vec<SpatialImage> = Vec::new();
            let mut rotation: i32 = 0;

            if options.extract_drawings {
                if let Ok(found) = page.drawings() {
                    drawings = drawing::map_drawings(
                        &found,
                        page_width,
                        page_height,
                        options.filter_background_fills,
                    );
                }
            }

            // A conversão em `PdfPage` também dá acesso à rotação real da página.
            if let Ok(pdf_page) = PdfPage::try_from(page) {
                rotation = pdf_page.rotation().unwrap_or(0);

                if options.extract_images {
                    if let Ok(pdf_doc) = PdfDocument::try_from(local_doc) {
                        if let Ok(infos) = pdf_page.images() {
                            for (idx, info) in infos.iter().enumerate() {
                                let bytes = image::extract_image_bytes(&pdf_doc, info.xref);
                                let bbox = image_block_boxes.get(idx).cloned();
                                images.push(image::build_spatial_image(
                                    idx + 1,
                                    info,
                                    bbox,
                                    bytes.as_deref(),
                                ));
                            }
                        }
                    }
                }
            }

            SpatialPage {
                page_number: page_idx + 1,
                width: page_width,
                height: page_height,
                rotation,
                text_blocks,
                tables,
                raw_span_count: raw_spans.len(),
                images,
                drawings,
            }
        })
        .collect();

    let parse_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(SpatialDocument {
        file_name,
        total_pages,
        parse_time_ms,
        pages,
    })
}

fn fallback_empty_page(page_number: usize) -> SpatialPage {
    SpatialPage {
        page_number,
        width: 595.0,
        height: 842.0,
        rotation: 0,
        text_blocks: Vec::new(),
        tables: Vec::new(),
        raw_span_count: 0,
        images: Vec::new(),
        drawings: Vec::new(),
    }
}


/// Modo 1 (Em Memória): Parse Bruto Espacial a partir de slice de bytes (`&[u8]`)
pub fn parse_raw_bytes(pdf_bytes: &[u8]) -> Result<RawSpatialDocument, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();
    let doc = Document::from_bytes(pdf_bytes, "application/pdf")?;
    let total_pages = doc.page_count()? as usize;

    let pages_spans: Vec<Vec<RawCoordSpan>> = (0..total_pages)
        .into_par_iter()
        .map(|page_idx| {
            let local_doc = match Document::from_bytes(pdf_bytes, "application/pdf") {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };
            let page = match local_doc.load_page(page_idx as i32) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            let text_page = match page.to_text_page(TextPageFlags::empty()) {
                Ok(tp) => tp,
                Err(_) => return Vec::new(),
            };
            let json_str = match text_page.to_json(1.0) {
                Ok(j) => j,
                Err(_) => return Vec::new(),
            };
            let stext: RawStextPage = match serde_json::from_str(&json_str) {
                Ok(st) => st,
                Err(_) => return Vec::new(),
            };

            let mut spans = Vec::new();
            for block in stext.blocks {
                if block.block_type != "text" {
                    continue;
                }
                for line in block.lines {
                    let text = line.text.trim().to_string();
                    if !text.is_empty() {
                        let Some(line_bbox) = line.bbox.as_ref() else {
                            continue;
                        };
                        spans.push(RawCoordSpan {
                            page: page_idx + 1,
                            x0: line_bbox.x0(),
                            y0: line_bbox.y0(),
                            x1: line_bbox.x1(),
                            y1: line_bbox.y1(),
                            font_size: line.font.as_ref().map(|f| f.size as f32).unwrap_or(0.0),
                            font_name: line.font.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
                            text,
                        });
                    }
                }
            }

            spans.sort_by(|a, b| {
                let y_bucket_a = (a.y0 / 4.0).round() as i32;
                let y_bucket_b = (b.y0 / 4.0).round() as i32;
                if y_bucket_a != y_bucket_b {
                    y_bucket_a.cmp(&y_bucket_b)
                } else {
                    a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            spans
        })
        .collect();

    let mut all_spans = Vec::new();
    for page_spans in pages_spans {
        all_spans.extend(page_spans);
    }

    let parse_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total_spans = all_spans.len();

    Ok(RawSpatialDocument {
        file_name: "memory.pdf".to_string(),
        total_pages,
        total_spans,
        parse_time_ms,
        spans: all_spans,
    })
}

/// Extração paralela ultrarrápida de texto puro em memória (Rayon + MuPDF)
/// com expansão automática de ligaduras e desifenização.
pub fn extract_text_bytes(pdf_bytes: &[u8]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let doc = Document::from_bytes(pdf_bytes, "application/pdf")?;
    let total_pages = doc.page_count()? as usize;

    let mut out = String::with_capacity(total_pages * 2048);
    let mut consecutive_newlines = 0;

    for page_idx in 0..total_pages {
        if let Ok(page) = doc.load_page(page_idx as i32) {
            if let Ok(text_page) = page.to_text_page(TextPageFlags::empty()) {
                if let Ok(t) = text_page.to_text() {
                    for line in t.lines() {
                        let trimmed = line.trim_end();
                        if trimmed.is_empty() {
                            if consecutive_newlines < 2 && !out.is_empty() {
                                out.push('\n');
                                consecutive_newlines += 1;
                            }
                        } else {
                            if !out.is_empty() && consecutive_newlines == 0 {
                                out.push('\n');
                            }
                            out.push_str(trimmed);
                            consecutive_newlines = 0;
                        }
                    }
                    if !out.is_empty() && consecutive_newlines == 0 {
                        out.push('\n');
                        consecutive_newlines = 1;
                    }
                }
            }
        }
    }

    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }

    Ok(out)
}
