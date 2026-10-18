//! Testes de integração da extração de imagens embutidas e gráficos vetoriais.
//!
//! As fixtures são PDFs mínimos gerados por `tests/fixtures/make_fixtures.py`
//! (texto, imagem JPEG embutida, vetores e uma página com `/Rotate 90`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use rasgapdf::types::{ParserOptions, SpatialDrawingItem, SpatialPage};
use rasgapdf::{extract_images, parse_file, parse_raw_file, write_images_to_dir, SpatialDocument};

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn structured(name: &str, extract_images: bool, extract_drawings: bool) -> SpatialDocument {
    let options = ParserOptions {
        extract_images,
        extract_drawings,
        ..ParserOptions::default()
    };

    parse_file(&fixture(name), &options).expect("parse_file falhou")
}

/// Regressão: o `stext` do MuPDF emite blocos `{"type":"image"}` **sem** o campo `lines`.
/// Enquanto o parser usava o `StextPage` do crate `mupdf`, qualquer página com imagem
/// fazia a desserialização falhar e a página inteira voltava vazia.
#[test]
fn page_with_image_keeps_its_text() {
    let doc = structured("mixed.pdf", true, false);
    let page = &doc.pages[0];

    let texts: Vec<&str> = page
        .text_blocks
        .iter()
        .flat_map(|block| block.lines.iter())
        .map(|line| line.text.as_str())
        .collect();

    assert!(
        texts.iter().any(|t| t.contains("RasgaPDF")),
        "o texto da página com imagem foi perdido: {texts:?}"
    );
    assert!(page.raw_span_count >= 3, "raw_span_count={}", page.raw_span_count);
}

#[test]
fn raw_mode_keeps_spans_on_page_with_image() {
    let doc = parse_raw_file(&fixture("mixed.pdf")).expect("parse_raw_file falhou");

    assert!(doc.total_spans >= 3, "total_spans={}", doc.total_spans);
    assert!(doc.spans.iter().any(|span| span.text.contains("RasgaPDF")));
}

/// Compatibilidade da forma do JSON: sem opt-in, os campos novos ficam vazios.
#[test]
fn graphics_are_off_by_default() {
    let doc = structured("mixed.pdf", false, false);
    let page = &doc.pages[0];

    assert!(page.images.is_empty(), "images={:?}", page.images);
    assert!(page.drawings.is_empty(), "drawings={:?}", page.drawings);
}

#[test]
fn embedded_image_reports_metadata_and_placement() {
    let doc = structured("mixed.pdf", true, false);
    let page = &doc.pages[0];

    assert_eq!(page.images.len(), 1, "images={:?}", page.images);
    let image = &page.images[0];

    assert_eq!(image.name, "Im0");
    assert_eq!((image.pixel_width, image.pixel_height), (400, 300));
    assert_eq!(image.filter.as_deref(), Some("DCTDecode"));
    assert_eq!(image.mime_type.as_deref(), Some("image/jpeg"));
    assert!(image.xref > 0, "xref={}", image.xref);
    assert!(image.byte_length.is_some_and(|len| len > 0));
    assert!(image.sha256.as_ref().is_some_and(|hash| hash.len() == 64));

    // A imagem é pintada com a matriz `200 0 0 150 330 200 cm` (y do PDF para baixo de tela).
    let bbox = image.bbox.clone().expect("bbox de posicionamento ausente");
    assert!((bbox.x0 - 330.0).abs() <= 2.0, "{bbox:?}");
    assert!((bbox.x1 - 530.0).abs() <= 2.0, "{bbox:?}");
    assert!((bbox.y0 - 442.0).abs() <= 4.0, "{bbox:?}");
    assert!((bbox.y1 - 592.0).abs() <= 4.0, "{bbox:?}");
}

#[test]
fn drawings_map_lines_curves_and_rectangles() {
    let doc = structured("mixed.pdf", false, true);
    let page = &doc.pages[0];

    assert!(page.drawings.len() >= 3, "drawings={:?}", page.drawings);

    let kinds: BTreeSet<&str> = page
        .drawings
        .iter()
        .flat_map(|drawing| drawing.items.iter())
        .map(|item| match item {
            SpatialDrawingItem::Line { .. } => "line",
            SpatialDrawingItem::Curve { .. } => "curve",
            SpatialDrawingItem::Rect { .. } => "rect",
            SpatialDrawingItem::Quad { .. } => "quad",
        })
        .collect();

    assert!(kinds.contains("line"), "kinds={kinds:?}");
    assert!(kinds.contains("curve"), "kinds={kinds:?}");
    assert!(kinds.contains("rect"), "kinds={kinds:?}");

    // Traço vermelho `1 0 0 RG` com espessura `4 w`.
    let red_line = page.drawings.iter().any(|drawing| {
        drawing.drawing_type == "stroke"
            && drawing
                .stroke_color
                .is_some_and(|color| (color[0] - 1.0).abs() < 0.01)
            && drawing.width.is_some_and(|width| (width - 4.0).abs() < 0.01)
    });
    assert!(red_line, "traço vermelho não encontrado: {:?}", page.drawings);

    // Preenchimento azul `0 0 1 rg`.
    let blue_fill = page.drawings.iter().any(|drawing| {
        drawing.drawing_type == "fill"
            && drawing
                .fill_color
                .is_some_and(|color| color[2] > 0.9 && color[0] < 0.1)
    });
    assert!(blue_fill, "preenchimento azul não encontrado");
}

#[test]
fn drawings_keep_ids_sequential_after_filtering() {
    let options = ParserOptions {
        extract_drawings: true,
        filter_background_fills: true,
        ..ParserOptions::default()
    };
    let doc = parse_file(&fixture("mixed.pdf"), &options).expect("parse_file falhou");

    for (index, drawing) in doc.pages[0].drawings.iter().enumerate() {
        assert_eq!(drawing.drawing_id, index + 1, "ids devem ser sequenciais");
        assert_eq!(drawing.item_count, drawing.items.len());
    }
}

#[test]
fn rotated_page_reports_real_rotation() {
    let doc = structured("rotated.pdf", true, false);
    let page = &doc.pages[0];

    assert_eq!(page.rotation, 90);
    assert_eq!(page.images.len(), 1);
}

#[test]
fn extract_images_returns_original_jpeg_stream() {
    let mut found = extract_images(&fixture("mixed.pdf")).expect("extract_images falhou");

    assert_eq!(found.len(), 1);
    let record = &found[0];

    assert_eq!(record.page, 1);
    assert_eq!(&record.bytes[..3], [0xFF, 0xD8, 0xFF], "assinatura JPEG");
    assert_eq!(record.image.byte_length, Some(record.bytes.len()));
    assert_eq!(record.image.filter.as_deref(), Some("DCTDecode"));

    let expected_hash = record.image.sha256.clone().expect("sha256 ausente");
    let dir = std::env::temp_dir().join("rasgapdf_test_sidecar");
    let _ = std::fs::remove_dir_all(&dir);

    let written = write_images_to_dir(&mut found, &dir).expect("write_images_to_dir falhou");
    assert_eq!(written, 1);

    let file_name = found[0].image.file.clone().expect("nome do arquivo ausente");
    assert_eq!(file_name, "page_001_img_001_xref7.jpg");

    let bytes = std::fs::read(dir.join(&file_name)).expect("arquivo sidecar não existe");
    assert_eq!(bytes, found[0].bytes);

    let written_hash = rasgapdf::image::sha256_hex(&bytes);
    assert_eq!(written_hash, expected_hash);

    let manifest_path =
        rasgapdf::write_manifest(&found, written, "mixed.pdf", &dir).expect("manifesto falhou");
    assert!(manifest_path.exists(), "manifest.json não foi gravado");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["file_name"], "mixed.pdf");
    assert_eq!(manifest["total_images"], 1);
    assert_eq!(manifest["written"], 1);
    assert_eq!(manifest["images"][0]["page"], 1);
    assert_eq!(manifest["images"][0]["image"]["file"], "page_001_img_001_xref7.jpg");

    let _ = std::fs::remove_dir_all(&dir);
}

/// O JSON antigo (sem `images`/`drawings`) continua legível.
#[test]
fn legacy_page_json_still_deserializes() {
    let legacy = r#"{
        "page_number": 1,
        "width": 595.0,
        "height": 842.0,
        "rotation": 0,
        "text_blocks": [],
        "tables": [],
        "raw_span_count": 0
    }"#;

    let page: SpatialPage = serde_json::from_str(legacy).expect("JSON legado deve deserializar");
    assert!(page.images.is_empty());
    assert!(page.drawings.is_empty());
}

/// `ParserOptions` aceita JSON parcial (a FFI dependia disso para não cair no default silencioso).
#[test]
fn partial_parser_options_json_uses_defaults() {
    let options: ParserOptions =
        serde_json::from_str(r#"{"mode":"raw"}"#).expect("opções parciais devem deserializar");

    assert_eq!(options.mode, "raw");
    assert!(options.detect_tables);
    assert_eq!(options.min_table_columns, 3);
    assert!(!options.extract_images);
    assert!(!options.extract_drawings);
    assert!(options.filter_background_fills);
}

#[test]
fn text_only_page_is_unaffected() {
    let doc = structured("text_only.pdf", true, true);
    let page = &doc.pages[0];

    assert!(page.text_blocks.len() >= 1);
    assert!(page.images.is_empty(), "não há imagem nesta fixture");
    assert!(page.drawings.is_empty(), "não há vetor nesta fixture");
}
