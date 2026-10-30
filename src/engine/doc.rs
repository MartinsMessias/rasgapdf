use super::content::ContentInterpreter;
use super::flate::decompress_stream;
use super::font::PdfFont;
use super::object::{ObjectId, PdfObject};
use super::xref::XRefTable;
use crate::image::{self, ExtractedImageData};
use crate::table::{RawSpan, TableDetector};
use crate::types::*;
use std::collections::{BTreeMap, HashMap, HashSet};

#[inline]
fn get_elapsed_ms(#[allow(unused)] start: Option<std::time::Instant>) -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        start.map(|s| s.elapsed().as_secs_f64() * 1000.0).unwrap_or(0.0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
}

#[inline]
fn now_instant() -> Option<std::time::Instant> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Some(std::time::Instant::now())
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

pub const MAX_OBJECT_RESOLVE_DEPTH: usize = 64;
pub const MAX_PAGE_TREE_DEPTH: usize = 32;

/// High-level pure-Rust PDF document parser
pub struct PurePdfDoc<'a> {
    data: &'a [u8],
    xref: XRefTable,
    obj_cache: HashMap<u32, BTreeMap<u32, PdfObject>>,
    pages: Vec<ObjectId>,
}

pub struct PageInfo {
    pub width: f32,
    pub height: f32,
    pub rotation: i32,
    pub fonts: HashMap<String, PdfFont>,
    pub contents: Vec<u8>,
    pub raw_page_obj: PdfObject,
}

impl<'a> PurePdfDoc<'a> {
    /// Opens and parses a PDF from a byte slice
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, String> {
        let xref = XRefTable::parse(data)?;
        let mut doc = Self {
            data,
            xref,
            obj_cache: HashMap::new(),
            pages: Vec::new(),
        };

        doc.collect_pages()?;
        Ok(doc)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Resolves an indirect object, dereferencing references recursively with bounded recursion and cycle detection
    pub fn resolve(&mut self, obj: &PdfObject) -> Result<PdfObject, String> {
        self.resolve_depth(obj, 0)
    }

    /// Resolves an indirect object with explicit depth counter and cycle detection
    pub fn resolve_depth(&mut self, obj: &PdfObject, depth: usize) -> Result<PdfObject, String> {
        let mut visited = HashSet::new();
        self.resolve_internal(obj, depth, &mut visited)
    }

    fn resolve_internal(
        &mut self,
        obj: &PdfObject,
        depth: usize,
        visited: &mut HashSet<ObjectId>,
    ) -> Result<PdfObject, String> {
        if depth >= MAX_OBJECT_RESOLVE_DEPTH {
            return Err("Maximum object resolution depth (64) exceeded (possible cycle)".to_string());
        }
        match obj {
            PdfObject::Reference(id) => {
                if !visited.insert(*id) {
                    return Err(format!("Cyclic object reference detected at {}", id));
                }
                let resolved = self.xref.get_object(self.data, *id, &mut self.obj_cache)?;
                self.resolve_internal(&resolved, depth + 1, visited)
            }
            _ => Ok(obj.clone()),
        }
    }

    fn collect_pages(&mut self) -> Result<(), String> {
        let root_ref = self
            .xref
            .trailer
            .get("Root")
            .and_then(|r| r.as_reference())
            .ok_or("Missing /Root in PDF trailer")?;

        let catalog = self.xref.get_object(self.data, root_ref, &mut self.obj_cache)?;
        let pages_ref = catalog
            .dict_get("Pages")
            .and_then(|p| p.as_reference())
            .ok_or("Missing /Pages in PDF Catalog")?;

        let mut visited = HashSet::new();
        self.traverse_pages_node(pages_ref, 0, &mut visited)?;
        Ok(())
    }

    fn traverse_pages_node(
        &mut self,
        node_ref: ObjectId,
        depth: usize,
        visited: &mut HashSet<ObjectId>,
    ) -> Result<(), String> {
        if depth >= MAX_PAGE_TREE_DEPTH {
            return Err("Maximum page tree depth (32) exceeded (possible cycle)".to_string());
        }
        if !visited.insert(node_ref) {
            return Err(format!("Cyclic page tree node detected at {}", node_ref));
        }

        let node = self.xref.get_object(self.data, node_ref, &mut self.obj_cache)?;
        let node_type = node.dict_get("Type").and_then(|t| t.as_name()).unwrap_or("");

        if node_type == "Pages" {
            if let Some(kids_arr) = node.dict_get("Kids").and_then(|k| k.as_array()) {
                let kid_refs: Vec<ObjectId> = kids_arr.iter().filter_map(|k| k.as_reference()).collect();
                for kid in kid_refs {
                    self.traverse_pages_node(kid, depth + 1, visited)?;
                }
            }
        } else if node_type == "Page" || node.dict_get("MediaBox").is_some() || node.dict_get("Contents").is_some() {
            self.pages.push(node_ref);
        }

        Ok(())
    }

    /// Loads metadata, resources, fonts, and decompressed content streams for a given page index (0-based)
    pub fn load_page(&mut self, page_idx: usize) -> Result<PageInfo, String> {
        if page_idx >= self.pages.len() {
            return Err(format!("Page index {} out of range", page_idx));
        }

        let page_ref = self.pages[page_idx];
        let page_obj = self.xref.get_object(self.data, page_ref, &mut self.obj_cache)?;

        // Page dimensions (/MediaBox or /CropBox)
        let (width, height) = self.get_page_dimensions(&page_obj)?;

        // Page rotation (/Rotate)
        let rotation = page_obj
            .dict_get("Rotate")
            .and_then(|r| r.as_i64())
            .unwrap_or(0) as i32;

        // Resources (/Font)
        let fonts = self.load_page_fonts(&page_obj)?;

        // Contents stream
        let contents = self.load_page_contents(&page_obj)?;

        Ok(PageInfo {
            width,
            height,
            rotation,
            fonts,
            contents,
            raw_page_obj: page_obj,
        })
    }

    fn get_page_dimensions(&mut self, page_obj: &PdfObject) -> Result<(f32, f32), String> {
        let box_obj = page_obj
            .dict_get("MediaBox")
            .or_else(|| page_obj.dict_get("CropBox"));

        if let Some(b) = box_obj {
            let resolved = self.resolve(b)?;
            if let Some(arr) = resolved.as_array() {
                if arr.len() >= 4 {
                    let x0 = arr[0].as_f64().unwrap_or(0.0) as f32;
                    let y0 = arr[1].as_f64().unwrap_or(0.0) as f32;
                    let x1 = arr[2].as_f64().unwrap_or(595.28) as f32;
                    let y1 = arr[3].as_f64().unwrap_or(841.89) as f32;
                    return Ok(((x1 - x0).abs(), (y1 - y0).abs()));
                }
            }
        }

        // Default to standard A4 dimensions
        Ok((595.28, 841.89))
    }

    fn load_page_fonts(&mut self, page_obj: &PdfObject) -> Result<HashMap<String, PdfFont>, String> {
        let mut fonts = HashMap::new();

        let res_obj = match page_obj.dict_get("Resources") {
            Some(r) => self.resolve(r)?,
            None => return Ok(fonts),
        };

        self.extract_fonts_from_resources(&res_obj, &mut fonts)?;

        // Also inspect Form XObjects in /XObject to extract any internal fonts
        if let Some(xobj_entry) = res_obj.dict_get("XObject") {
            if let Ok(xobjs) = self.resolve(xobj_entry) {
                if let Some(xdict) = xobjs.as_dict() {
                    for entry in xdict.values() {
                        if let Ok(resolved) = self.resolve(entry) {
                            if let Some(stream) = resolved.as_stream() {
                                if let Some(form_res) = stream.dict.get("Resources") {
                                    if let Ok(r_res) = self.resolve(form_res) {
                                        let _ = self.extract_fonts_from_resources(&r_res, &mut fonts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(fonts)
    }

    fn extract_fonts_from_resources(&mut self, res_obj: &PdfObject, fonts: &mut HashMap<String, PdfFont>) -> Result<(), String> {
        let font_dict_obj = match res_obj.dict_get("Font") {
            Some(f) => self.resolve(f)?,
            None => return Ok(()),
        };

        if let Some(font_map) = font_dict_obj.as_dict() {
            for (font_alias, font_entry) in font_map {
                if fonts.contains_key(font_alias) {
                    continue;
                }
                if let Ok(resolved_font) = self.resolve(font_entry) {
                    let mut pdf_font = PdfFont::new(font_alias.clone());
                    if let Some(bf) = resolved_font.dict_get("BaseFont").and_then(|b| b.as_name()) {
                        pdf_font.set_base_font(bf);
                    }
                    if let Some(st) = resolved_font.dict_get("Subtype").and_then(|s| s.as_name()) {
                        pdf_font.subtype = st.to_string();
                    }

                    if let Some(fc) = resolved_font.dict_get("FirstChar").and_then(|c| c.as_i64()) {
                        pdf_font.first_char = fc as u32;
                    }
                    if let Some(lc) = resolved_font.dict_get("LastChar").and_then(|c| c.as_i64()) {
                        pdf_font.last_char = lc as u32;
                    }

                    // /Widths
                    if let Some(widths_obj) = resolved_font.dict_get("Widths") {
                        if let Ok(w_resolved) = self.resolve(widths_obj) {
                            if let Some(w_arr) = w_resolved.as_array() {
                                pdf_font.widths = w_arr
                                    .iter()
                                    .map(|v| v.as_f64().unwrap_or(1000.0) as f32)
                                    .collect();
                            }
                        }
                    }

                    // /DescendantFonts (for Type0 / CID fonts)
                    if let Some(desc_obj) = resolved_font.dict_get("DescendantFonts") {
                        if let Ok(desc_resolved) = self.resolve(desc_obj) {
                            if let Some(desc_arr) = desc_resolved.as_array() {
                                if let Some(first_desc) = desc_arr.first() {
                                    if let Ok(df_resolved) = self.resolve(first_desc) {
                                        if let Some(dw) = df_resolved.dict_get("DW").and_then(|d| d.as_f64()) {
                                            pdf_font.default_width = dw as f32;
                                        }

                                        if let Some(w_obj) = df_resolved.dict_get("W") {
                                            if let Ok(w_resolved) = self.resolve(w_obj) {
                                                if let Some(w_arr) = w_resolved.as_array() {
                                                    self.parse_cid_font_widths(&mut pdf_font, w_arr);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // /ToUnicode CMap
                    if let Some(to_uni_obj) = resolved_font.dict_get("ToUnicode") {
                        if let Ok(tu_resolved) = self.resolve(to_uni_obj) {
                            if let Some(stream) = tu_resolved.as_stream() {
                                if let Ok(decompressed) = decompress_stream(&stream.data, &stream.dict) {
                                    pdf_font.parse_to_unicode(&decompressed);
                                }
                            }
                        }
                    }

                    fonts.insert(font_alias.clone(), pdf_font);
                }
            }
        }

        Ok(())
    }

        fn parse_cid_font_widths(&mut self, font: &mut PdfFont, w_arr: &[PdfObject]) {
        let mut i = 0;
        while i < w_arr.len() {
            let start_cid = match self.resolve(&w_arr[i]).ok().and_then(|o| o.as_i64()) {
                Some(sc) => sc as u32,
                None => {
                    i += 1;
                    continue;
                }
            };
            i += 1;
            if i >= w_arr.len() {
                break;
            }

            let next_resolved = match self.resolve(&w_arr[i]) {
                Ok(nr) => nr,
                Err(_) => {
                    i += 1;
                    continue;
                }
            };

            if let Some(arr) = next_resolved.as_array() {
                // Format 1: start_cid [ w1 w2 w3 ... ]
                let widths: Vec<f32> = arr.iter().filter_map(|w| self.resolve(w).ok().and_then(|v| v.as_f64()).map(|v| v as f32)).collect();
                font.add_cid_width_entry(start_cid, &widths);
                i += 1;
            } else if let Some(end_cid) = next_resolved.as_i64() {
                // Format 2: start_cid end_cid width
                i += 1;
                if i < w_arr.len() {
                    if let Some(width) = self.resolve(&w_arr[i]).ok().and_then(|w| w.as_f64()) {
                        font.add_cid_width_range(start_cid, end_cid as u32, width as f32);
                    }
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    fn load_page_contents(&mut self, page_obj: &PdfObject) -> Result<Vec<u8>, String> {
        let mut full_contents = Vec::new();

        if let Some(contents_obj) = page_obj.dict_get("Contents") {
            if let Ok(c) = self.resolve(contents_obj) {
                match c {
                    PdfObject::Stream(s) => {
                        if let Ok(decompressed) = decompress_stream(&s.data, &s.dict) {
                            full_contents.extend_from_slice(&decompressed);
                            full_contents.push(b'\n');
                        }
                    }
                    PdfObject::Array(arr) => {
                        for item in arr {
                            if let Ok(resolved_stream) = self.resolve(&item) {
                                if let Some(s) = resolved_stream.as_stream() {
                                    if let Ok(decompressed) = decompress_stream(&s.data, &s.dict) {
                                        full_contents.extend_from_slice(&decompressed);
                                        full_contents.push(b'\n');
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // If the page delegates rendering to a Form XObject (/Subtype /Form) via /Do
        // append the Form stream so the interpreter parses all text and vector graphics seamlessly!
        if let Some(res_obj) = page_obj.dict_get("Resources") {
            if let Ok(res) = self.resolve(res_obj) {
                if let Some(xobj_entry) = res.dict_get("XObject") {
                    if let Ok(xobjs) = self.resolve(xobj_entry) {
                        if let Some(xdict) = xobjs.as_dict() {
                            let mut names: Vec<_> = xdict.keys().cloned().collect();
                            names.sort();
                            for name in names {
                                if let Ok(entry) = self.resolve(&xdict[&name]) {
                                    if let Some(stream) = entry.as_stream() {
                                        let subtype = stream.dict.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");
                                        if subtype == "Form" {
                                            if let Ok(decompressed) = decompress_stream(&stream.data, &stream.dict) {
                                                full_contents.extend_from_slice(&decompressed);
                                                full_contents.push(b'\n');
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(full_contents)
    }

    /// Extracts all atomic raw coordinate spans across all pages
    pub fn parse_raw(&mut self, file_name: &str) -> Result<RawSpatialDocument, String> {
        let start = now_instant();
        let total_pages = self.page_count();
        let mut all_spans = Vec::new();

        for p in 0..total_pages {
            let page_info = self.load_page(p)?;
            let mut interp = ContentInterpreter::new(p + 1, page_info.height, &page_info.fonts);
            interp.interpret(&page_info.contents)?;
            all_spans.extend(interp.spans);
        }

        let elapsed = get_elapsed_ms(start);
        let total_spans = all_spans.len();

        Ok(RawSpatialDocument {
            file_name: file_name.to_string(),
            total_pages,
            total_spans,
            parse_time_ms: (elapsed * 100.0).round() / 100.0,
            spans: all_spans,
        })
    }

    /// Extracts embedded images directly from the PDF object tree
    pub fn extract_all_images(&mut self) -> Result<Vec<ExtractedImageData>, String> {
        let mut out = Vec::new();
        let total_pages = self.page_count();

        for p in 0..total_pages {
            let page_info = self.load_page(p)?;
            let mut interp = ContentInterpreter::new(p + 1, page_info.height, &page_info.fonts);
            let _ = interp.interpret(&page_info.contents);

            let res_obj = match page_info.raw_page_obj.dict_get("Resources") {
                Some(r) => self.resolve(r)?,
                None => continue,
            };

            let xobj_dict = match res_obj.dict_get("XObject") {
                Some(x) => self.resolve(x)?,
                None => continue,
            };

            if let Some(xobjs) = xobj_dict.as_dict() {
                // Collect and sort names for deterministic output
                let mut names: Vec<_> = xobjs.keys().cloned().collect();
                names.sort();

                for (idx, name) in names.into_iter().enumerate() {
                    let entry = &xobjs[&name];
                    let xref = entry.as_reference().map(|r| r.number as i32).unwrap_or(0);
                    let resolved = self.resolve(entry)?;

                    if let Some(stream) = resolved.as_stream() {
                        let subtype = stream.dict.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");
                        if subtype == "Image" {
                            let width = stream.dict.get("Width").and_then(|w| w.as_i64()).unwrap_or(0) as u32;
                            let height = stream.dict.get("Height").and_then(|h| h.as_i64()).unwrap_or(0) as u32;
                            let bpc = stream.dict.get("BitsPerComponent").and_then(|b| b.as_i64()).map(|b| b as i32);
                            let cs = stream.dict.get("ColorSpace").and_then(|c| c.as_name()).map(|s| s.to_string());
                            let filter = stream.dict.get("Filter").and_then(|f| {
                                f.as_name()
                                    .map(|s| s.to_string())
                                    .or_else(|| f.as_array().and_then(|a| a.last().and_then(|e| e.as_name().map(|s| s.to_string()))))
                            });

                            let bbox = interp.image_placements.get(&name).cloned();
                            let raw_stream_bytes = stream.data.clone();

                            let has_bytes = !raw_stream_bytes.is_empty();
                            let byte_len = if has_bytes { Some(raw_stream_bytes.len()) } else { None };
                            let sha256 = if has_bytes { Some(image::sha256_hex(&raw_stream_bytes)) } else { None };
                            let mime_type = image::mime_from_filter(filter.as_deref()).map(str::to_string);

                            let spatial_img = SpatialImage {
                                image_id: idx + 1,
                                name: name.clone(),
                                xref,
                                bbox,
                                pixel_width: width,
                                pixel_height: height,
                                bits_per_component: bpc,
                                color_space: cs,
                                filter: image::last_filter_name(filter.as_deref()),
                                mime_type,
                                byte_length: byte_len,
                                sha256,
                                file: None,
                            };

                            out.push(ExtractedImageData {
                                page: p + 1,
                                image: spatial_img,
                                bytes: raw_stream_bytes,
                            });
                        }
                    }
                }
            }
        }

        Ok(out)
    }

    /// Mode 2: Structured Parse (Hierarchical with Lines, Blocks, and Tables)
    pub fn parse_structured(&mut self, file_name: &str, options: &ParserOptions) -> Result<SpatialDocument, String> {
        let start = now_instant();
        let total_pages = self.page_count();
        let detector = TableDetector::new(
            options.min_table_columns,
            options.column_tolerance,
            options.row_tolerance,
        );

        let mut pages = Vec::with_capacity(total_pages);

        for p in 0..total_pages {
            let page_info = self.load_page(p)?;
            let mut interp = ContentInterpreter::new(p + 1, page_info.height, &page_info.fonts);
            interp.interpret(&page_info.contents)?;

            let raw_coord_spans = interp.spans;
            let drawings = if options.extract_drawings {
                let mut d = interp.drawings;
                if options.filter_background_fills {
                    let page_area = (page_info.width * page_info.height).abs();
                    d.retain(|item| {
                        if item.drawing_type == "stroke" {
                            true
                        } else if page_area > 0.0 {
                            let area = (item.bbox.width * item.bbox.height).abs();
                            area < 0.95 * page_area
                        } else {
                            true
                        }
                    });
                }
                for (idx, item) in d.iter_mut().enumerate() {
                    item.drawing_id = idx + 1;
                }
                d
            } else {
                Vec::new()
            };

            // Convert to RawSpan for table detector and clustering
            let mut raw_spans: Vec<RawSpan> = raw_coord_spans
                .iter()
                .map(|s| RawSpan {
                    x0: s.x0,
                    y0: s.y0,
                    x1: s.x1,
                    y1: s.y1,
                    font_size: s.font_size,
                    text: s.text.clone(),
                })
                .collect();

            // Spatial sort (Top-to-Bottom, Left-to-Right)
            raw_spans.sort_by(|a, b| {
                let y_bucket_a = (a.y0 / options.row_tolerance).round() as i32;
                let y_bucket_b = (b.y0 / options.row_tolerance).round() as i32;
                if y_bucket_a != y_bucket_b {
                    y_bucket_a.cmp(&y_bucket_b)
                } else {
                    a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            // Cluster touching or sub-character glyph spans on the same baseline
            let clustered_spans = cluster_touching_spans(&raw_spans);

            // Table detection
            let (tables, non_table_spans) = if options.detect_tables {
                detector.extract_tables(&clustered_spans, 1)
            } else {
                (Vec::new(), clustered_spans.clone())
            };

            // Group non-table spans into lines and blocks
            let mut text_blocks = Vec::new();
            if !non_table_spans.is_empty() {
                let mut current_block_lines = Vec::new();
                let mut current_line_spans = Vec::new();
                let mut last_y = non_table_spans[0].y0;
                let mut last_line_y = non_table_spans[0].y0;
                let mut block_counter = 1;

                for span in &non_table_spans {
                    let is_same_line = (span.y0 - last_y).abs() <= (options.row_tolerance / 2.0).max(3.0);
                    if !is_same_line {
                        if !current_line_spans.is_empty() {
                            let line = make_spatial_line(&current_line_spans);
                            current_block_lines.push(line);
                            current_line_spans.clear();
                        }

                        let is_new_block = (span.y0 - last_line_y).abs() > 13.5;
                        if is_new_block && !current_block_lines.is_empty() {
                            let block_bbox = compute_lines_bbox(&current_block_lines);
                            text_blocks.push(SpatialBlock {
                                block_id: block_counter,
                                block_type: "text".to_string(),
                                bbox: block_bbox,
                                lines: std::mem::take(&mut current_block_lines),
                            });
                            block_counter += 1;
                        }

                        last_line_y = span.y0;
                    }

                    current_line_spans.push(span.clone());
                    last_y = span.y0;
                }

                if !current_line_spans.is_empty() {
                    let line = make_spatial_line(&current_line_spans);
                    current_block_lines.push(line);
                }

                if !current_block_lines.is_empty() {
                    let block_bbox = compute_lines_bbox(&current_block_lines);
                    text_blocks.push(SpatialBlock {
                        block_id: block_counter,
                        block_type: "text".to_string(),
                        bbox: block_bbox,
                        lines: current_block_lines,
                    });
                }
            }

            // Extract images if requested
            let mut images = Vec::new();
            if options.extract_images {
                let res_obj = page_info.raw_page_obj.dict_get("Resources")
                    .and_then(|r| self.resolve(r).ok());

                if let Some(res) = res_obj {
                    if let Some(xobj_dict) = res.dict_get("XObject").and_then(|x| self.resolve(x).ok()) {
                        if let Some(xobjs) = xobj_dict.as_dict() {
                            let mut names: Vec<_> = xobjs.keys().cloned().collect();
                            names.sort();

                            for (idx, name) in names.into_iter().enumerate() {
                                let entry = &xobjs[&name];
                                let xref = entry.as_reference().map(|r| r.number as i32).unwrap_or(0);
                                if let Ok(resolved) = self.resolve(entry) {
                                    if let Some(stream) = resolved.as_stream() {
                                        let subtype = stream.dict.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");
                                        if subtype == "Image" {
                                            let width = stream.dict.get("Width").and_then(|w| w.as_i64()).unwrap_or(0) as u32;
                                            let height = stream.dict.get("Height").and_then(|h| h.as_i64()).unwrap_or(0) as u32;
                                            let bpc = stream.dict.get("BitsPerComponent").and_then(|b| b.as_i64()).map(|b| b as i32);
                                            let cs = stream.dict.get("ColorSpace").and_then(|c| c.as_name()).map(|s| s.to_string());
                                            let filter = stream.dict.get("Filter").and_then(|f| {
                                                f.as_name()
                                                    .map(|s| s.to_string())
                                                    .or_else(|| f.as_array().and_then(|a| a.last().and_then(|e| e.as_name().map(|s| s.to_string()))))
                                            });

                                            let bbox = interp.image_placements.get(&name).cloned();
                                            let raw_stream_bytes = stream.data.clone();
                                            let has_bytes = !raw_stream_bytes.is_empty();
                                            let byte_len = if has_bytes { Some(raw_stream_bytes.len()) } else { None };
                                            let sha256 = if has_bytes { Some(image::sha256_hex(&raw_stream_bytes)) } else { None };
                                            let mime_type = image::mime_from_filter(filter.as_deref()).map(str::to_string);

                                            images.push(SpatialImage {
                                                image_id: idx + 1,
                                                name: name.clone(),
                                                xref,
                                                bbox,
                                                pixel_width: width,
                                                pixel_height: height,
                                                bits_per_component: bpc,
                                                color_space: cs,
                                                filter: image::last_filter_name(filter.as_deref()),
                                                mime_type,
                                                byte_length: byte_len,
                                                sha256,
                                                file: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            pages.push(SpatialPage {
                page_number: p + 1,
                width: page_info.width,
                height: page_info.height,
                rotation: page_info.rotation,
                text_blocks,
                tables,
                raw_span_count: raw_spans.len(),
                images,
                drawings,
            });
        }

        let elapsed = get_elapsed_ms(start);

        Ok(SpatialDocument {
            file_name: file_name.to_string(),
            total_pages,
            parse_time_ms: (elapsed * 100.0).round() / 100.0,
            pages,
        })
    }
}

fn make_spatial_line(spans: &[RawSpan]) -> SpatialLine {
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y1 = f32::MIN;
    let mut full_text = String::new();
    let mut spatial_spans = Vec::with_capacity(spans.len());

    for (i, span) in spans.iter().enumerate() {
        x0 = x0.min(span.x0);
        y0 = y0.min(span.y0);
        x1 = x1.max(span.x1);
        y1 = y1.max(span.y1);

        if i > 0 && !full_text.ends_with(' ') && !span.text.starts_with(' ') {
            full_text.push(' ');
        }
        full_text.push_str(&span.text);

        spatial_spans.push(SpatialSpan {
            text: span.text.clone(),
            bbox: BoundingBox::new(span.x0, span.y0, span.x1, span.y1),
            font_name: String::new(),
            font_size: span.font_size,
            char_count: span.text.chars().count(),
        });
    }

    SpatialLine {
        text: full_text.trim_end().to_string(),
        bbox: BoundingBox::new(x0, y0, x1, y1),
        spans: spatial_spans,
    }
}

fn compute_lines_bbox(lines: &[SpatialLine]) -> BoundingBox {
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

    if x0 > x1 || y0 > y1 {
        BoundingBox::new(0.0, 0.0, 0.0, 0.0)
    } else {
        BoundingBox::new(x0, y0, x1, y1)
    }
}

/// Clusters touching or sub-character glyph spans on the same baseline.
/// Uses a dynamic threshold based on font size to distinguish genuine continuation/hyphen
/// sub-spans (gap <= 0.75pt) from distinct words in small fonts (word spacing >= 1.59pt).
fn cluster_touching_spans(spans: &[RawSpan]) -> Vec<RawSpan> {
    if spans.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(spans.len());
    let mut current = spans[0].clone();

    for next_span in &spans[1..] {
        let same_baseline = (next_span.y0 - current.y0).abs() <= 3.0;
        let gap = next_span.x0 - current.x1;

        // Dynamic spacing limit based on font size:
        // Inter-word gaps in 6pt fonts are ~1.59pt, while glyph/continuation gaps are <= 0.75pt.
        let char_spacing_limit = (current.font_size * 0.10).clamp(0.4, 0.85);

        // Continuation conditions:
        // 1. Same baseline
        // 2. Not already separated by an explicit space in the text
        // 3. Either touching/overlapping (gap >= -1.0 && gap <= char_spacing_limit),
        //    OR continuing a trailing hyphen/slash (e.g. "NFS-", "0001-") where gap <= 1.2
        let ends_with_hyphen = current.text.ends_with('-') || current.text.ends_with('/');
        let allowed_limit = if ends_with_hyphen {
            char_spacing_limit.max(1.2)
        } else {
            char_spacing_limit
        };

        let has_explicit_space = current.text.ends_with(' ') || next_span.text.starts_with(' ');

        if same_baseline && !has_explicit_space && gap >= -1.0 && gap <= allowed_limit {
            // Merge horizontally touching spans
            current.text.push_str(&next_span.text);
            current.x1 = current.x1.max(next_span.x1);
            current.y0 = current.y0.min(next_span.y0);
            current.y1 = current.y1.max(next_span.y1);
        } else {
            result.push(current);
            current = next_span.clone();
        }
    }

    result.push(current);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_pdf_document() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Length 44 >> stream\nBT\n/F1 12 Tf\n72 700 Td\n(Hello Pure Rust PDF) Tj\nET\nendstream\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 5\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
                o1, o2, o3, o4, xref_pos
            )
            .as_bytes(),
        );

        let mut doc = PurePdfDoc::from_bytes(&pdf).unwrap();
        assert_eq!(doc.page_count(), 1);

        let raw_doc = doc.parse_raw("test.pdf").unwrap();
        assert_eq!(raw_doc.total_pages, 1);
        assert_eq!(raw_doc.total_spans, 1);
        assert_eq!(raw_doc.spans[0].text, "Hello Pure Rust PDF");
        assert_eq!(raw_doc.spans[0].x0, 72.0);
    }

    #[test]
    fn test_parse_real_fixture_text_only() {
        let path = std::path::Path::new("tests/fixtures/text_only.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let mut doc = PurePdfDoc::from_bytes(&bytes).unwrap();
        assert_eq!(doc.page_count(), 1);

        let raw_doc = doc.parse_raw("text_only.pdf").unwrap();
        assert_eq!(raw_doc.total_pages, 1);
        assert!(raw_doc.total_spans > 0, "Must extract spans from real PDF");
        println!("Extracted {} spans from text_only.pdf: {:?}", raw_doc.total_spans, raw_doc.spans);
    }

    #[test]
    fn test_parse_all_benchmark_pdfs() {
        let pdf_files = [
            "tests/fixtures/text_only.pdf",
            "tests/fixtures/mixed.pdf",
            "tests/fixtures/rotated.pdf",
            "tests/fixtures/image_only.pdf",
            "docs/sample.pdf",
        ];

        for path_str in &pdf_files {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                continue;
            }
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    println!("Skip {}: {}", path_str, e);
                    continue;
                }
            };
            let start = std::time::Instant::now();
            match PurePdfDoc::from_bytes(&bytes) {
                Ok(mut doc) => {
                    let page_count = doc.page_count();
                    let raw = doc.parse_raw(path_str).unwrap();
                    let elapsed = start.elapsed();
                    println!(
                        "⚡ [BENCHMARK] {}: {} page(s), {} spans, parsed in {:?}",
                        path_str, page_count, raw.total_spans, elapsed
                    );
                    let limit = if path_str.contains("sample.pdf") { 100 } else { 3 };
                    for span in raw.spans.iter().take(limit) {
                        println!("   ↳ span: [{:.1}, {:.1}, {:.1}, {:.1}] font={} size={} text={:?}",
                            span.x0, span.y0, span.x1, span.y1, span.font_name, span.font_size, span.text);
                    }
                }
                Err(err) => {
                    println!("❌ [ERROR] {}: {}", path_str, err);
                    panic!("Failed to parse {}: {}", path_str, err);
                }
            }
        }
    }

    #[test]
    fn test_benchmark_pypdf_corpus() {
        let pdf_files = [
            "../benchmarks/pdfs/2201.00200.pdf",
            "../benchmarks/pdfs/2201.00029.pdf",
            "../benchmarks/pdfs/1601.03642.pdf",
            "../benchmarks/pdfs/2201.00021.pdf",
        ];

        for path_str in &pdf_files {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                println!("File not found: {}", path_str);
                continue;
            }
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    println!("Could not read {}: {}", path_str, e);
                    continue;
                }
            };
            let start = std::time::Instant::now();
            match PurePdfDoc::from_bytes(&bytes) {
                Ok(mut doc) => {
                    let page_count = doc.page_count();
                    let raw = doc.parse_raw(path_str).unwrap();
                    let elapsed = start.elapsed();
                    let ms_per_page = if page_count > 0 {
                        (elapsed.as_secs_f64() * 1000.0) / (page_count as f64)
                    } else {
                        0.0
                    };
                    println!(
                        "🚀 [PURE RUST vs PYPDF CORPUS] {}: {} pages, {} spans, parsed in {:?} ({:.2} ms/page)",
                        path_str, page_count, raw.total_spans, elapsed, ms_per_page
                    );
                }
                Err(e) => {
                    println!("⚠️ [PARSE INFO] {}: {}", path_str, e);
                }
            }
        }
    }

    #[test]
    fn test_cpu_consumption_benchmark() {
        let path = std::path::Path::new("../benchmarks/pdfs/2201.00200.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();

        // 100 runs x 7 pages = 700 pages!
        let runs = 100;
        let start = std::time::Instant::now();
        let mut total_spans = 0;
        for _ in 0..runs {
            let mut doc = PurePdfDoc::from_bytes(&bytes).unwrap();
            let raw = doc.parse_raw("2201.00200.pdf").unwrap();
            total_spans += raw.total_spans;
        }
        let elapsed = start.elapsed();
        let total_pages = runs * 7;
        let throughput = (total_pages as f64) / elapsed.as_secs_f64();
        println!(
            "🔥 [PURE RUST CPU BENCHMARK] Processed {} pages ({} spans) in {:?} (Throughput: {:.1} pages/sec)",
            total_pages, total_spans, elapsed, throughput
        );
    }

    #[test]
    fn test_parse_structured_tables_and_blocks() {
        let path = std::path::Path::new("docs/sample.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let mut doc = PurePdfDoc::from_bytes(&bytes).unwrap();
        let options = ParserOptions {
            detect_tables: true,
            min_table_columns: 3,
            column_tolerance: 16.0,
            row_tolerance: 6.0,
            ..Default::default()
        };

        let structured = doc.parse_structured("sample.pdf", &options).unwrap();
        assert_eq!(structured.total_pages, 1);
        let page = &structured.pages[0];
        assert_eq!(page.tables.len(), 1, "Deveria ter detectado a tabela de 4 colunas");
        let table = &page.tables[0];
        assert_eq!(table.col_count, 4);
        assert_eq!(table.headers, vec!["Item", "X pt", "Y pt", "Type"]);
        assert_eq!(table.rows.len(), 4);
        assert!(!page.text_blocks.is_empty(), "Deveria ter blocos de texto fora da tabela");
    }

    #[test]
    fn test_extract_all_images_pure_rust() {
        let path = std::path::Path::new("tests/fixtures/mixed.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let mut doc = PurePdfDoc::from_bytes(&bytes).unwrap();
        let images = doc.extract_all_images().unwrap();
        assert_eq!(images.len(), 1);
        let img = &images[0];
        assert_eq!(img.page, 1);
        assert_eq!(img.image.name, "Im0");
        assert_eq!((img.image.pixel_width, img.image.pixel_height), (400, 300));
        assert_eq!(img.image.filter.as_deref(), Some("DCTDecode"));
        assert_eq!(img.image.mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(&img.bytes[..3], [0xFF, 0xD8, 0xFF], "Assinatura JPEG original");
        assert!(img.image.bbox.is_some(), "Deveria ter posicionado a imagem");
        let bbox = img.image.bbox.as_ref().unwrap();
        assert!((bbox.x0 - 330.0).abs() <= 2.0);
        assert!((bbox.x1 - 530.0).abs() <= 2.0);
    }

    #[test]
    fn test_cyclic_reference_does_not_crash() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Length 10 >> stream\nET\nendstream\nendobj\n");
        let o5 = pdf.len();
        // Object 5 references 6, object 6 references 5 (cycle)
        pdf.extend_from_slice(b"5 0 obj\n6 0 R\nendobj\n");
        let o6 = pdf.len();
        pdf.extend_from_slice(b"6 0 obj\n5 0 R\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 7\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
                o1, o2, o3, o4, o5, o6, xref_pos
            )
            .as_bytes(),
        );

        let mut doc = PurePdfDoc::from_bytes(&pdf).unwrap();
        let res = doc.resolve(&PdfObject::Reference(ObjectId::new(5, 0)));
        assert!(res.is_err(), "Cyclic reference resolution must return Err without overflowing stack");
    }

    #[test]
    fn test_cyclic_pages_node_does_not_crash() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        // Pages 2 lists itself in Kids (cycle)
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [ 2 0 R ] /Count 1 >>\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
                o1, o2, xref_pos
            )
            .as_bytes(),
        );

        let res = PurePdfDoc::from_bytes(&pdf);
        assert!(res.is_err(), "Cyclic page tree must return Err without overflowing stack");
    }
}
