use super::lexer::{Lexer, Token};
use std::collections::HashMap;

/// Parsed PDF Font metadata for character width calculation and Unicode mapping
#[derive(Debug, Clone)]
pub struct PdfFont {
    pub name: String,
    pub base_font: String,
    pub subtype: String,
    pub first_char: u32,
    pub last_char: u32,
    pub widths: Vec<f32>,
    pub default_width: f32,
    pub to_unicode: HashMap<u32, String>,
}

impl PdfFont {
    pub fn new(name: String) -> Self {
        Self {
            name,
            base_font: String::new(),
            subtype: "Type1".into(),
            first_char: 0,
            last_char: 255,
            widths: Vec::new(),
            default_width: 500.0,
            to_unicode: HashMap::new(),
        }
    }

    /// Sets the base font name and updates fallback metrics accordingly
    pub fn set_base_font(&mut self, base_font: &str) {
        self.base_font = base_font.to_string();
        if self.widths.is_empty() {
            let lower = base_font.to_lowercase();
            if lower.contains("courier") || lower.contains("mono") {
                self.default_width = 600.0;
            } else {
                self.default_width = 500.0;
            }
        }
    }

    /// Calculates the normalized advance width of a character code in font units (typically 1000 units = 1 em)
    pub fn get_char_width(&self, char_code: u32) -> f32 {
        if char_code >= self.first_char && char_code <= self.last_char {
            let idx = (char_code - self.first_char) as usize;
            if idx < self.widths.len() {
                return self.widths[idx];
            }
        }
        // Base 14 standard font lookup when explicit /Widths table is absent
        let lower = self.base_font.to_lowercase();
        let is_courier = lower.contains("courier") || lower.contains("mono");
        let is_times = lower.contains("times");
        let is_bold = lower.contains("bold");

        if char_code <= 255 {
            let idx = char_code as usize;
            if is_courier {
                return COURIER_WIDTHS[idx];
            } else if is_times {
                return TIMES_ROMAN_WIDTHS[idx];
            } else if is_bold {
                return HELVETICA_BOLD_WIDTHS[idx];
            } else {
                return HELVETICA_WIDTHS[idx];
            }
        }

        self.default_width
    }

    /// Converts a raw character code to Unicode string via /ToUnicode CMap, or fallback to Latin-1/ASCII
    pub fn decode_char(&self, char_code: u32) -> String {
        if let Some(s) = self.to_unicode.get(&char_code) {
            return s.clone();
        }
        if char_code < 256 {
            let c = char_code as u8 as char;
            return c.to_string();
        }
        char::from_u32(char_code).map(|c| c.to_string()).unwrap_or_default()
    }

    /// Decodes a PDF text string byte sequence into Unicode characters with per-glyph advances
    pub fn decode_string(&self, bytes: &[u8]) -> Vec<(String, f32)> {
        let mut result = Vec::new();
        let mut i = 0;

        // Check if 2-byte CID font
        let is_cid = self.subtype == "Type0";

        while i < bytes.len() {
            let char_code = if is_cid && i + 1 < bytes.len() {
                let code = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
                i += 2;
                code
            } else {
                let code = bytes[i] as u32;
                i += 1;
                code
            };

            let ch = self.decode_char(char_code);
            let width = self.get_char_width(char_code) / 1000.0;
            result.push((ch, width));
        }

        result
    }

    /// Parses a /ToUnicode CMap stream
    pub fn parse_to_unicode(&mut self, cmap_bytes: &[u8]) {
        let mut lexer = Lexer::new(cmap_bytes);
        while let Ok(Some(tok)) = lexer.next_token() {
            if let Token::Keyword(ref kw) = tok {
                if kw == "beginbfchar" {
                    self.parse_bfchar(&mut lexer);
                } else if kw == "beginbfrange" {
                    self.parse_bfrange(&mut lexer);
                }
            }
        }
    }

    fn parse_bfchar(&mut self, lexer: &mut Lexer) {
        while let Ok(Some(src_tok)) = lexer.next_token() {
            match src_tok {
                Token::HexString(src_bytes) => {
                    if let Ok(Some(dst_tok)) = lexer.next_token() {
                        let dst_str = match dst_tok {
                            Token::HexString(dst_bytes) => decode_utf16_be(&dst_bytes),
                            Token::LiteralString(dst_bytes) => String::from_utf8_lossy(&dst_bytes).to_string(),
                            _ => continue,
                        };
                        let src_code = read_code(&src_bytes);
                        self.to_unicode.insert(src_code, dst_str);
                    }
                }
                Token::Keyword(ref k) if k == "endbfchar" => break,
                _ => break,
            }
        }
    }

    fn parse_bfrange(&mut self, lexer: &mut Lexer) {
        while let Ok(Some(start_tok)) = lexer.next_token() {
            match start_tok {
                Token::HexString(start_bytes) => {
                    let end_bytes = match lexer.next_token() {
                        Ok(Some(Token::HexString(b))) => b,
                        _ => break,
                    };
                    let dst_tok = match lexer.next_token() {
                        Ok(Some(t)) => t,
                        _ => break,
                    };

                    let start_code = read_code(&start_bytes);
                    let end_code = read_code(&end_bytes);

                    match dst_tok {
                        Token::HexString(dst_bytes) => {
                            let mut base_dst = read_code(&dst_bytes);
                            for c in start_code..=end_code {
                                let ch_str = char::from_u32(base_dst).map(|ch| ch.to_string()).unwrap_or_default();
                                self.to_unicode.insert(c, ch_str);
                                base_dst += 1;
                            }
                        }
                        Token::ArrayOpen => {
                            // Array of mappings
                            let mut cur = start_code;
                            while let Ok(Some(t)) = lexer.next_token() {
                                if t == Token::ArrayClose {
                                    break;
                                }
                                if let Token::HexString(db) = t {
                                    let s = decode_utf16_be(&db);
                                    self.to_unicode.insert(cur, s);
                                    cur += 1;
                                }
                            }
                        }
                        _ => break,
                    }
                }
                Token::Keyword(ref k) if k == "endbfrange" => break,
                _ => break,
            }
        }
    }
}

fn read_code(bytes: &[u8]) -> u32 {
    let mut code = 0u32;
    for &b in bytes {
        code = (code << 8) | (b as u32);
    }
    code
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    if bytes.len() % 2 == 0 {
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| ((chunk[0] as u16) << 8) | (chunk[1] as u16))
            .collect();
        if let Ok(s) = String::from_utf16(&u16s) {
            return s;
        }
    }
    String::from_utf8_lossy(bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_to_unicode_cmap() {
        let cmap = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
1 beginbfchar\n\
<0021> <00480065006C006C006F>\n\
endbfchar\n\
1 beginbfrange\n\
<0030> <0039> <0030>\n\
endbfrange\n\
endcmap\n";

        let mut font = PdfFont::new("TestFont".into());
        font.parse_to_unicode(cmap);

        assert_eq!(font.decode_char(0x21), "Hello");
        assert_eq!(font.decode_char(0x30), "0");
        assert_eq!(font.decode_char(0x35), "5");
        assert_eq!(font.decode_char(0x39), "9");
    }
}

pub static HELVETICA_WIDTHS: [f32; 256] = [
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    278.0, 278.0, 355.0, 556.0, 556.0, 889.0, 667.0, 222.0, 333.0, 333.0, 389.0, 584.0, 278.0, 333.0, 278.0, 278.0,
    556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 278.0, 278.0, 584.0, 584.0, 584.0, 556.0,
    1015.0, 667.0, 667.0, 722.0, 722.0, 667.0, 611.0, 778.0, 722.0, 278.0, 500.0, 667.0, 556.0, 833.0, 722.0, 778.0,
    667.0, 778.0, 722.0, 667.0, 611.0, 722.0, 667.0, 944.0, 667.0, 667.0, 611.0, 278.0, 278.0, 278.0, 469.0, 556.0,
    222.0, 556.0, 556.0, 500.0, 556.0, 556.0, 278.0, 556.0, 556.0, 222.0, 222.0, 500.0, 222.0, 833.0, 556.0, 556.0,
    556.0, 556.0, 333.0, 500.0, 278.0, 556.0, 500.0, 722.0, 500.0, 500.0, 500.0, 334.0, 260.0, 334.0, 584.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 333.0, 556.0, 556.0, 167.0, 556.0, 556.0, 556.0, 556.0, 191.0, 333.0, 556.0, 333.0, 333.0, 500.0, 500.0,
    500.0, 556.0, 556.0, 556.0, 278.0, 500.0, 537.0, 350.0, 222.0, 333.0, 333.0, 556.0, 1000.0, 1000.0, 500.0, 611.0,
    500.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 500.0, 333.0, 333.0, 500.0, 333.0, 333.0, 333.0,
    1000.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 1000.0, 500.0, 370.0, 500.0, 500.0, 500.0, 500.0, 556.0, 778.0, 1000.0, 365.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 889.0, 500.0, 500.0, 500.0, 278.0, 500.0, 500.0, 222.0, 611.0, 944.0, 611.0, 500.0, 500.0, 500.0, 500.0,
];

pub static HELVETICA_BOLD_WIDTHS: [f32; 256] = [
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    278.0, 333.0, 474.0, 556.0, 556.0, 889.0, 722.0, 278.0, 333.0, 333.0, 389.0, 584.0, 278.0, 333.0, 278.0, 278.0,
    556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 333.0, 333.0, 584.0, 584.0, 584.0, 611.0,
    975.0, 722.0, 722.0, 722.0, 722.0, 667.0, 611.0, 778.0, 722.0, 278.0, 556.0, 722.0, 611.0, 833.0, 722.0, 778.0,
    667.0, 778.0, 722.0, 667.0, 611.0, 722.0, 667.0, 944.0, 667.0, 667.0, 611.0, 333.0, 278.0, 333.0, 584.0, 556.0,
    278.0, 556.0, 611.0, 556.0, 611.0, 556.0, 333.0, 611.0, 611.0, 278.0, 278.0, 556.0, 278.0, 889.0, 611.0, 611.0,
    611.0, 611.0, 389.0, 556.0, 333.0, 611.0, 556.0, 778.0, 556.0, 556.0, 500.0, 389.0, 280.0, 389.0, 584.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 333.0, 556.0, 556.0, 167.0, 556.0, 556.0, 556.0, 556.0, 238.0, 500.0, 556.0, 333.0, 333.0, 611.0, 611.0,
    500.0, 556.0, 556.0, 556.0, 278.0, 500.0, 556.0, 350.0, 278.0, 500.0, 500.0, 556.0, 1000.0, 1000.0, 500.0, 611.0,
    500.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 500.0, 333.0, 333.0, 500.0, 333.0, 333.0, 333.0,
    1000.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 1000.0, 500.0, 370.0, 500.0, 500.0, 500.0, 500.0, 611.0, 778.0, 1000.0, 365.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 889.0, 500.0, 500.0, 500.0, 278.0, 500.0, 500.0, 278.0, 611.0, 944.0, 611.0, 500.0, 500.0, 500.0, 500.0,
];

pub static TIMES_ROMAN_WIDTHS: [f32; 256] = [
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    250.0, 333.0, 408.0, 500.0, 500.0, 833.0, 778.0, 333.0, 333.0, 333.0, 500.0, 564.0, 250.0, 333.0, 250.0, 278.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 278.0, 278.0, 564.0, 564.0, 564.0, 444.0,
    921.0, 722.0, 667.0, 667.0, 722.0, 611.0, 556.0, 722.0, 722.0, 333.0, 389.0, 722.0, 611.0, 889.0, 722.0, 722.0,
    556.0, 722.0, 667.0, 556.0, 611.0, 722.0, 722.0, 944.0, 722.0, 722.0, 611.0, 333.0, 278.0, 333.0, 469.0, 500.0,
    333.0, 444.0, 500.0, 444.0, 500.0, 444.0, 333.0, 500.0, 500.0, 278.0, 278.0, 500.0, 278.0, 778.0, 500.0, 500.0,
    500.0, 500.0, 333.0, 389.0, 278.0, 500.0, 500.0, 722.0, 500.0, 500.0, 444.0, 480.0, 200.0, 480.0, 541.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 333.0, 500.0, 500.0, 167.0, 500.0, 500.0, 500.0, 500.0, 180.0, 444.0, 500.0, 333.0, 333.0, 556.0, 556.0,
    500.0, 500.0, 500.0, 500.0, 250.0, 500.0, 453.0, 350.0, 333.0, 444.0, 444.0, 500.0, 1000.0, 1000.0, 500.0, 444.0,
    500.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 333.0, 500.0, 333.0, 333.0, 500.0, 333.0, 333.0, 333.0,
    1000.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 889.0, 500.0, 276.0, 500.0, 500.0, 500.0, 500.0, 611.0, 722.0, 889.0, 310.0, 500.0, 500.0, 500.0, 500.0,
    500.0, 667.0, 500.0, 500.0, 500.0, 278.0, 500.0, 500.0, 278.0, 500.0, 722.0, 500.0, 500.0, 500.0, 500.0, 500.0,
];

pub static COURIER_WIDTHS: [f32; 256] = [
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
];

