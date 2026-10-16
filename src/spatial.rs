use std::path::Path;
use std::time::Instant;
use mupdf::{Document, TextPageFlags};
use mupdf::page::StextPage;
use rayon::prelude::*;

use crate::types::*;
use crate::table::{RawSpan, TableDetector};

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
            let stext: StextPage = match serde_json::from_str(&json_str) {
                Ok(st) => st,
                Err(_) => return Vec::new(),
            };

            let mut spans = Vec::new();
            for block in stext.blocks {
                if block.r#type == "text" {
                    for line in block.lines {
                        let text = line.text.trim().to_string();
                        if !text.is_empty() {
                            let x0 = line.bbox.x as f32;
                            let y0 = line.bbox.y as f32;
                            let x1 = x0 + line.bbox.w as f32;
                            let y1 = y0 + line.bbox.h as f32;
                            spans.push(RawCoordSpan {
                                page: page_idx + 1,
                                x0,
                                y0,
                                x1,
                                y1,
                                font_size: line.font.size as f32,
                                font_name: line.font.name,
                                text,
                            });
                        }
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

            let text_page = match page.to_text_page(TextPageFlags::empty()) {
                Ok(tp) => tp,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let json_str = match text_page.to_json(1.0) {
                Ok(j) => j,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let stext: StextPage = match serde_json::from_str(&json_str) {
                Ok(st) => st,
                Err(_) => return fallback_empty_page(page_idx + 1),
            };

            let mut raw_spans: Vec<RawSpan> = Vec::new();
            let mut text_blocks: Vec<SpatialBlock> = Vec::new();
            let mut block_counter = 1;

            for block in stext.blocks {
                if block.r#type == "text" {
                    let mut lines: Vec<SpatialLine> = Vec::new();

                    for line in block.lines {
                        let text = line.text.trim().to_string();
                        if text.is_empty() {
                            continue;
                        }

                        let x0 = line.bbox.x as f32;
                        let y0 = line.bbox.y as f32;
                        let x1 = x0 + line.bbox.w as f32;
                        let y1 = y0 + line.bbox.h as f32;
                        let font_size = line.font.size as f32;

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
                            font_name: line.font.name.clone(),
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
                        let bx0 = block.bbox.x as f32;
                        let by0 = block.bbox.y as f32;
                        let bx1 = bx0 + block.bbox.w as f32;
                        let by1 = by0 + block.bbox.h as f32;

                        text_blocks.push(SpatialBlock {
                            block_id: block_counter,
                            block_type: "text".to_string(),
                            bbox: BoundingBox::new(bx0, by0, bx1, by1),
                            lines,
                        });
                        block_counter += 1;
                    }
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

            SpatialPage {
                page_number: page_idx + 1,
                width: page_width,
                height: page_height,
                rotation: 0,
                text_blocks,
                tables,
                raw_span_count: raw_spans.len(),
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
    }
}
