use std::error::Error;
use std::path::Path;
use crate::engine::PurePdfDoc;
use crate::types::*;

/// Modo 1: Parse Bruto Espacial (Raw X/Y)
/// Retorna exclusivamente a lista plana e direta de coordenadas físicas de cada elemento
pub fn parse_raw_file(pdf_path: &str) -> Result<RawSpatialDocument, Box<dyn Error + Send + Sync>> {
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", pdf_path).into());
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.pdf".to_string());

    let bytes = std::fs::read(pdf_path)?;
    let mut doc = PurePdfDoc::from_bytes(&bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc.parse_raw(&file_name).map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;
    Ok(raw)
}

/// Modo 1 (Em Memória): Parse Bruto Espacial a partir de slice de bytes (`&[u8]`)
pub fn parse_raw_bytes(pdf_bytes: &[u8]) -> Result<RawSpatialDocument, Box<dyn Error + Send + Sync>> {
    let mut doc = PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc.parse_raw("memory.pdf").map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;
    Ok(raw)
}

/// Modo 2: Parse Estruturado (Hierárquico com Blocos e Tabelas)
pub fn parse_file(pdf_path: &str, options: &ParserOptions) -> Result<SpatialDocument, Box<dyn Error + Send + Sync>> {
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", pdf_path).into());
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.pdf".to_string());

    let bytes = std::fs::read(pdf_path)?;
    let mut doc = PurePdfDoc::from_bytes(&bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let structured = doc.parse_structured(&file_name, options).map_err(|e| format!("Pure PDF parse_structured error: {e}"))?;
    Ok(structured)
}

/// Extração paralela de texto puro em memória a partir de slice de bytes (`&[u8]`)
pub fn extract_text_bytes(pdf_bytes: &[u8]) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut doc = PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc.parse_raw("memory.pdf").map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;

    let mut out = String::with_capacity(raw.total_spans * 16);
    let mut last_page = 0;
    let mut last_y0 = -1.0;

    for span in &raw.spans {
        if span.page != last_page {
            if last_page != 0 {
                out.push('\n');
                out.push('\n');
            }
            last_page = span.page;
            last_y0 = span.y0;
        } else if (span.y0 - last_y0).abs() > 4.0 {
            out.push('\n');
            last_y0 = span.y0;
        } else if !out.ends_with(' ') && !span.text.starts_with(' ') {
            out.push(' ');
        }

        out.push_str(&span.text);
    }

    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }

    Ok(out)
}
