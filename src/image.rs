//! Extração de imagens embutidas (XObjects `/Subtype /Image`).
//!
//! O RasgaPDF expõe **metadados** das imagens no JSON espacial e, opcionalmente,
//! grava o **stream codificado original** em disco (sidecar) via [`extract_images`].
//! Nenhum pixel é decodificado: o stream é copiado como está (ex.: `DCTDecode` → JPEG).

use std::error::Error;
use std::path::{Path, PathBuf};

use mupdf::pdf::{PageImageInfo, PdfDocument, PdfPage};
use mupdf::Document;
use sha2::{Digest, Sha256};

use crate::types::{BoundingBox, SpatialImage};

/// Uma imagem embutida extraída, já com os bytes originais do stream.
#[derive(Debug, Clone)]
pub struct ExtractedImageData {
    /// Número da página (1-indexed).
    pub page: usize,
    /// Metadados espaciais.
    pub image: SpatialImage,
    /// Stream codificado original (pode ser vazio).
    pub bytes: Vec<u8>,
}

/// Retorna o último nome de filtro declarado em `/Filter`, preservando o caso original.
///
/// Em `/Filter [/ASCII85Decode /DCTDecode]` o último filtro é o codec real da imagem.
pub fn last_filter_name(filter: Option<&str>) -> Option<String> {
    let raw = filter?;
    raw.rsplit(|c: char| !c.is_ascii_alphanumeric())
        .find(|token| !token.is_empty())
        .map(str::to_string)
}

/// Versão em caixa alta do filtro, para comparação.
pub fn normalized_filter(filter: Option<&str>) -> Option<String> {
    last_filter_name(filter).map(|f| f.to_ascii_uppercase())
}

/// MIME type do stream quando ele já é um arquivo de imagem autocontido.
pub fn mime_from_filter(filter: Option<&str>) -> Option<&'static str> {
    Some(match normalized_filter(filter)?.as_str() {
        "DCTDECODE" | "DCT" => "image/jpeg",
        "JPXDECODE" | "JPX" => "image/jp2",
        "JBIG2DECODE" => "image/jbig2",
        "CCITTFAXDECODE" | "CCF" => "image/tiff",
        _ => return None,
    })
}

/// Extensão de arquivo sugerida para o stream codificado.
pub fn extension_for_filter(filter: Option<&str>) -> &'static str {
    match normalized_filter(filter).as_deref() {
        Some("DCTDECODE") | Some("DCT") => "jpg",
        Some("JPXDECODE") | Some("JPX") => "jp2",
        Some("JBIG2DECODE") => "jb2",
        Some("CCITTFAXDECODE") | Some("CCF") => "tif",
        Some("FLATEDECODE") | Some("FL") => "flate.raw",
        Some("LZWDECODE") | Some("LZW") => "lzw.raw",
        _ => "bin",
    }
}

/// SHA-256 em hexadecimal minúsculo.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Monta os metadados espaciais de uma imagem.
pub fn build_spatial_image(
    image_id: usize,
    info: &PageImageInfo,
    bbox: Option<BoundingBox>,
    bytes: Option<&[u8]>,
) -> SpatialImage {
    SpatialImage {
        image_id,
        name: info.name.clone(),
        xref: info.xref,
        bbox,
        pixel_width: info.width,
        pixel_height: info.height,
        bits_per_component: info.bits_per_component,
        color_space: info.color_space.clone(),
        filter: last_filter_name(info.filter.as_deref()),
        mime_type: mime_from_filter(info.filter.as_deref()).map(str::to_string),
        byte_length: bytes.map(|b| b.len()),
        sha256: bytes.map(sha256_hex),
        file: None,
    }
}

/// Lê o stream codificado de uma imagem pelo seu xref.
pub fn extract_image_bytes(doc: &PdfDocument, xref: i32) -> Option<Vec<u8>> {
    if xref <= 0 {
        return None;
    }
    doc.extract_image(xref).ok().map(|img| img.encoded)
}

/// Escreve os streams extraídos em `dir` e preenche `image.file` com o nome gerado.
///
/// Retorna a quantidade de arquivos gravados. Imagens sem bytes são ignoradas.
pub fn write_images_to_dir(
    images: &mut [ExtractedImageData],
    dir: &Path,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    std::fs::create_dir_all(dir)?;

    let mut written = 0;
    for record in images.iter_mut() {
        if record.bytes.is_empty() {
            continue;
        }

        let extension = extension_for_filter(record.image.filter.as_deref());
        let file_name = format!(
            "page_{:03}_img_{:03}_xref{}.{}",
            record.page, record.image.image_id, record.image.xref, extension
        );
        std::fs::write(dir.join(&file_name), &record.bytes)?;

        record.image.file = Some(file_name);
        written += 1;
    }

    Ok(written)
}

/// Monta o manifesto JSON das imagens extraídas.
pub fn build_manifest(
    images: &[ExtractedImageData],
    written: usize,
    file_name: &str,
) -> serde_json::Value {
    serde_json::json!({
        "file_name": file_name,
        "total_images": images.len(),
        "written": written,
        "images": images
            .iter()
            .map(|record| serde_json::json!({
                "page": record.page,
                "image": record.image,
            }))
            .collect::<Vec<_>>(),
    })
}

/// Grava `<dir>/manifest.json` e devolve o caminho escrito.
pub fn write_manifest(
    images: &[ExtractedImageData],
    written: usize,
    file_name: &str,
    dir: &Path,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    std::fs::create_dir_all(dir)?;

    let path = dir.join("manifest.json");
    let file = std::fs::File::create(&path)?;
    serde_json::to_writer_pretty(file, &build_manifest(images, written, file_name))?;

    Ok(path)
}

/// Extrai todas as imagens embutidas do documento, com os bytes originais.
///
/// Percorre as páginas sequencialmente (o documento MuPDF não é `Send`, e a extração
/// de bytes é dominada por I/O de stream, não por CPU).
pub fn extract_images(
    pdf_path: &str,
) -> Result<Vec<ExtractedImageData>, Box<dyn Error + Send + Sync>> {
    let doc = Document::open(pdf_path)?;
    let page_count = doc.page_count()? as usize;
    let pdf_doc = PdfDocument::try_from(doc)
        .map_err(|e| format!("document is not a PDF: {e}"))?;

    let mut out: Vec<ExtractedImageData> = Vec::new();

    for page_idx in 0..page_count {
        let page = pdf_doc.load_page(page_idx as i32)?;
        let pdf_page = PdfPage::try_from(page)
            .map_err(|e| format!("failed to convert page {}: {e}", page_idx + 1))?;

        let infos = pdf_page.images().unwrap_or_default();
        for (idx, info) in infos.iter().enumerate() {
            let bytes = extract_image_bytes(&pdf_doc, info.xref).unwrap_or_default();
            let has_bytes = !bytes.is_empty();
            let image = build_spatial_image(
                idx + 1,
                info,
                None,
                if has_bytes { Some(bytes.as_slice()) } else { None },
            );
            out.push(ExtractedImageData {
                page: page_idx + 1,
                image,
                bytes,
            });
        }
    }

    Ok(out)
}


/// Extrai todas as imagens embutidas a partir de um buffer de bytes em memória (`&[u8]`).
pub fn extract_images_from_bytes(
    pdf_bytes: &[u8],
) -> Result<Vec<ExtractedImageData>, Box<dyn Error + Send + Sync>> {
    let doc = Document::from_bytes(pdf_bytes, "application/pdf")?;
    let page_count = doc.page_count()? as usize;
    let pdf_doc = PdfDocument::try_from(doc)
        .map_err(|e| format!("document is not a PDF: {e}"))?;

    let mut out: Vec<ExtractedImageData> = Vec::new();

    for page_idx in 0..page_count {
        let page = pdf_doc.load_page(page_idx as i32)?;
        let pdf_page = PdfPage::try_from(page)
            .map_err(|e| format!("failed to convert page {}: {e}", page_idx + 1))?;

        let infos = pdf_page.images().unwrap_or_default();
        for (idx, info) in infos.iter().enumerate() {
            let bytes = extract_image_bytes(&pdf_doc, info.xref).unwrap_or_default();
            let has_bytes = !bytes.is_empty();
            let image = build_spatial_image(
                idx + 1,
                info,
                None,
                if has_bytes { Some(bytes.as_slice()) } else { None },
            );
            out.push(ExtractedImageData {
                page: page_idx + 1,
                image,
                bytes,
            });
        }
    }

    Ok(out)
}
