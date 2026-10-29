use super::font::PdfFont;
use super::lexer::{Lexer, Token};
use crate::types::{BoundingBox, RawCoordSpan, SpatialDrawing, SpatialDrawingItem};
use std::collections::HashMap;

/// 2D Affine Transformation Matrix [a, b, c, d, e, f]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Matrix {
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Multiplies self by other (self * other)
    pub fn concat(&self, other: &Matrix) -> Matrix {
        Matrix {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Transforms a point (x, y) through this matrix
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let tx = self.a * x + self.c * y + self.e;
        let ty = self.b * x + self.d * y + self.f;
        (tx, ty)
    }
}

/// Graphics state in the PDF interpreter stack
#[derive(Debug, Clone)]
struct GraphicsState {
    ctm: Matrix,
    line_width: f32,
    stroke_color: Option<[f32; 3]>,
    fill_color: Option<[f32; 3]>,
}

impl GraphicsState {
    fn new() -> Self {
        Self {
            ctm: Matrix::identity(),
            line_width: 1.0,
            stroke_color: None,
            fill_color: None,
        }
    }
}

/// Content stream interpreter that extracts 2D atomic spans and vector drawings
pub struct ContentInterpreter<'a> {
    page_num: usize,
    page_height: f32,
    fonts: &'a HashMap<String, PdfFont>,
    state_stack: Vec<GraphicsState>,
    current_state: GraphicsState,

    // Text state
    text_matrix: Matrix,
    text_line_matrix: Matrix,
    current_font_name: String,
    current_font_size: f32,
    text_leading: f32,
    pub in_text: bool,

    // Vector path state
    current_path: Vec<SpatialDrawingItem>,
    current_point: (f32, f32),
    seqno_counter: usize,

    // Extracted results
    pub spans: Vec<RawCoordSpan>,
    pub drawings: Vec<SpatialDrawing>,
    pub image_placements: HashMap<String, BoundingBox>,
}

impl<'a> ContentInterpreter<'a> {
    pub fn new(page_num: usize, page_height: f32, fonts: &'a HashMap<String, PdfFont>) -> Self {
        Self {
            page_num,
            page_height,
            fonts,
            state_stack: Vec::new(),
            current_state: GraphicsState::new(),
            text_matrix: Matrix::identity(),
            text_line_matrix: Matrix::identity(),
            current_font_name: String::new(),
            current_font_size: 10.0,
            text_leading: 0.0,
            in_text: false,
            current_path: Vec::new(),
            current_point: (0.0, 0.0),
            seqno_counter: 0,
            spans: Vec::new(),
            drawings: Vec::new(),
            image_placements: HashMap::new(),
        }
    }

    /// Interprets a decompressed PDF content stream
    pub fn interpret(&mut self, content_bytes: &[u8]) -> Result<(), String> {
        let mut lexer = Lexer::new(content_bytes);
        let mut operands: Vec<Token> = Vec::new();

        while let Some(tok) = lexer.next_token()? {
            match tok {
                Token::Keyword(kw) => {
                    self.execute_operator(&kw, &operands)?;
                    operands.clear();
                }
                Token::ArrayOpen => {
                    // Collect array tokens until ArrayClose
                    let mut arr_tokens = Vec::new();
                    while let Some(sub_tok) = lexer.next_token()? {
                        if sub_tok == Token::ArrayClose {
                            break;
                        }
                        arr_tokens.push(sub_tok);
                    }
                    operands.push(Token::Keyword(format!("__ARR_LEN_{}", arr_tokens.len())));
                    for t in arr_tokens {
                        operands.push(t);
                    }
                }
                other => {
                    operands.push(other);
                }
            }
        }

        Ok(())
    }

    fn execute_operator(&mut self, op: &str, args: &[Token]) -> Result<(), String> {
        match op {
            // --- Graphics state operators ---
            "q" => {
                self.state_stack.push(self.current_state.clone());
            }
            "Q" => {
                if let Some(st) = self.state_stack.pop() {
                    self.current_state = st;
                }
            }
            "cm" => {
                if args.len() >= 6 {
                    let a = get_num(&args[args.len() - 6]);
                    let b = get_num(&args[args.len() - 5]);
                    let c = get_num(&args[args.len() - 4]);
                    let d = get_num(&args[args.len() - 3]);
                    let e = get_num(&args[args.len() - 2]);
                    let f = get_num(&args[args.len() - 1]);
                    let m = Matrix::new(a, b, c, d, e, f);
                    self.current_state.ctm = m.concat(&self.current_state.ctm);
                }
            }
            "w" => {
                if let Some(t) = args.last() {
                    self.current_state.line_width = get_num(t);
                }
            }
            "rg" => {
                if args.len() >= 3 {
                    let r = get_num(&args[args.len() - 3]);
                    let g = get_num(&args[args.len() - 2]);
                    let b = get_num(&args[args.len() - 1]);
                    self.current_state.fill_color = Some([r, g, b]);
                }
            }
            "RG" => {
                if args.len() >= 3 {
                    let r = get_num(&args[args.len() - 3]);
                    let g = get_num(&args[args.len() - 2]);
                    let b = get_num(&args[args.len() - 1]);
                    self.current_state.stroke_color = Some([r, g, b]);
                }
            }

            // --- Text object operators ---
            "BT" => {
                self.in_text = true;
                self.text_matrix = Matrix::identity();
                self.text_line_matrix = Matrix::identity();
            }
            "ET" => {
                self.in_text = false;
            }
            "Tf" => {
                if args.len() >= 2 {
                    if let Token::Name(ref fn_name) = args[args.len() - 2] {
                        self.current_font_name = fn_name.clone();
                    }
                    self.current_font_size = get_num(&args[args.len() - 1]);
                }
            }
            "Tm" => {
                if args.len() >= 6 {
                    let a = get_num(&args[args.len() - 6]);
                    let b = get_num(&args[args.len() - 5]);
                    let c = get_num(&args[args.len() - 4]);
                    let d = get_num(&args[args.len() - 3]);
                    let e = get_num(&args[args.len() - 2]);
                    let f = get_num(&args[args.len() - 1]);
                    let m = Matrix::new(a, b, c, d, e, f);
                    self.text_matrix = m;
                    self.text_line_matrix = m;
                }
            }
            "Td" => {
                if args.len() >= 2 {
                    let tx = get_num(&args[args.len() - 2]);
                    let ty = get_num(&args[args.len() - 1]);
                    let trans = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                    self.text_line_matrix = trans.concat(&self.text_line_matrix);
                    self.text_matrix = self.text_line_matrix;
                }
            }
            "TD" => {
                if args.len() >= 2 {
                    let tx = get_num(&args[args.len() - 2]);
                    let ty = get_num(&args[args.len() - 1]);
                    self.text_leading = -ty;
                    let trans = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                    self.text_line_matrix = trans.concat(&self.text_line_matrix);
                    self.text_matrix = self.text_line_matrix;
                }
            }
            "T*" => {
                let trans = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -self.text_leading);
                self.text_line_matrix = trans.concat(&self.text_line_matrix);
                self.text_matrix = self.text_line_matrix;
            }
            "Tj" => {
                if let Some(t) = args.last() {
                    match t {
                        Token::LiteralString(bytes) | Token::HexString(bytes) => {
                            self.emit_text_string(bytes);
                        }
                        _ => {}
                    }
                }
            }
            "TJ" => {
                for item in args {
                    match item {
                        Token::LiteralString(bytes) | Token::HexString(bytes) => {
                            self.emit_text_string(bytes);
                        }
                        Token::Integer(k) => {
                            let kern = (*k as f32) / 1000.0 * self.current_font_size;
                            let delta = Matrix::new(1.0, 0.0, 0.0, 1.0, -kern, 0.0);
                            self.text_matrix = delta.concat(&self.text_matrix);
                        }
                        Token::Real(k) => {
                            let kern = (*k as f32) / 1000.0 * self.current_font_size;
                            let delta = Matrix::new(1.0, 0.0, 0.0, 1.0, -kern, 0.0);
                            self.text_matrix = delta.concat(&self.text_matrix);
                        }
                        _ => {}
                    }
                }
            }
            "'" => {
                let trans = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -self.text_leading);
                self.text_line_matrix = trans.concat(&self.text_line_matrix);
                self.text_matrix = self.text_line_matrix;
                if let Some(t) = args.last() {
                    if let Token::LiteralString(bytes) | Token::HexString(bytes) = t {
                        self.emit_text_string(bytes);
                    }
                }
            }

            // --- Vector path construction operators ---
            "m" => {
                if args.len() >= 2 {
                    let x = get_num(&args[args.len() - 2]);
                    let y = get_num(&args[args.len() - 1]);
                    let (tx, ty) = self.current_state.ctm.transform_point(x, y);
                    self.current_point = (tx, self.page_height - ty);
                }
            }
            "l" => {
                if args.len() >= 2 {
                    let x = get_num(&args[args.len() - 2]);
                    let y = get_num(&args[args.len() - 1]);
                    let (tx, ty) = self.current_state.ctm.transform_point(x, y);
                    let p2 = (tx, self.page_height - ty);
                    self.current_path.push(SpatialDrawingItem::Line {
                        p1: [self.current_point.0, self.current_point.1],
                        p2: [p2.0, p2.1],
                    });
                    self.current_point = p2;
                }
            }
            "re" => {
                if args.len() >= 4 {
                    let x = get_num(&args[args.len() - 4]);
                    let y = get_num(&args[args.len() - 3]);
                    let w = get_num(&args[args.len() - 2]);
                    let h = get_num(&args[args.len() - 1]);
                    let (p0_x, p0_y) = self.current_state.ctm.transform_point(x, y + h);
                    let (p1_x, p1_y) = self.current_state.ctm.transform_point(x + w, y);
                    let rx0 = p0_x.min(p1_x);
                    let ry0 = (self.page_height - p0_y).min(self.page_height - p1_y);
                    let rx1 = p0_x.max(p1_x);
                    let ry1 = (self.page_height - p0_y).max(self.page_height - p1_y);
                    self.current_path.push(SpatialDrawingItem::Rect {
                        bbox: BoundingBox::new(rx0, ry0, rx1, ry1),
                        orientation: 1,
                    });
                }
            }
            "c" => {
                if args.len() >= 6 {
                    let x1 = get_num(&args[args.len() - 6]);
                    let y1 = get_num(&args[args.len() - 5]);
                    let x2 = get_num(&args[args.len() - 4]);
                    let y2 = get_num(&args[args.len() - 3]);
                    let x3 = get_num(&args[args.len() - 2]);
                    let y3 = get_num(&args[args.len() - 1]);
                    let (p2x, p2y) = self.current_state.ctm.transform_point(x1, y1);
                    let (p3x, p3y) = self.current_state.ctm.transform_point(x2, y2);
                    let (p4x, p4y) = self.current_state.ctm.transform_point(x3, y3);
                    let p4 = (p4x, self.page_height - p4y);
                    self.current_path.push(SpatialDrawingItem::Curve {
                        p1: [self.current_point.0, self.current_point.1],
                        p2: [p2x, self.page_height - p2y],
                        p3: [p3x, self.page_height - p3y],
                        p4: [p4.0, p4.1],
                    });
                    self.current_point = p4;
                }
            }
            "h" => {}

            // --- Vector painting operators ---
            "S" | "s" => {
                self.finish_drawing("stroke", true, false);
            }
            "f" | "F" | "f*" => {
                self.finish_drawing("fill", false, true);
            }
            "B" | "B*" | "b" | "b*" => {
                self.finish_drawing("fill_stroke", true, true);
            }
            "n" => {
                self.current_path.clear();
            }

            // --- XObject invocation ---
            "Do" => {
                if let Some(Token::Name(xname)) = args.last() {
                    // In PDF, an image XObject has unit square dimensions [0, 0] to [1, 1].
                    // The CTM maps this unit square to the user space coordinates.
                    let (p0_x, p0_y) = self.current_state.ctm.transform_point(0.0, 0.0);
                    let (p1_x, p1_y) = self.current_state.ctm.transform_point(1.0, 1.0);
                    let (p2_x, p2_y) = self.current_state.ctm.transform_point(1.0, 0.0);
                    let (p3_x, p3_y) = self.current_state.ctm.transform_point(0.0, 1.0);

                    let min_x = p0_x.min(p1_x).min(p2_x).min(p3_x);
                    let max_x = p0_x.max(p1_x).max(p2_x).max(p3_x);
                    let min_y = p0_y.min(p1_y).min(p2_y).min(p3_y);
                    let max_y = p0_y.max(p1_y).max(p2_y).max(p3_y);

                    let y0 = (self.page_height - max_y).min(self.page_height - min_y);
                    let y1 = (self.page_height - max_y).max(self.page_height - min_y);

                    self.image_placements.insert(
                        xname.clone(),
                        BoundingBox::new(min_x, y0, max_x, y1),
                    );
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn emit_text_string(&mut self, bytes: &[u8]) {
        let font_opt = self.fonts.get(&self.current_font_name);
        let default_font = PdfFont::new(self.current_font_name.clone());
        let font = font_opt.unwrap_or(&default_font);

        let decoded_glyphs = font.decode_string(bytes);
        if decoded_glyphs.is_empty() {
            return;
        }

        let combined_text: String = decoded_glyphs.iter().map(|(s, _)| s.as_str()).collect();
        let total_adv: f32 = decoded_glyphs.iter().map(|(_, w)| *w).sum();
        let word_width = total_adv * self.current_font_size;

        let eff_matrix = self.text_matrix.concat(&self.current_state.ctm);

        let ascent = 0.85 * self.current_font_size;
        let descent = -0.25 * self.current_font_size;

        let (bl_x, bl_y) = eff_matrix.transform_point(0.0, descent);
        let (tr_x, tr_y) = eff_matrix.transform_point(word_width, ascent);

        let x0 = bl_x.min(tr_x);
        let x1 = bl_x.max(tr_x);
        let y0 = (self.page_height - tr_y).min(self.page_height - bl_y);
        let y1 = (self.page_height - tr_y).max(self.page_height - bl_y);

        // Only emit spans with non-whitespace text to avoid emitting blank bounding boxes
        let trimmed = combined_text.trim_matches(|c: char| c.is_whitespace() || c == '\u{00A0}');
        if !trimmed.is_empty() {
            self.spans.push(RawCoordSpan {
                page: self.page_num,
                x0: round_coord(x0),
                y0: round_coord(y0),
                x1: round_coord(x1),
                y1: round_coord(y1),
                font_size: self.current_font_size,
                font_name: self.current_font_name.clone(),
                text: combined_text,
            });
        }

        let advance_matrix = Matrix::new(1.0, 0.0, 0.0, 1.0, word_width, 0.0);
        self.text_matrix = advance_matrix.concat(&self.text_matrix);
    }

    fn finish_drawing(&mut self, dtype: &str, is_stroke: bool, is_fill: bool) {
        if self.current_path.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for item in &self.current_path {
            match item {
                SpatialDrawingItem::Line { p1, p2 } => {
                    min_x = min_x.min(p1[0]).min(p2[0]);
                    min_y = min_y.min(p1[1]).min(p2[1]);
                    max_x = max_x.max(p1[0]).max(p2[0]);
                    max_y = max_y.max(p1[1]).max(p2[1]);
                }
                SpatialDrawingItem::Curve { p1, p2, p3, p4 } => {
                    for p in [p1, p2, p3, p4] {
                        min_x = min_x.min(p[0]);
                        min_y = min_y.min(p[1]);
                        max_x = max_x.max(p[0]);
                        max_y = max_y.max(p[1]);
                    }
                }
                SpatialDrawingItem::Rect { bbox, .. } => {
                    min_x = min_x.min(bbox.x0);
                    min_y = min_y.min(bbox.y0);
                    max_x = max_x.max(bbox.x1);
                    max_y = max_y.max(bbox.y1);
                }
                SpatialDrawingItem::Quad { points } => {
                    for p in points {
                        min_x = min_x.min(p[0]);
                        min_y = min_y.min(p[1]);
                        max_x = max_x.max(p[0]);
                        max_y = max_y.max(p[1]);
                    }
                }
            }
        }

        let bbox = if min_x < max_x && min_y < max_y {
            BoundingBox::new(min_x, min_y, max_x, max_y)
        } else {
            BoundingBox::new(0.0, 0.0, 0.0, 0.0)
        };

        self.seqno_counter += 1;
        let items = std::mem::take(&mut self.current_path);
        let item_count = items.len();

        self.drawings.push(SpatialDrawing {
            drawing_id: self.seqno_counter,
            bbox,
            drawing_type: dtype.to_string(),
            item_count,
            items,
            stroke_color: if is_stroke { self.current_state.stroke_color } else { None },
            fill_color: if is_fill { self.current_state.fill_color } else { None },
            width: if is_stroke { Some(self.current_state.line_width) } else { None },
            fill_opacity: None,
            stroke_opacity: None,
            even_odd: None,
            dash_pattern: None,
            dash_phase: None,
            line_join: None,
            line_cap: None,
            layer: None,
            seqno: self.seqno_counter,
        });
    }
}

fn get_num(tok: &Token) -> f32 {
    match tok {
        Token::Integer(i) => *i as f32,
        Token::Real(f) => *f as f32,
        _ => 0.0,
    }
}

fn round_coord(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_concat_and_transform() {
        let m = Matrix::new(1.0, 0.0, 0.0, 1.0, 50.0, 100.0);
        let (x, y) = m.transform_point(10.0, 20.0);
        assert_eq!((x, y), (60.0, 120.0));
    }

    #[test]
    fn test_interpret_content_stream_text() {
        let stream = b"BT\n\
/F1 12 Tf\n\
72 712 Td\n\
(Hello RasgaPDF) Tj\n\
ET\n";

        let mut fonts = HashMap::new();
        let mut f1 = PdfFont::new("F1".into());
        f1.default_width = 500.0;
        fonts.insert("F1".into(), f1);

        let mut interp = ContentInterpreter::new(1, 842.0, &fonts);
        interp.interpret(stream).unwrap();

        assert_eq!(interp.spans.len(), 1);
        let span = &interp.spans[0];
        assert_eq!(span.text, "Hello RasgaPDF");
        assert_eq!(span.font_name, "F1");
        assert_eq!(span.font_size, 12.0);
        assert_eq!(span.page, 1);
        assert_eq!(span.x0, 72.0);
        assert!(span.y0 < span.y1);
        assert!(span.x0 < span.x1);
    }

    #[test]
    fn test_interpret_vector_rect() {
        let stream = b"0 0 1 rg\n\
2 w\n\
50 50 200 100 re\n\
B\n";
        let fonts = HashMap::new();
        let mut interp = ContentInterpreter::new(1, 842.0, &fonts);
        interp.interpret(stream).unwrap();

        assert_eq!(interp.drawings.len(), 1);
        let d = &interp.drawings[0];
        assert_eq!(d.width, Some(2.0));
        assert_eq!(d.fill_color, Some([0.0, 0.0, 1.0]));
    }
}
