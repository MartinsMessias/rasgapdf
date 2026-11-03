use crate::engine::PurePdfDoc;
use crate::types::*;
use std::error::Error;
use std::path::Path;

/// Modo 1: Parse Bruto Espacial (Raw X/Y)
/// Retorna exclusivamente a lista plana e direta de coordenadas físicas de cada elemento
pub fn parse_raw_file(pdf_path: &str) -> Result<RawSpatialDocument, Box<dyn Error + Send + Sync>> {
    let clean_path = pdf_path.replace('\0', "");
    let path = Path::new(&clean_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", clean_path).into());
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string().replace('\0', "_"))
        .unwrap_or_else(|| "document.pdf".to_string());

    let bytes = std::fs::read(&clean_path)?;
    if bytes.is_empty() {
        return Err("PDF file is empty".into());
    }
    let mut doc =
        PurePdfDoc::from_bytes(&bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc
        .parse_raw(&file_name)
        .map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;
    Ok(raw)
}

/// Modo 1 (Em Memória): Parse Bruto Espacial a partir de slice de bytes (`&[u8]`)
pub fn parse_raw_bytes(
    pdf_bytes: &[u8],
) -> Result<RawSpatialDocument, Box<dyn Error + Send + Sync>> {
    if pdf_bytes.is_empty() {
        return Err("PDF bytes slice is empty".into());
    }
    let mut doc =
        PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc
        .parse_raw("memory.pdf")
        .map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;
    Ok(raw)
}

/// Modo 2: Parse Estruturado (Hierárquico com Blocos e Tabelas)
pub fn parse_file(
    pdf_path: &str,
    options: &ParserOptions,
) -> Result<SpatialDocument, Box<dyn Error + Send + Sync>> {
    let clean_path = pdf_path.replace('\0', "");
    let path = Path::new(&clean_path);
    if !path.exists() {
        return Err(format!("PDF file not found: {}", clean_path).into());
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string().replace('\0', "_"))
        .unwrap_or_else(|| "document.pdf".to_string());

    let bytes = std::fs::read(&clean_path)?;
    if bytes.is_empty() {
        return Err("PDF file is empty".into());
    }
    let mut doc =
        PurePdfDoc::from_bytes(&bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let structured = doc
        .parse_structured(&file_name, options)
        .map_err(|e| format!("Pure PDF parse_structured error: {e}"))?;
    Ok(structured)
}

/// Modo 2 (Em Memória): Parse Estruturado a partir de slice de bytes (`&[u8]`)
pub fn parse_bytes(
    pdf_bytes: &[u8],
    options: &ParserOptions,
) -> Result<SpatialDocument, Box<dyn Error + Send + Sync>> {
    if pdf_bytes.is_empty() {
        return Err("PDF bytes slice is empty".into());
    }
    let mut doc =
        PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let structured = doc
        .parse_structured("memory.pdf", options)
        .map_err(|e| format!("Pure PDF parse_structured error: {e}"))?;
    Ok(structured)
}

/// Extração paralela de texto puro em memória a partir de slice de bytes (`&[u8]`)
pub fn extract_text_bytes(pdf_bytes: &[u8]) -> Result<String, Box<dyn Error + Send + Sync>> {
    if pdf_bytes.is_empty() {
        return Ok(String::new());
    }
    let mut doc =
        PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let raw = doc
        .parse_raw("memory.pdf")
        .map_err(|e| format!("Pure PDF parse_raw error: {e}"))?;

    let cap = raw.total_spans.saturating_mul(16).min(10 * 1024 * 1024);
    let mut out = String::with_capacity(cap);
    let mut last_page = 0;
    let mut last_y0 = -1.0;
    let mut last_x1 = -1.0_f32;

    for span in &raw.spans {
        let span_y0 = if span.y0.is_finite() {
            span.y0
        } else {
            last_y0
        };
        if span.page != last_page {
            if last_page != 0 {
                out.push('\n');
                out.push('\n');
            }
            last_page = span.page;
            last_y0 = span_y0;
        } else if (span_y0 - last_y0).abs() > 4.0 {
            out.push('\n');
            last_y0 = span_y0;
        } else if !out.ends_with(' ') && !span.text.starts_with(' ') {
            let gap = if span.x0.is_finite() && last_x1 >= 0.0 {
                span.x0 - last_x1
            } else {
                f32::MAX
            };
            let char_spacing_limit = (span.font_size * 0.09).clamp(0.4, 1.35);
            if gap > char_spacing_limit {
                out.push(' ');
            }
        }

        out.push_str(&span.text);
        if span.x1.is_finite() {
            last_x1 = span.x1;
        }
    }

    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bytes_handling() {
        assert!(parse_raw_bytes(&[]).is_err());
        assert!(parse_bytes(&[], &ParserOptions::default()).is_err());
        let res = extract_text_bytes(&[]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "");
    }

    #[test]
    fn test_nonexistent_and_nul_path() {
        assert!(parse_raw_file("nonexistent\0_file.pdf").is_err());
        assert!(parse_file("nonexistent\0_file.pdf", &ParserOptions::default()).is_err());
    }

    #[test]
    fn test_extract_text_bytes_glyph_by_glyph_stream() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R >>\nendobj\n",
        );
        let o4 = pdf.len();
        let content = b"BT\n/F1 12 Tf\n1 0 0 1 72 700 Tm\n(T) Tj\n7.3 0 Td (A) Tj\n8.0 0 Td (M) Tj\n13.5 0 Td (L) Tj\n6.7 0 Td (i) Tj\n3.3 0 Td (n) Tj\n6.7 0 Td (h) Tj\n6.7 0 Td (a) Tj\n6.7 0 Td (s) Tj\nET\n";
        pdf.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
        );
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 5\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
                o1, o2, o3, o4, xref_pos
            )
            .as_bytes(),
        );

        let text = extract_text_bytes(&pdf).unwrap();
        assert_eq!(text.trim(), "TAM Linhas");
    }
}
