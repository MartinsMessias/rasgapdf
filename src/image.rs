//! Extração de imagens embutidas (XObjects `/Subtype /Image`).
//!
//! O RasgaPDF expõe **metadados** das imagens no JSON espacial e, opcionalmente,
//! grava o **stream codificado original** em disco (sidecar) via [`extract_images`].
//! Nenhum pixel é decodificado: o stream é copiado como está (ex.: `DCTDecode` → JPEG).

use std::error::Error;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::engine::PurePdfDoc;
use crate::types::SpatialImage;

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

/// Escreve os streams extraídos em `dir` e preenche `image.file` com o nome gerado.
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
pub fn extract_images(
    pdf_path: &str,
) -> Result<Vec<ExtractedImageData>, Box<dyn Error + Send + Sync>> {
    let bytes = std::fs::read(pdf_path)?;
    extract_images_from_bytes(&bytes)
}

/// Extrai todas as imagens embutidas a partir de um buffer de bytes em memória (`&[u8]`).
pub fn extract_images_from_bytes(
    pdf_bytes: &[u8],
) -> Result<Vec<ExtractedImageData>, Box<dyn Error + Send + Sync>> {
    let mut doc = PurePdfDoc::from_bytes(pdf_bytes).map_err(|e| format!("Pure PDF parse error: {e}"))?;
    let images = doc.extract_all_images().map_err(|e| format!("Extract images error: {e}"))?;
    Ok(images)
}
