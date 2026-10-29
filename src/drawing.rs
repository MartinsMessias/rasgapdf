//! Mapeamento e filtragem de caminhos vetoriais.

use crate::types::SpatialDrawing;

/// Indica se o desenho é essencialmente um fundo que cobre a página inteira.
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

/// Normaliza e renumera a lista de desenhos de uma página após filtragem de fundos.
pub fn map_drawings(
    drawings: &[SpatialDrawing],
    page_width: f32,
    page_height: f32,
    filter_backgrounds: bool,
) -> Vec<SpatialDrawing> {
    drawings
        .iter()
        .filter(|d| !(filter_backgrounds && is_background_fill(d, page_width, page_height)))
        .enumerate()
        .map(|(idx, d)| {
            let mut cloned = d.clone();
            cloned.drawing_id = idx + 1;
            cloned
        })
        .collect()
}
