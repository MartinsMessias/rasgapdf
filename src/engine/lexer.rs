/// PDF Lexer / Tokenizer conforming to ISO 32000-1 (Section 7.2)

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Integer(i64),
    Real(f64),
    LiteralString(Vec<u8>),
    HexString(Vec<u8>),
    Name(String),
    Keyword(String),
    DictOpen,  // <<
    DictClose, // >>
    ArrayOpen, // [
    ArrayClose,// ]
}

pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn with_pos(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos: pos.min(data.len()),
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    pub fn remaining(&self) -> &'a [u8] {
        let safe_pos = self.pos.min(self.data.len());
        &self.data[safe_pos..]
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skips whitespace and PDF comments (% ... \n)
    pub fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if is_whitespace(b) {
                self.pos += 1;
            } else if b == b'%' {
                // Comment runs until next newline (\r or \n)
                self.pos += 1;
                while self.pos < self.data.len() {
                    let cb = self.data[self.pos];
                    self.pos += 1;
                    if cb == b'\r' || cb == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Reads the next token from the stream
    pub fn next_token(&mut self) -> Result<Option<Token>, String> {
        self.skip_whitespace_and_comments();
        if self.is_eof() {
            return Ok(None);
        }

        let b = self.data[self.pos];

        // Delimiters
        if b == b'<' {
            if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                self.pos += 2;
                return Ok(Some(Token::DictOpen));
            } else {
                return self.read_hex_string().map(Some);
            }
        }

        if b == b'>' {
            if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                self.pos += 2;
                return Ok(Some(Token::DictClose));
            } else {
                self.pos += 1;
                return Err(format!("Unexpected '>' at position {}", self.pos - 1));
            }
        }

        if b == b')' {
            self.pos += 1;
            return Err(format!("Unexpected ')' at position {}", self.pos - 1));
        }

        if b == b'{' || b == b'}' {
            self.pos += 1;
            return Ok(Some(Token::Keyword((b as char).to_string())));
        }

        if b == b'[' {
            self.pos += 1;
            return Ok(Some(Token::ArrayOpen));
        }

        if b == b']' {
            self.pos += 1;
            return Ok(Some(Token::ArrayClose));
        }

        if b == b'/' {
            return self.read_name().map(Some);
        }

        if b == b'(' {
            return self.read_literal_string().map(Some);
        }

        // Numbers or keywords
        if is_number_start(b) {
            let (num_tok, is_num) = self.try_read_number();
            if is_num {
                return Ok(Some(num_tok));
            }
        }

        // Regular keyword / identifier
        self.read_keyword().map(Some)
    }

    fn read_name(&mut self) -> Result<Token, String> {
        self.pos += 1; // skip '/'
        let mut name_bytes = Vec::new();

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if is_whitespace(b) || is_delimiter(b) {
                break;
            }

            if b == b'#' && self.pos + 2 < self.data.len() {
                // Hex-encoded character, e.g. #20 -> space
                let d1 = self.data[self.pos + 1];
                let d2 = self.data[self.pos + 2];
                if let (Some(h1), Some(h2)) = (hex_digit(d1), hex_digit(d2)) {
                    name_bytes.push((h1 << 4) | h2);
                    self.pos += 3;
                    continue;
                }
            }

            name_bytes.push(b);
            self.pos += 1;
        }

        let name_str = String::from_utf8_lossy(&name_bytes).to_string();
        Ok(Token::Name(name_str))
    }

    fn read_literal_string(&mut self) -> Result<Token, String> {
        self.pos += 1; // skip '('
        let mut result = Vec::new();
        let mut depth = 1;

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;

            if b == b'(' {
                depth += 1;
                result.push(b);
            } else if b == b')' {
                depth -= 1;
                if depth == 0 {
                    return Ok(Token::LiteralString(result));
                }
                result.push(b);
            } else if b == b'\\' {
                if self.pos >= self.data.len() {
                    break;
                }
                let esc = self.data[self.pos];
                self.pos += 1;
                match esc {
                    b'n' => result.push(b'\n'),
                    b'r' => result.push(b'\r'),
                    b't' => result.push(b'\t'),
                    b'b' => result.push(0x08),
                    b'f' => result.push(0x0C),
                    b'(' => result.push(b'('),
                    b')' => result.push(b')'),
                    b'\\' => result.push(b'\\'),
                    b'\r' => {
                        // Line continuation: \r\n or \r
                        if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                            self.pos += 1;
                        }
                    }
                    b'\n' => {
                        // Line continuation
                    }
                    b'0'..=b'7' => {
                        // Octal escape sequence (up to 3 octal digits)
                        let mut oct_val = (esc - b'0') as u32;
                        let mut count = 1;
                        while count < 3 && self.pos < self.data.len() {
                            let ob = self.data[self.pos];
                            if (b'0'..=b'7').contains(&ob) {
                                oct_val = (oct_val << 3) + (ob - b'0') as u32;
                                self.pos += 1;
                                count += 1;
                            } else {
                                break;
                            }
                        }
                        result.push((oct_val & 0xFF) as u8);
                    }
                    _ => {
                        result.push(esc);
                    }
                }
            } else {
                result.push(b);
            }
        }

        Err("Unterminated literal string in PDF".to_string())
    }

    fn read_hex_string(&mut self) -> Result<Token, String> {
        self.pos += 1; // skip '<'
        let mut hex_digits = Vec::new();

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;

            if b == b'>' {
                break;
            }
            if is_whitespace(b) {
                continue;
            }
            if let Some(digit) = hex_digit(b) {
                hex_digits.push(digit);
            }
        }

        // If odd number of hex digits, append '0' per PDF standard
        if hex_digits.len() % 2 != 0 {
            hex_digits.push(0);
        }

        let mut bytes = Vec::with_capacity(hex_digits.len() / 2);
        for chunk in hex_digits.chunks_exact(2) {
            bytes.push((chunk[0] << 4) | chunk[1]);
        }

        Ok(Token::HexString(bytes))
    }

    fn try_read_number(&mut self) -> (Token, bool) {
        let start = self.pos;
        let mut is_real = false;

        // Optional leading sign
        if self.pos < self.data.len() && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-') {
            self.pos += 1;
        }

        let mut has_digits = false;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b.is_ascii_digit() {
                has_digits = true;
                self.pos += 1;
            } else if b == b'.' && !is_real {
                is_real = true;
                self.pos += 1;
            } else {
                break;
            }
        }

        if has_digits {
            let slice = &self.data[start..self.pos];
            if let Ok(s) = std::str::from_utf8(slice) {
                if is_real {
                    if let Ok(f) = s.parse::<f64>() {
                        if f.is_finite() {
                            return (Token::Real(f), true);
                        }
                    }
                } else if let Ok(i) = s.parse::<i64>() {
                    return (Token::Integer(i), true);
                } else if let Ok(f) = s.parse::<f64>() {
                    // Integer overflowed i64, parse as real if finite
                    if f.is_finite() {
                        return (Token::Real(f), true);
                    }
                }
            }
        }

        // Reset if failed
        self.pos = start;
        (Token::Keyword(String::new()), false)
    }

    fn read_keyword(&mut self) -> Result<Token, String> {
        let start = self.pos;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if is_whitespace(b) || is_delimiter(b) {
                break;
            }
            self.pos += 1;
        }

        if start == self.pos {
            // Defensively ensure progress if called on an unexpected delimiter or byte
            if self.pos < self.data.len() {
                let bad = self.data[self.pos];
                self.pos += 1;
                return Err(format!("Unexpected byte 0x{:02X} at position {}", bad, start));
            }
            return Ok(Token::Keyword(String::new()));
        }

        let kw_slice = &self.data[start..self.pos];
        let kw_str = String::from_utf8_lossy(kw_slice).to_string();
        Ok(Token::Keyword(kw_str))
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

fn is_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn is_number_start(b: u8) -> bool {
    b.is_ascii_digit() || b == b'+' || b == b'-' || b == b'.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_numbers() {
        let data = b" 123 -456 +78 3.1415 -.002 ";
        let mut lexer = Lexer::new(data);
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(123)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(-456)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(78)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(3.1415)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(-0.002)));
        assert_eq!(lexer.next_token().unwrap(), None);
    }

    #[test]
    fn test_lex_names() {
        let data = b"/Type /Font#20Name /Page/Kids";
        let mut lexer = Lexer::new(data);
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Type".into())));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Font Name".into())));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Page".into())));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Kids".into())));
        assert_eq!(lexer.next_token().unwrap(), None);
    }

    #[test]
    fn test_lex_strings() {
        let data = b"(Hello (nested) World\\n) <48656c6c6f> <41424>";
        let mut lexer = Lexer::new(data);
        assert_eq!(
            lexer.next_token().unwrap(),
            Some(Token::LiteralString(b"Hello (nested) World\n".to_vec()))
        );
        assert_eq!(
            lexer.next_token().unwrap(),
            Some(Token::HexString(b"Hello".to_vec()))
        );
        // Odd length 41424 -> 414240 -> 'A', 'B', '@' (0x40)
        assert_eq!(
            lexer.next_token().unwrap(),
            Some(Token::HexString(vec![0x41, 0x42, 0x40]))
        );
        assert_eq!(lexer.next_token().unwrap(), None);
    }

    #[test]
    fn test_lex_delimiters_and_comments() {
        let data = b"% header comment\n<< /Key [ 1 2 (str) ] >> % end comment\n";
        let mut lexer = Lexer::new(data);
        assert_eq!(lexer.next_token().unwrap(), Some(Token::DictOpen));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Name("Key".into())));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayOpen));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(1)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Integer(2)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::LiteralString(b"str".to_vec())));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::ArrayClose));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::DictClose));
        assert_eq!(lexer.next_token().unwrap(), None);
    }

    #[test]
    fn test_lexer_unexpected_delimiter_does_not_infinite_loop() {
        let data = b") } { >";
        let mut lexer = Lexer::new(data);
        // Each call should either error or return keyword, but NEVER loop infinitely
        let mut steps = 0;
        while lexer.pos() < data.len() && steps < 100 {
            let _ = lexer.next_token();
            steps += 1;
        }
        assert!(lexer.is_eof());
    }

    #[test]
    fn test_lexer_out_of_bounds_pos_and_remaining() {
        let data = b"abc";
        let lexer = Lexer::with_pos(data, 100);
        assert_eq!(lexer.pos(), 3);
        assert!(lexer.remaining().is_empty());
        assert!(lexer.is_eof());
    }

    #[test]
    fn test_lexer_truncated_and_octal_strings() {
        // Truncated literal string with backslash at EOF
        let mut lex1 = Lexer::new(b"(abc\\");
        assert!(lex1.next_token().is_err());

        // Octal escape sequence overflow mod 256
        let mut lex2 = Lexer::new(b"(\\777)");
        let tok = lex2.next_token().unwrap();
        // 0o777 = 511, 511 & 0xFF = 255
        assert_eq!(tok, Some(Token::LiteralString(vec![255])));

        // Truncated hex string without '>'
        let mut lex3 = Lexer::new(b"<4142");
        assert_eq!(lex3.next_token().unwrap(), Some(Token::HexString(vec![0x41, 0x42])));
    }

    #[test]
    fn test_lexer_name_with_invalid_hex() {
        let mut lex = Lexer::new(b"/Name#ZZ/#/A#20B");
        assert_eq!(lex.next_token().unwrap(), Some(Token::Name("Name#ZZ".into())));
        assert_eq!(lex.next_token().unwrap(), Some(Token::Name("#".into())));
        assert_eq!(lex.next_token().unwrap(), Some(Token::Name("A B".into())));
    }

    #[test]
    fn test_lexer_integer_overflow_to_real() {
        let mut lex = Lexer::new(b"123456789012345678901234567890");
        let tok = lex.next_token().unwrap();
        match tok {
            Some(Token::Real(f)) => assert!(f > 1e20),
            _ => panic!("Expected Real for overflowing integer, got {:?}", tok),
        }
    }
}
