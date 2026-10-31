use rasgapdf::*;

#[test]
fn test_bounding_box_rotation_transformations() {
    // Unrotated page: 600 wide, 800 high
    let pw = 600.0;
    let ph = 800.0;
    let original = BoundingBox::new(10.0, 20.0, 100.0, 60.0);
    assert_eq!(original.width, 90.0);
    assert_eq!(original.height, 40.0);

    // 0 degrees rotation: identical
    let rot0 = original.rotate(0, pw, ph);
    assert_eq!((rot0.x0, rot0.y0, rot0.x1, rot0.y1), (10.0, 20.0, 100.0, 60.0));

    // 90 degrees clockwise rotation:
    // Former bottom-left y becomes new left x; width & height swap
    let rot90 = original.rotate(90, pw, ph);
    assert_eq!((rot90.x0, rot90.y0, rot90.x1, rot90.y1), (740.0, 10.0, 780.0, 100.0));
    assert_eq!(rot90.width, 40.0);
    assert_eq!(rot90.height, 90.0);

    // 180 degrees rotation:
    let rot180 = original.rotate(180, pw, ph);
    assert_eq!((rot180.x0, rot180.y0, rot180.x1, rot180.y1), (500.0, 740.0, 590.0, 780.0));
    assert_eq!(rot180.width, 90.0);
    assert_eq!(rot180.height, 40.0);

    // 270 degrees rotation (or -90):
    let rot270 = original.rotate(270, pw, ph);
    assert_eq!((rot270.x0, rot270.y0, rot270.x1, rot270.y1), (20.0, 500.0, 60.0, 590.0));
    assert_eq!(rot270.width, 40.0);
    assert_eq!(rot270.height, 90.0);
}

#[test]
fn test_spatial_chunks_preserve_all_constituent_bboxes() {
    let block1 = SpatialBlock {
        block_id: 1,
        block_type: "text".to_string(),
        bbox: BoundingBox::new(50.0, 100.0, 550.0, 180.0),
        lines: vec![
            SpatialLine {
                text: "Introduction to Spatial Retrieval".to_string(),
                bbox: BoundingBox::new(50.0, 100.0, 350.0, 130.0),
                spans: vec![],
            },
            SpatialLine {
                text: "Flat text loses visual and coordinate bounding boxes.".to_string(),
                bbox: BoundingBox::new(50.0, 140.0, 550.0, 180.0),
                spans: vec![],
            },
        ],
    };

    let block2 = SpatialBlock {
        block_id: 2,
        block_type: "text".to_string(),
        bbox: BoundingBox::new(50.0, 200.0, 550.0, 260.0),
        lines: vec![
            SpatialLine {
                text: "By keeping the bounding box array intact,".to_string(),
                bbox: BoundingBox::new(50.0, 200.0, 400.0, 225.0),
                spans: vec![],
            },
            SpatialLine {
                text: "downstream viewers can highlight multiple rectangles.".to_string(),
                bbox: BoundingBox::new(50.0, 230.0, 550.0, 260.0),
                spans: vec![],
            },
        ],
    };

    let page = SpatialPage {
        page_number: 1,
        width: 612.0,
        height: 792.0,
        rotation: 0,
        text_blocks: vec![block1, block2],
        tables: vec![],
        raw_span_count: 4,
        images: vec![],
        drawings: vec![],
        media_box: Some([0.0, 0.0, 612.0, 792.0]),
        crop_box: Some([0.0, 0.0, 612.0, 792.0]),
    };

    // When max_chars is large enough, both blocks are grouped into a single chunk
    let chunks = page.to_spatial_chunks(1500, 0);
    assert_eq!(chunks.len(), 1, "Should aggregate contiguous blocks into 1 chunk");
    assert_eq!(chunks[0].block_count, 2);
    assert_eq!(chunks[0].bboxes.len(), 2, "Must preserve individual bounding boxes for each block");
    assert_eq!(chunks[0].bboxes[0].y0, 100.0);
    assert_eq!(chunks[0].bboxes[1].y0, 200.0);
    assert_eq!(chunks[0].overall_bbox.y0, 100.0);
    assert_eq!(chunks[0].overall_bbox.y1, 260.0);
    assert!(chunks[0].text.contains("Introduction to Spatial Retrieval"));
    assert!(chunks[0].text.contains("downstream viewers can highlight multiple rectangles"));
}

#[test]
fn test_crop_box_origin_normalization_and_precedence() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    // MediaBox [0 0 612 792] with CropBox [36 54 576 756] (margins cropped)
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /CropBox [ 36 54 576 756 ] /Contents 4 0 R >>\nendobj\n");
    let o4 = pdf.len();
    let content = b"BT\n/F1 12 Tf\n36 740 Td\n(TopMarginCroppedText) Tj\nET\n";
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"endstream\nendobj\n");
    let xref_pos = pdf.len();
    pdf.extend_from_slice(
        format!(
            "xref\n0 5\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
            o1, o2, o3, o4, xref_pos
        )
        .as_bytes(),
    );

    let mut doc = rasgapdf::engine::PurePdfDoc::from_bytes(&pdf).expect("Failed to parse PDF");
    let page = doc.load_page(0).expect("Failed to load page");

    // Precedence: effective width and height should match CropBox (576 - 36 = 540, 756 - 54 = 702)
    assert_eq!(page.width, 540.0);
    assert_eq!(page.height, 702.0);
    assert_eq!(page.origin_x, 36.0);
    assert_eq!(page.origin_y, 54.0);
    assert_eq!(page.media_box, Some([0.0, 0.0, 612.0, 792.0]));
    assert_eq!(page.crop_box, Some([36.0, 54.0, 576.0, 756.0]));

    // Parse structured document: normalized coordinates relative to cropbox top-left
    let mut opts = ParserOptions::default();
    opts.detect_tables = false;
    let sdoc = doc.parse_structured("cropped.pdf", &opts).expect("Structured parse failed");
    assert_eq!(sdoc.pages[0].width, 540.0);
    assert_eq!(sdoc.pages[0].height, 702.0);
    assert_eq!(sdoc.pages[0].crop_box, Some([36.0, 54.0, 576.0, 756.0]));
    assert_eq!(sdoc.pages[0].media_box, Some([0.0, 0.0, 612.0, 792.0]));

    // The text was placed at (36, 740) in user space.
    // Cropbox has origin_x=36, max_y=756.
    // Normalized X = 36 - 36 = 0.
    // Normalized Y = 756 - 740 = 16 (plus font baseline).
    let span = &sdoc.pages[0].text_blocks[0].lines[0];
    assert!((span.bbox.x0 - 0.0).abs() <= 1.0, "Expected x0 near 0, got {}", span.bbox.x0);
}

#[test]
fn test_rotated_page_with_applied_rotation() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    // 612x792 page with /Rotate 90
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Rotate 90 /Contents 4 0 R >>\nendobj\n");
    let o4 = pdf.len();
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(Rotated90Text) Tj\nET\n";
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"endstream\nendobj\n");
    let xref_pos = pdf.len();
    pdf.extend_from_slice(
        format!(
            "xref\n0 5\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
            o1, o2, o3, o4, xref_pos
        )
        .as_bytes(),
    );

    let mut doc = rasgapdf::engine::PurePdfDoc::from_bytes(&pdf).expect("Failed to parse PDF");
    let mut opts = ParserOptions::default();
    opts.detect_tables = false;

    // Default mode: unrotated bboxes + rotation metadata
    let raw_parsed = doc.parse_structured("rot.pdf", &opts).unwrap();
    assert_eq!(raw_parsed.pages[0].rotation, 90);
    assert_eq!(raw_parsed.pages[0].width, 612.0);
    assert_eq!(raw_parsed.pages[0].height, 792.0);

    // With apply_rotation: true
    opts.apply_rotation = true;
    let mut doc2 = rasgapdf::engine::PurePdfDoc::from_bytes(&pdf).expect("Failed to parse PDF");
    let rotated_doc = doc2.parse_structured("rot.pdf", &opts).unwrap();
    // 90 degrees rotation swaps dimensions: width becomes 792, height becomes 612
    assert_eq!(rotated_doc.pages[0].width, 792.0);
    assert_eq!(rotated_doc.pages[0].height, 612.0);
    assert_eq!(rotated_doc.pages[0].rotation, 0); // normalized

    let rotated_block = &rotated_doc.pages[0].text_blocks[0];
    assert!(rotated_block.bbox.x0 >= 0.0 && rotated_block.bbox.x1 <= 792.0);
    assert!(rotated_block.bbox.y0 >= 0.0 && rotated_block.bbox.y1 <= 612.0);
}
