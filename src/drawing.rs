//! Extração de gráficos vetoriais (caminhos preenchidos/contornados).
//!
//! Mapeia o `Page::drawings()` do `mupdf` (equivalente ao `Page.get_drawings()` do PyMuPDF)
//! para os tipos serializáveis do RasgaPDF.

use mupdf::drawing::{Drawing, DrawingItem, DrawingType};
use mupdf::{LineCap, LineJoin, Point, Rect};

use crate::types::{BoundingBox, SpatialDrawing, SpatialDrawingItem};

#[inline]
fn point(p: Point) -> [f32; 2] {
    [p.x, p.y]
}

#[inline]
fn bounding_box(rect: Rect) -> BoundingBox {
    BoundingBox::new(rect.x0, rect.y0, rect.x1, rect.y1)
}

fn drawing_type_name(drawing_type: DrawingType) -> &'static str {
    match drawing_type {
        DrawingType::Fill => "fill",
        DrawingType::Stroke => "stroke",
        DrawingType::FillStroke => "fill_stroke",
    }
}

fn line_cap_name(cap: LineCap) -> &'static str {
    match cap {
        LineCap::Butt => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
        LineCap::Triangle => "triangle",
    }
}

fn line_join_name(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "miter",
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
        LineJoin::MiterXps => "miter_xps",
    }
}

/// Converte um `Drawing` do MuPDF no tipo serializável do RasgaPDF.
pub fn map_drawing(drawing: &Drawing, drawing_id: usize) -> SpatialDrawing {
    let items: Vec<SpatialDrawingItem> = drawing
        .items
        .iter()
        .map(|item| match item {
            DrawingItem::Line(a, b) => SpatialDrawingItem::Line {
                p1: point(*a),
                p2: point(*b),
            },
            DrawingItem::Curve(a, b, c, d) => SpatialDrawingItem::Curve {
                p1: point(*a),
                p2: point(*b),
                p3: point(*c),
                p4: point(*d),
            },
            DrawingItem::Rect { rect, orientation } => SpatialDrawingItem::Rect {
                bbox: bounding_box(*rect),
                orientation: *orientation,
            },
            DrawingItem::Quad(quad) => SpatialDrawingItem::Quad {
                points: [
                    point(quad.ul),
                    point(quad.ur),
                    point(quad.ll),
                    point(quad.lr),
                ],
            },
        })
        .collect();

    SpatialDrawing {
        drawing_id,
        bbox: bounding_box(drawing.rect),
        drawing_type: drawing_type_name(drawing.drawing_type).to_string(),
        item_count: items.len(),
        items,
        stroke_color: drawing.color,
        fill_color: drawing.fill,
        width: drawing.width,
        fill_opacity: drawing.fill_opacity,
        stroke_opacity: drawing.stroke_opacity,
        even_odd: drawing.even_odd,
        dash_pattern: drawing.dashes.as_ref().map(|d| d.dashes.clone()),
        dash_phase: drawing.dashes.as_ref().map(|d| d.phase),
        line_join: drawing.line_join.map(line_join_name).map(str::to_string),
        line_cap: drawing
            .line_cap
            .map(|(start, _, end)| {
                if start == end {
                    line_cap_name(start).to_string()
                } else {
                    format!("{}/{}", line_cap_name(start), line_cap_name(end))
                }
            }),
        layer: drawing.layer.clone(),
        seqno: drawing.seqno,
    }
}

/// Indica se o desenho é essencialmente um fundo que cobre a página inteira.
///
/// Usado para descartar retângulos de fundo (brancos/coloridos) que não agregam
/// informação espacial, mantendo o sinal útil (linhas, bordas, formas).
pub fn is_background_fill(drawing: &SpatialDrawing, page_width: f32, page_height: f32) -> bool {
    if drawing.drawing_type == "stroke" {
        return false;
    }

    let page_area = (page_width * page_height).abs();
    if page_area <= 0.0 {
        return false;
    }

    let area = (drawing.bbox.width * drawing.bbox.height).abs();
    area >= 0.95 * page_area
}

/// Converte e normaliza a lista de desenhos de uma página.
pub fn map_drawings(
    drawings: &[Drawing],
    page_width: f32,
    page_height: f32,
    filter_backgrounds: bool,
) -> Vec<SpatialDrawing> {
    drawings
        .iter()
        .map(|d| map_drawing(d, 0))
        .filter(|d| !(filter_backgrounds && is_background_fill(d, page_width, page_height)))
        .enumerate()
        .map(|(idx, mut d)| {
            d.drawing_id = idx + 1;
            d
        })
        .collect()
}
