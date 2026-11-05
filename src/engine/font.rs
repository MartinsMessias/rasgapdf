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
    pub ascent: f32,
    pub descent: f32,
    pub to_unicode: HashMap<u32, String>,
    pub encoding_map: HashMap<u32, String>,
    pub cid_widths: HashMap<u32, f32>,
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
            ascent: 0.95,
            descent: -0.25,
            to_unicode: HashMap::new(),
            encoding_map: HashMap::new(),
            cid_widths: HashMap::new(),
        }
    }

    /// Sets the base font name and updates fallback metrics accordingly
    pub fn add_cid_width_entry(&mut self, start_cid: u32, widths: &[f32]) {
        for (i, &w) in widths.iter().enumerate() {
            self.cid_widths.insert(start_cid + i as u32, w);
        }
    }

    pub fn add_cid_width_range(&mut self, start_cid: u32, end_cid: u32, width: f32) {
        if start_cid <= end_cid && (end_cid - start_cid) <= 65536 {
            for c in start_cid..=end_cid {
                self.cid_widths.insert(c, width);
            }
        }
    }

    pub fn set_base_font(&mut self, base_font: &str) {
        self.base_font = base_font.to_string();
        let lower = base_font.to_lowercase();
        let is_courier = lower.contains("courier") || lower.contains("mono");
        let is_times = lower.contains("times");
        let is_bold = lower.contains("bold") || lower.contains("black") || lower.contains("heavy");

        if is_courier {
            if is_bold {
                self.ascent = 1.00;
                self.descent = -0.39;
            } else {
                self.ascent = 0.93;
                self.descent = -0.32;
            }
        } else if is_times {
            if is_bold {
                self.ascent = 1.04;
                self.descent = -0.34;
            } else {
                self.ascent = 1.05;
                self.descent = -0.28;
            }
        } else {
            // Helvetica / default sans-serif
            if is_bold {
                self.ascent = 1.07;
                self.descent = -0.31;
            } else {
                self.ascent = 1.075;
                self.descent = -0.30;
            }
        }

        if self.widths.is_empty() {
            if is_courier {
                self.default_width = 600.0;
            } else {
                self.default_width = 500.0;
            }
        }
    }

    /// Calculates the normalized advance width of a character code in font units (typically 1000 units = 1 em)
    pub fn get_char_width(&self, char_code: u32) -> f32 {
        if let Some(&w) = self.cid_widths.get(&char_code) {
            return w;
        }
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
        let is_bold = lower.contains("bold") || lower.contains("black") || lower.contains("heavy");

        if char_code <= 255 {
            let idx = char_code as usize;
            let w = if is_courier {
                if is_bold {
                    COURIER_BOLD_WIDTHS[idx]
                } else {
                    COURIER_WIDTHS[idx]
                }
            } else if is_times {
                if is_bold {
                    TIMES_BOLD_WIDTHS[idx]
                } else {
                    TIMES_ROMAN_WIDTHS[idx]
                }
            } else if is_bold {
                HELVETICA_BOLD_WIDTHS[idx]
            } else {
                HELVETICA_WIDTHS[idx]
            };
            if w > 0.0 {
                return w;
            }
        }

        self.default_width
    }

    /// Maps Windows-1252 (WinAnsiEncoding) bytes to Unicode chars.
    /// For 0x00..=0x7F and 0xA0..=0xFF, CP-1252 is identical to Latin-1/Unicode.
    /// The range 0x80..=0x9F contains Windows-1252 specific typographical characters.
    pub fn cp1252_to_char(b: u8) -> char {
        match b {
            0x80 => '€',    // Euro sign
            0x82 => '‚',    // Single low-9 quotation mark
            0x83 => 'ƒ',    // Latin small letter f with hook
            0x84 => '„',    // Double low-9 quotation mark
            0x85 => '…',    // Horizontal ellipsis
            0x86 => '†',    // Dagger
            0x87 => '‡',    // Double dagger
            0x88 => 'ˆ',    // Modifier letter circumflex accent
            0x89 => '‰',    // Per mille sign
            0x8A => 'Š',    // Latin capital letter S with caron
            0x8B => '‹',    // Single left-pointing angle quotation mark
            0x8C => 'Œ',    // Latin capital ligature OE
            0x8E => 'Ž',    // Latin capital letter Z with caron
            0x91 => '‘',    // Left single quotation mark
            0x92 => '’',    // Right single quotation mark
            0x93 => '“',    // Left double quotation mark
            0x94 => '”',    // Right double quotation mark
            0x95 => '•',    // Bullet
            0x96 => '–',    // En dash (U+2013)
            0x97 => '—',    // Em dash (U+2014)
            0x98 => '˜',    // Small tilde
            0x99 => '™',    // Trade mark sign
            0x9A => 'š',    // Latin small letter s with caron
            0x9B => '›',    // Single right-pointing angle quotation mark
            0x9C => 'œ',    // Latin small ligature oe
            0x9E => 'ž',    // Latin small letter z with caron
            0x9F => 'Ÿ',    // Latin capital letter Y with diaeresis
            _ => b as char, // ASCII (0x00..=0x7F) and Latin-1 supplement (0xA0..=0xFF)
        }
    }

    /// Applies a named base encoding (e.g., `/MacRomanEncoding` or `/WinAnsiEncoding`) per ISO 32000-2 §9.6.5
    pub fn apply_base_encoding(&mut self, encoding_name: &str) {
        if encoding_name == "MacRomanEncoding" {
            for &(code, s) in MAC_ROMAN_HIGH_BYTES {
                self.encoding_map.insert(code as u32, s.to_string());
            }
        }
    }

    /// Applies an entry from a font `/Encoding` `/Differences` array per ISO 32000-2 §9.6.5.2
    pub fn apply_differences_entry(&mut self, code: u32, glyph_name: &str) {
        if let Some(mapped) = Self::resolve_glyph_name(glyph_name) {
            self.encoding_map.insert(code, mapped);
        }
    }

    /// Resolves an Adobe Glyph List (AGL) or `uniXXXX`/`uXXXX` glyph name to its Unicode string
    pub fn resolve_glyph_name(raw_name: &str) -> Option<String> {
        let base = raw_name.split('.').next().unwrap_or(raw_name);
        if base.is_empty() || base == ".notdef" {
            return None;
        }

        // Single ASCII letter or digit
        if base.len() == 1 {
            let b = base.as_bytes()[0];
            if b.is_ascii_alphanumeric() {
                return Some(base.to_string());
            }
        }

        // uniXXXX (one or more 4-hex-digit UTF-16 code units)
        if let Some(hex_part) = base.strip_prefix("uni") {
            if !hex_part.is_empty()
                && hex_part.len() % 4 == 0
                && hex_part.bytes().all(|b| b.is_ascii_hexdigit())
            {
                let mut u16s = Vec::with_capacity(hex_part.len() / 4);
                for chunk in hex_part.as_bytes().chunks_exact(4) {
                    if let Ok(s) = std::str::from_utf8(chunk) {
                        if let Ok(v) = u16::from_str_radix(s, 16) {
                            u16s.push(v);
                        }
                    }
                }
                if let Ok(decoded) = String::from_utf16(&u16s) {
                    if !decoded.is_empty() {
                        return Some(decoded);
                    }
                }
            }
        }

        // uXXXX .. uXXXXXX scalar value
        if let Some(hex_part) = base.strip_prefix('u') {
            if (4..=6).contains(&hex_part.len()) && hex_part.bytes().all(|b| b.is_ascii_hexdigit())
            {
                if let Ok(code) = u32::from_str_radix(hex_part, 16) {
                    if let Some(ch) = char::from_u32(code) {
                        return Some(ch.to_string());
                    }
                }
            }
        }

        let mapped = match base {
            // Ligatures
            "fi" => "fi",
            "fl" => "fl",
            "ff" => "ff",
            "ffi" => "ffi",
            "ffl" => "ffl",
            "AE" => "Æ",
            "ae" => "æ",
            "OE" => "Œ",
            "oe" => "œ",
            "IJ" => "IJ",
            "ij" => "ij",
            // Whitespace & ASCII punctuation / digits
            "space" => " ",
            "nbspace" | "nonbreakingspace" => "\u{00A0}",
            "exclam" => "!",
            "quotedbl" => "\"",
            "numbersign" => "#",
            "dollar" => "$",
            "percent" => "%",
            "ampersand" => "&",
            "quotesingle" => "'",
            "parenleft" => "(",
            "parenright" => ")",
            "asterisk" => "*",
            "plus" => "+",
            "comma" => ",",
            "hyphen" | "minus" => "-",
            "sfthyphen" | "softhyphen" => "\u{00AD}",
            "period" => ".",
            "slash" => "/",
            "zero" => "0",
            "one" => "1",
            "two" => "2",
            "three" => "3",
            "four" => "4",
            "five" => "5",
            "six" => "6",
            "seven" => "7",
            "eight" => "8",
            "nine" => "9",
            "colon" => ":",
            "semicolon" => ";",
            "less" => "<",
            "equal" => "=",
            "greater" => ">",
            "question" => "?",
            "at" => "@",
            "bracketleft" => "[",
            "backslash" => "\\",
            "bracketright" => "]",
            "asciicircum" => "^",
            "underscore" => "_",
            "grave" => "`",
            "braceleft" => "{",
            "bar" | "brokenbar" => "|",
            "braceright" => "}",
            "asciitilde" => "~",
            // Typography & Symbols
            "bullet" => "•",
            "endash" => "–",
            "emdash" => "—",
            "quoteleft" => "‘",
            "quoteright" => "’",
            "quotedblleft" => "“",
            "quotedblright" => "”",
            "quotesinglbase" => "‚",
            "quotedblbase" => "„",
            "guillemotleft" => "«",
            "guillemotright" => "»",
            "guilsinglleft" => "‹",
            "guilsinglright" => "›",
            "ellipsis" => "…",
            "dagger" => "†",
            "daggerdbl" => "‡",
            "perthousand" => "‰",
            "Euro" | "euro" => "€",
            "sterling" => "£",
            "yen" => "¥",
            "cent" => "¢",
            "currency" => "¤",
            "florin" => "ƒ",
            "copyright" => "©",
            "registered" => "®",
            "trademark" => "™",
            "degree" => "°",
            "plusminus" => "±",
            "multiply" => "×",
            "divide" => "÷",
            "section" => "§",
            "paragraph" => "¶",
            "ordfeminine" => "ª",
            "ordmasculine" => "º",
            "exclamdown" => "¡",
            "questiondown" => "¿",
            "logicalnot" => "¬",
            "mu" | "micro" => "µ",
            "periodcentered" | "middot" => "·",
            "onesuperior" => "¹",
            "twosuperior" => "²",
            "threesuperior" => "³",
            "onequarter" => "¼",
            "onehalf" => "½",
            "threequarters" => "¾",
            // Latin-1 & Latin Extended Accents
            "Agrave" => "À",
            "Aacute" => "Á",
            "Acircumflex" => "Â",
            "Atilde" => "Ã",
            "Adieresis" => "Ä",
            "Aring" => "Å",
            "Ccedilla" => "Ç",
            "Egrave" => "È",
            "Eacute" => "É",
            "Ecircumflex" => "Ê",
            "Edieresis" => "Ë",
            "Igrave" => "Ì",
            "Iacute" => "Í",
            "Icircumflex" => "Î",
            "Idieresis" => "Ï",
            "Eth" => "Ð",
            "Ntilde" => "Ñ",
            "Ograve" => "Ò",
            "Oacute" => "Ó",
            "Ocircumflex" => "Ô",
            "Otilde" => "Õ",
            "Odieresis" => "Ö",
            "Oslash" => "Ø",
            "Ugrave" => "Ù",
            "Uacute" => "Ú",
            "Ucircumflex" => "Û",
            "Udieresis" => "Ü",
            "Yacute" => "Ý",
            "Ydieresis" => "Ÿ",
            "Thorn" => "Þ",
            "germandbls" => "ß",
            "agrave" => "à",
            "aacute" => "á",
            "acircumflex" => "â",
            "atilde" => "ã",
            "adieresis" => "ä",
            "aring" => "å",
            "ccedilla" => "ç",
            "egrave" => "è",
            "eacute" => "é",
            "ecircumflex" => "ê",
            "edieresis" => "ë",
            "igrave" => "ì",
            "iacute" => "í",
            "icircumflex" => "î",
            "idieresis" => "ï",
            "eth" => "ð",
            "ntilde" => "ñ",
            "ograve" => "ò",
            "oacute" => "ó",
            "ocircumflex" => "ô",
            "otilde" => "õ",
            "odieresis" => "ö",
            "oslash" => "ø",
            "ugrave" => "ù",
            "uacute" => "ú",
            "ucircumflex" => "û",
            "udieresis" => "ü",
            "yacute" => "ý",
            "thorn" => "þ",
            "ydieresis" => "ÿ",
            "Scaron" => "Š",
            "scaron" => "š",
            "Zcaron" => "Ž",
            "zcaron" => "ž",
            "Lslash" => "Ł",
            "lslash" => "ł",
            "dotlessi" => "ı",
            "circumflex" => "ˆ",
            "tilde" => "˜",
            "macron" => "¯",
            "acute" => "´",
            "dieresis" => "¨",
            "cedilla" => "¸",
            _ => return None,
        };
        Some(mapped.to_string())
    }

    /// Converts a raw character code to Unicode string via /ToUnicode CMap, /Encoding (/Differences or BaseEncoding), or fallback to Windows-1252 / Latin-1 / ASCII per ISO 32000-2:2020 §9.10.2
    pub fn decode_char(&self, char_code: u32) -> String {
        if let Some(s) = self.to_unicode.get(&char_code) {
            return s.clone();
        }
        if let Some(s) = self.encoding_map.get(&char_code) {
            return s.clone();
        }
        if char_code < 256 {
            let c = Self::cp1252_to_char(char_code as u8);
            return c.to_string();
        }
        char::from_u32(char_code)
            .map(|c| c.to_string())
            .unwrap_or_default()
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
                            Token::LiteralString(dst_bytes) => decode_pdf_text_string(&dst_bytes),
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
                            let base_dst = read_code(&dst_bytes);
                            for (c, current_dst) in (start_code..=end_code).zip(base_dst..) {
                                let ch_str = char::from_u32(current_dst)
                                    .map(|ch| ch.to_string())
                                    .unwrap_or_default();
                                self.to_unicode.insert(c, ch_str);
                            }
                        }
                        Token::ArrayOpen => {
                            // Array of mappings
                            let mut cur = start_code;
                            while let Ok(Some(t)) = lexer.next_token() {
                                if t == Token::ArrayClose {
                                    break;
                                }
                                match t {
                                    Token::HexString(db) => {
                                        let s = decode_utf16_be(&db);
                                        self.to_unicode.insert(cur, s);
                                        cur += 1;
                                    }
                                    Token::LiteralString(db) => {
                                        let s = decode_pdf_text_string(&db);
                                        self.to_unicode.insert(cur, s);
                                        cur += 1;
                                    }
                                    _ => {}
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

/// Decodes a PDF 2.0 (ISO 32000-2:2020 §7.9.2.2) text string:
/// - `0xEF 0xBB 0xBF` prefix -> UTF-8 encoded string (new in PDF 2.0)
/// - `0xFE 0xFF` prefix -> UTF-16BE encoded string
/// - Otherwise valid UTF-8 or PDFDocEncoding / Windows-1252 fallback
pub fn decode_pdf_text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16_be(rest);
    }
    if let Ok(utf8_str) = std::str::from_utf8(bytes) {
        return utf8_str.to_string();
    }
    bytes.iter().map(|&b| PdfFont::cp1252_to_char(b)).collect()
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    // ISO 32000-2:2020 §7.9.2.2: Support UTF-8 BOM (EF BB BF) and UTF-16BE BOM (FE FF)
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let payload = bytes.strip_prefix(&[0xFE, 0xFF]).unwrap_or(bytes);
    if payload.len() % 2 == 0 {
        let u16s: Vec<u16> = payload
            .chunks_exact(2)
            .map(|chunk| ((chunk[0] as u16) << 8) | (chunk[1] as u16))
            .collect();
        if let Ok(s) = String::from_utf16(&u16s) {
            return s;
        }
    }
    String::from_utf8_lossy(payload).to_string()
}

static MAC_ROMAN_HIGH_BYTES: &[(u8, &str)] = &[
    (0x80, "Ä"),
    (0x81, "Å"),
    (0x82, "Ç"),
    (0x83, "É"),
    (0x84, "Ñ"),
    (0x85, "Ö"),
    (0x86, "Ü"),
    (0x87, "á"),
    (0x88, "à"),
    (0x89, "â"),
    (0x8A, "ä"),
    (0x8B, "ã"),
    (0x8C, "å"),
    (0x8D, "ç"),
    (0x8E, "é"),
    (0x8F, "è"),
    (0x90, "ê"),
    (0x91, "ë"),
    (0x92, "í"),
    (0x93, "ì"),
    (0x94, "î"),
    (0x95, "ï"),
    (0x96, "ñ"),
    (0x97, "ó"),
    (0x98, "ò"),
    (0x99, "ô"),
    (0x9A, "ö"),
    (0x9B, "õ"),
    (0x9C, "ú"),
    (0x9D, "ù"),
    (0x9E, "û"),
    (0x9F, "ü"),
    (0xA0, "†"),
    (0xA1, "°"),
    (0xA2, "¢"),
    (0xA3, "£"),
    (0xA4, "§"),
    (0xA5, "•"),
    (0xA6, "¶"),
    (0xA7, "ß"),
    (0xA8, "®"),
    (0xA9, "©"),
    (0xAA, "™"),
    (0xAB, "´"),
    (0xAC, "¨"),
    (0xAE, "Æ"),
    (0xAF, "Ø"),
    (0xB4, "¥"),
    (0xB5, "µ"),
    (0xBB, "ª"),
    (0xBC, "º"),
    (0xBE, "æ"),
    (0xBF, "ø"),
    (0xC0, "¿"),
    (0xC1, "¡"),
    (0xC2, "¬"),
    (0xC4, "ƒ"),
    (0xC7, "«"),
    (0xC8, "»"),
    (0xC9, "…"),
    (0xCA, "\u{00A0}"),
    (0xCB, "À"),
    (0xCC, "Ã"),
    (0xCD, "Õ"),
    (0xCE, "Œ"),
    (0xCF, "œ"),
    (0xD0, "–"),
    (0xD1, "—"),
    (0xD2, "“"),
    (0xD3, "”"),
    (0xD4, "‘"),
    (0xD5, "’"),
    (0xD6, "÷"),
    (0xD8, "ÿ"),
    (0xD9, "Ÿ"),
    (0xDA, "/"),
    (0xDB, "€"),
    (0xDC, "‹"),
    (0xDD, "›"),
    (0xDE, "fi"),
    (0xDF, "fl"),
    (0xE0, "‡"),
    (0xE1, "·"),
    (0xE2, "‚"),
    (0xE3, "„"),
    (0xE4, "‰"),
    (0xE5, "Â"),
    (0xE6, "Ê"),
    (0xE7, "Á"),
    (0xE8, "Ë"),
    (0xE9, "È"),
    (0xEA, "Í"),
    (0xEB, "Î"),
    (0xEC, "Ï"),
    (0xED, "Ì"),
    (0xEE, "Ó"),
    (0xEF, "Ô"),
    (0xF1, "Ò"),
    (0xF2, "Ú"),
    (0xF3, "Û"),
    (0xF4, "Ù"),
    (0xF5, "ı"),
    (0xF6, "ˆ"),
    (0xF7, "˜"),
    (0xF8, "¯"),
    (0xFA, "˙"),
    (0xFB, "˚"),
    (0xFC, "¸"),
    (0xFD, "˝"),
    (0xFE, "˛"),
    (0xFF, "ˇ"),
];

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

    #[test]
    fn test_pdf20_utf8_and_utf16be_bom_text_strings() {
        // ISO 32000-2:2020 §7.9.2.2 UTF-8 BOM (EF BB BF) + "São Paulo"
        let mut utf8_bom = vec![0xEF, 0xBB, 0xBF];
        utf8_bom.extend_from_slice("São Paulo".as_bytes());
        assert_eq!(decode_pdf_text_string(&utf8_bom), "São Paulo");

        // ISO 32000-2:2020 §7.9.2.2 UTF-16BE BOM (FE FF) + "Ação"
        let utf16_bom = vec![0xFE, 0xFF, 0x00, 0x41, 0x00, 0xE7, 0x00, 0xE3, 0x00, 0x6F];
        assert_eq!(decode_pdf_text_string(&utf16_bom), "Ação");
        assert_eq!(decode_utf16_be(&utf16_bom), "Ação");
    }

    #[test]
    fn test_font_encoding_differences_and_mac_roman() {
        let mut font = PdfFont::new("F1".into());
        font.apply_base_encoding("MacRomanEncoding");
        assert_eq!(font.decode_char(0x8E), "é");
        assert_eq!(font.decode_char(0xDE), "fi");

        // /Differences overrides BaseEncoding
        font.apply_differences_entry(1, "fi");
        font.apply_differences_entry(2, "eacute.sc");
        font.apply_differences_entry(3, "uni00E3");
        font.apply_differences_entry(4, "Euro");
        assert_eq!(font.decode_char(1), "fi");
        assert_eq!(font.decode_char(2), "é");
        assert_eq!(font.decode_char(3), "ã");
        assert_eq!(font.decode_char(4), "€");
    }
}

pub static TIMES_ROMAN_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 250.0, 333.0, 408.0,
    500.0, 500.0, 833.0, 778.0, 180.0, 333.0, 333.0, 500.0, 564.0, 250.0, 333.0, 250.0, 278.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 278.0, 278.0, 564.0,
    564.0, 564.0, 444.0, 921.0, 722.0, 667.0, 667.0, 722.0, 611.0, 556.0, 722.0, 722.0, 333.0,
    389.0, 722.0, 611.0, 889.0, 722.0, 722.0, 556.0, 722.0, 667.0, 556.0, 611.0, 722.0, 722.0,
    944.0, 722.0, 722.0, 611.0, 333.0, 278.0, 333.0, 469.0, 500.0, 333.0, 444.0, 500.0, 444.0,
    500.0, 444.0, 333.0, 500.0, 500.0, 278.0, 278.0, 500.0, 278.0, 778.0, 500.0, 500.0, 500.0,
    500.0, 333.0, 389.0, 278.0, 500.0, 500.0, 722.0, 500.0, 500.0, 444.0, 480.0, 200.0, 480.0,
    541.0, 0.0, 500.0, 500.0, 333.0, 500.0, 444.0, 1000.0, 500.0, 500.0, 333.0, 1000.0, 556.0,
    333.0, 889.0, 500.0, 611.0, 500.0, 500.0, 333.0, 333.0, 444.0, 444.0, 350.0, 500.0, 1000.0,
    333.0, 980.0, 389.0, 333.0, 722.0, 500.0, 444.0, 722.0, 250.0, 333.0, 500.0, 500.0, 500.0,
    500.0, 200.0, 500.0, 333.0, 760.0, 276.0, 500.0, 564.0, 333.0, 760.0, 333.0, 400.0, 564.0,
    300.0, 300.0, 333.0, 500.0, 453.0, 250.0, 333.0, 300.0, 310.0, 500.0, 750.0, 750.0, 750.0,
    444.0, 722.0, 722.0, 722.0, 722.0, 722.0, 722.0, 889.0, 667.0, 611.0, 611.0, 611.0, 611.0,
    333.0, 333.0, 333.0, 333.0, 722.0, 722.0, 722.0, 722.0, 722.0, 722.0, 722.0, 564.0, 722.0,
    722.0, 722.0, 722.0, 722.0, 722.0, 556.0, 500.0, 444.0, 444.0, 444.0, 444.0, 444.0, 444.0,
    667.0, 444.0, 444.0, 444.0, 444.0, 444.0, 278.0, 278.0, 278.0, 278.0, 500.0, 500.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 564.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
];

pub static TIMES_BOLD_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 250.0, 333.0, 555.0,
    500.0, 500.0, 1000.0, 833.0, 278.0, 333.0, 333.0, 500.0, 570.0, 250.0, 333.0, 250.0, 278.0,
    500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0, 333.0, 333.0, 570.0,
    570.0, 570.0, 500.0, 930.0, 722.0, 667.0, 722.0, 722.0, 667.0, 611.0, 778.0, 778.0, 389.0,
    500.0, 778.0, 667.0, 944.0, 722.0, 778.0, 611.0, 778.0, 722.0, 556.0, 667.0, 722.0, 722.0,
    1000.0, 722.0, 722.0, 667.0, 333.0, 278.0, 333.0, 581.0, 500.0, 333.0, 500.0, 556.0, 444.0,
    556.0, 444.0, 333.0, 500.0, 556.0, 278.0, 333.0, 556.0, 278.0, 833.0, 556.0, 500.0, 556.0,
    556.0, 444.0, 389.0, 333.0, 556.0, 500.0, 722.0, 500.0, 500.0, 444.0, 394.0, 220.0, 394.0,
    520.0, 0.0, 500.0, 500.0, 333.0, 500.0, 500.0, 1000.0, 500.0, 500.0, 333.0, 1000.0, 556.0,
    333.0, 1000.0, 500.0, 667.0, 500.0, 500.0, 333.0, 333.0, 500.0, 500.0, 350.0, 500.0, 1000.0,
    333.0, 1000.0, 389.0, 333.0, 722.0, 500.0, 444.0, 722.0, 250.0, 333.0, 500.0, 500.0, 500.0,
    500.0, 220.0, 500.0, 333.0, 747.0, 300.0, 500.0, 570.0, 333.0, 747.0, 333.0, 400.0, 570.0,
    300.0, 300.0, 333.0, 556.0, 540.0, 250.0, 333.0, 300.0, 330.0, 500.0, 750.0, 750.0, 750.0,
    500.0, 722.0, 722.0, 722.0, 722.0, 722.0, 722.0, 1000.0, 722.0, 667.0, 667.0, 667.0, 667.0,
    389.0, 389.0, 389.0, 389.0, 722.0, 722.0, 778.0, 778.0, 778.0, 778.0, 778.0, 570.0, 778.0,
    722.0, 722.0, 722.0, 722.0, 722.0, 611.0, 556.0, 500.0, 500.0, 500.0, 500.0, 500.0, 500.0,
    722.0, 444.0, 444.0, 444.0, 444.0, 444.0, 278.0, 278.0, 278.0, 278.0, 500.0, 556.0, 500.0,
    500.0, 500.0, 500.0, 500.0, 570.0, 500.0, 556.0, 556.0, 556.0, 556.0, 500.0, 556.0, 500.0,
];

pub static HELVETICA_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 278.0, 278.0, 355.0,
    556.0, 556.0, 889.0, 667.0, 191.0, 333.0, 333.0, 389.0, 584.0, 278.0, 333.0, 278.0, 278.0,
    556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 278.0, 278.0, 584.0,
    584.0, 584.0, 556.0, 1015.0, 667.0, 667.0, 722.0, 722.0, 667.0, 611.0, 778.0, 722.0, 278.0,
    500.0, 667.0, 556.0, 833.0, 722.0, 778.0, 667.0, 778.0, 722.0, 667.0, 611.0, 722.0, 667.0,
    944.0, 667.0, 667.0, 611.0, 278.0, 278.0, 278.0, 469.0, 556.0, 333.0, 556.0, 556.0, 500.0,
    556.0, 556.0, 278.0, 556.0, 556.0, 222.0, 222.0, 500.0, 222.0, 833.0, 556.0, 556.0, 556.0,
    556.0, 333.0, 500.0, 278.0, 556.0, 500.0, 722.0, 500.0, 500.0, 500.0, 334.0, 260.0, 334.0,
    584.0, 0.0, 556.0, 500.0, 222.0, 556.0, 333.0, 1000.0, 556.0, 556.0, 333.0, 1000.0, 667.0,
    333.0, 1000.0, 500.0, 611.0, 500.0, 500.0, 222.0, 222.0, 333.0, 333.0, 350.0, 556.0, 1000.0,
    333.0, 1000.0, 500.0, 333.0, 944.0, 500.0, 500.0, 667.0, 278.0, 333.0, 556.0, 556.0, 556.0,
    556.0, 260.0, 556.0, 333.0, 737.0, 370.0, 556.0, 584.0, 333.0, 737.0, 333.0, 400.0, 584.0,
    333.0, 333.0, 333.0, 556.0, 537.0, 278.0, 333.0, 333.0, 365.0, 556.0, 834.0, 834.0, 834.0,
    611.0, 667.0, 667.0, 667.0, 667.0, 667.0, 667.0, 1000.0, 722.0, 667.0, 667.0, 667.0, 667.0,
    278.0, 278.0, 278.0, 278.0, 722.0, 722.0, 778.0, 778.0, 778.0, 778.0, 778.0, 584.0, 778.0,
    722.0, 722.0, 722.0, 722.0, 667.0, 667.0, 611.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0,
    889.0, 500.0, 556.0, 556.0, 556.0, 556.0, 278.0, 278.0, 278.0, 278.0, 556.0, 556.0, 556.0,
    556.0, 556.0, 556.0, 556.0, 584.0, 611.0, 556.0, 556.0, 556.0, 556.0, 500.0, 556.0, 500.0,
];

pub static HELVETICA_BOLD_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 278.0, 333.0, 474.0,
    556.0, 556.0, 889.0, 722.0, 238.0, 333.0, 333.0, 389.0, 584.0, 278.0, 333.0, 278.0, 278.0,
    556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0, 333.0, 333.0, 584.0,
    584.0, 584.0, 611.0, 975.0, 722.0, 722.0, 722.0, 722.0, 667.0, 611.0, 778.0, 722.0, 278.0,
    556.0, 722.0, 611.0, 833.0, 722.0, 778.0, 667.0, 778.0, 722.0, 667.0, 611.0, 722.0, 667.0,
    944.0, 667.0, 667.0, 611.0, 333.0, 278.0, 333.0, 584.0, 556.0, 333.0, 556.0, 611.0, 556.0,
    611.0, 556.0, 333.0, 611.0, 611.0, 278.0, 278.0, 556.0, 278.0, 889.0, 611.0, 611.0, 611.0,
    611.0, 389.0, 556.0, 333.0, 611.0, 556.0, 778.0, 556.0, 556.0, 500.0, 389.0, 280.0, 389.0,
    584.0, 0.0, 556.0, 500.0, 278.0, 556.0, 500.0, 1000.0, 556.0, 556.0, 333.0, 1000.0, 667.0,
    333.0, 1000.0, 500.0, 611.0, 500.0, 500.0, 278.0, 278.0, 500.0, 500.0, 350.0, 556.0, 1000.0,
    333.0, 1000.0, 556.0, 333.0, 944.0, 500.0, 500.0, 667.0, 278.0, 333.0, 556.0, 556.0, 556.0,
    556.0, 280.0, 556.0, 333.0, 737.0, 370.0, 556.0, 584.0, 333.0, 737.0, 333.0, 400.0, 584.0,
    333.0, 333.0, 333.0, 611.0, 556.0, 278.0, 333.0, 333.0, 365.0, 556.0, 834.0, 834.0, 834.0,
    611.0, 722.0, 722.0, 722.0, 722.0, 722.0, 722.0, 1000.0, 722.0, 667.0, 667.0, 667.0, 667.0,
    278.0, 278.0, 278.0, 278.0, 722.0, 722.0, 778.0, 778.0, 778.0, 778.0, 778.0, 584.0, 778.0,
    722.0, 722.0, 722.0, 722.0, 667.0, 667.0, 611.0, 556.0, 556.0, 556.0, 556.0, 556.0, 556.0,
    889.0, 556.0, 556.0, 556.0, 556.0, 556.0, 278.0, 278.0, 278.0, 278.0, 611.0, 611.0, 611.0,
    611.0, 611.0, 611.0, 611.0, 584.0, 611.0, 611.0, 611.0, 611.0, 611.0, 556.0, 611.0, 556.0,
];

pub static COURIER_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 0.0, 600.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 500.0, 600.0, 500.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
];

pub static COURIER_BOLD_WIDTHS: [f32; 256] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 260.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 0.0, 600.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 500.0, 600.0, 500.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 500.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
    600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0,
];
