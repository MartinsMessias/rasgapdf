use super::lexer::{Lexer, Token};
use super::object::{ObjectId, PdfObject, PdfStream};
use std::collections::BTreeMap;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    full_data: &'a [u8],
    depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(data),
            full_data: data,
            depth: 0,
        }
    }

    pub fn with_pos(data: &'a [u8], pos: usize) -> Self {
        Self {
            lexer: Lexer::with_pos(data, pos),
            full_data: data,
            depth: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.lexer.pos()
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.lexer.set_pos(pos);
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, String> {
        self.lexer.next_token()
    }

    /// Parses the next PDF object from the stream
    pub fn parse_object(&mut self) -> Result<Option<PdfObject>, String> {
        if self.depth > 512 {
            return Err("Exceeded maximum nesting depth in PDF parser".to_string());
        }
        let tok = match self.lexer.next_token()? {
            Some(t) => t,
            None => return Ok(None),
        };

        self.depth += 1;
        let res = self.parse_object_from_token(tok);
        self.depth -= 1;
        res
    }

    fn parse_object_from_token(&mut self, tok: Token) -> Result<Option<PdfObject>, String> {
        match tok {
            Token::Integer(i) => {
                // Check if this integer is the start of an indirect reference: num gen R
                let saved_pos = self.lexer.pos();
                if let Ok(Some(Token::Integer(gen))) = self.lexer.next_token() {
                    let _after_gen_pos = self.lexer.pos();
                    if let Ok(Some(Token::Keyword(kw))) = self.lexer.next_token() {
                        if kw == "R" && i >= 0 && i <= u32::MAX as i64 && gen >= 0 && gen <= u16::MAX as i64 {
                            return Ok(Some(PdfObject::Reference(ObjectId::new(i as u32, gen as u16))));
                        }
                    }
                    // Not an 'R', rollback to after first integer
                    self.lexer.set_pos(saved_pos);
                } else {
                    self.lexer.set_pos(saved_pos);
                }
                Ok(Some(PdfObject::Integer(i)))
            }
            Token::Real(f) => Ok(Some(PdfObject::Real(f))),
            Token::LiteralString(bytes) => Ok(Some(PdfObject::String(bytes))),
            Token::HexString(bytes) => Ok(Some(PdfObject::String(bytes))),
            Token::Name(name) => Ok(Some(PdfObject::Name(name))),
            Token::Keyword(kw) => match kw.as_str() {
                "true" => Ok(Some(PdfObject::Boolean(true))),
                "false" => Ok(Some(PdfObject::Boolean(false))),
                "null" => Ok(Some(PdfObject::Null)),
                _ => Err(format!("Unexpected keyword: {}", kw)),
            },
            Token::ArrayOpen => self.parse_array().map(Some),
            Token::DictOpen => {
                let dict = self.parse_dict()?;
                // Check if this dictionary is immediately followed by a stream
                let saved_pos = self.lexer.pos();
                if let Ok(Some(Token::Keyword(kw))) = self.lexer.next_token() {
                    if kw == "stream" {
                        let stream_data = self.read_stream_content(&dict)?;
                        return Ok(Some(PdfObject::Stream(PdfStream {
                            dict,
                            data: stream_data,
                        })));
                    }
                }
                self.lexer.set_pos(saved_pos);
                Ok(Some(PdfObject::Dictionary(dict)))
            }
            Token::ArrayClose => Err("Unexpected array close ']'".to_string()),
            Token::DictClose => Err("Unexpected dictionary close '>>'".to_string()),
        }
    }

    fn parse_array(&mut self) -> Result<PdfObject, String> {
        if self.depth > 512 {
            return Err("Exceeded maximum nesting depth in PDF array".to_string());
        }
        let mut arr = Vec::new();
        loop {
            self.lexer.skip_whitespace_and_comments();
            if self.lexer.is_eof() {
                return Err("Unterminated array in PDF".to_string());
            }

            let _saved_pos = self.lexer.pos();
            let tok = match self.lexer.next_token()? {
                Some(t) => t,
                None => return Err("Unterminated array in PDF".to_string()),
            };

            if tok == Token::ArrayClose {
                break;
            }

            self.depth += 1;
            let obj = self.parse_object_from_token(tok);
            self.depth -= 1;

            match obj? {
                Some(o) => arr.push(o),
                None => return Err("Expected object in array".to_string()),
            }
        }
        Ok(PdfObject::Array(arr))
    }

    fn parse_dict(&mut self) -> Result<BTreeMap<String, PdfObject>, String> {
        if self.depth > 512 {
            return Err("Exceeded maximum nesting depth in PDF dictionary".to_string());
        }
        let mut dict = BTreeMap::new();
        loop {
            self.lexer.skip_whitespace_and_comments();
            if self.lexer.is_eof() {
                return Err("Unterminated dictionary in PDF".to_string());
            }

            let tok = match self.lexer.next_token()? {
                Some(t) => t,
                None => return Err("Unterminated dictionary in PDF".to_string()),
            };

            if tok == Token::DictClose {
                break;
            }

            let key = match tok {
                Token::Name(n) => n,
                _ => return Err(format!("Expected name key in dictionary, found {:?}", tok)),
            };

            let val = match self.parse_object()? {
                Some(v) => v,
                None => return Err(format!("Expected value for key '/{}' in dictionary", key)),
            };

            dict.insert(key, val);
        }
        Ok(dict)
    }

    fn read_stream_content(&mut self, dict: &BTreeMap<String, PdfObject>) -> Result<Vec<u8>, String> {
        // Stream keyword must be followed by newline (\r\n or \n)
        let mut pos = self.lexer.pos().min(self.full_data.len());
        if pos < self.full_data.len() && self.full_data[pos] == b'\r' {
            pos += 1;
        }
        if pos < self.full_data.len() && self.full_data[pos] == b'\n' {
            pos += 1;
        }

        // If /Length is explicitly specified as non-negative integer, read exact bytes
        if let Some(PdfObject::Integer(len)) = dict.get("Length") {
            if *len >= 0 {
                let stream_len = *len as usize;
                if let Some(end_pos) = pos.checked_add(stream_len) {
                    if end_pos <= self.full_data.len() {
                        let stream_bytes = self.full_data[pos..end_pos].to_vec();
                        self.lexer.set_pos(end_pos);
                        // Skip following whitespace / endstream keyword
                        self.lexer.skip_whitespace_and_comments();
                        if let Ok(Some(Token::Keyword(kw))) = self.lexer.next_token() {
                            if kw == "endstream" {
                                return Ok(stream_bytes);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: search for "endstream" marker
        let remaining = &self.full_data[pos..];
        if let Some(end_offset) = find_subsequence(remaining, b"endstream") {
            // Trim trailing \r\n before endstream
            let mut end = pos + end_offset;
            if end > pos && (self.full_data[end - 1] == b'\n' || self.full_data[end - 1] == b'\r') {
                end -= 1;
                if end > pos && self.full_data[end - 1] == b'\r' {
                    end -= 1;
                }
            }
            let stream_bytes = self.full_data[pos..end].to_vec();
            self.lexer.set_pos((pos + end_offset + 9).min(self.full_data.len())); // 9 = "endstream".len()
            return Ok(stream_bytes);
        }

        Err("Failed to locate 'endstream' in stream object".to_string())
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primitives() {
        let mut p = Parser::new(b"true false null 100 -50 3.14 (hello) /Name 10 0 R");
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Boolean(true)));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Boolean(false)));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Null));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Integer(100)));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Integer(-50)));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Real(3.14)));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::String(b"hello".to_vec())));
        assert_eq!(p.parse_object().unwrap(), Some(PdfObject::Name("Name".into())));
        assert_eq!(
            p.parse_object().unwrap(),
            Some(PdfObject::Reference(ObjectId::new(10, 0)))
        );
        assert_eq!(p.parse_object().unwrap(), None);
    }

    #[test]
    fn test_parse_nested_dict_and_array() {
        let data = b"<< /Type /Pages /Kids [ 3 0 R 4 0 R ] /Count 2 >>";
        let mut p = Parser::new(data);
        let obj = p.parse_object().unwrap().unwrap();
        assert_eq!(obj.dict_get("Type").unwrap().as_name(), Some("Pages"));
        assert_eq!(obj.dict_get("Count").unwrap().as_i64(), Some(2));
        let kids = obj.dict_get("Kids").unwrap().as_array().unwrap();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].as_reference(), Some(ObjectId::new(3, 0)));
        assert_eq!(kids[1].as_reference(), Some(ObjectId::new(4, 0)));
    }

    #[test]
    fn test_parse_stream() {
        let data = b"<< /Length 11 >> stream\r\nHello World\nendstream";
        let mut p = Parser::new(data);
        let obj = p.parse_object().unwrap().unwrap();
        let stream = obj.as_stream().unwrap();
        assert_eq!(stream.data, b"Hello World");
    }

    #[test]
    fn test_parser_negative_length_stream_does_not_panic() {
        let data = b"<< /Length -10 >> stream\r\nHello World\nendstream";
        let mut p = Parser::new(data);
        let obj = p.parse_object().unwrap().unwrap();
        let stream = obj.as_stream().unwrap();
        // Should fallback to scanning for endstream instead of overflowing
        assert_eq!(stream.data, b"Hello World");
    }

    #[test]
    fn test_parser_negative_indirect_reference() {
        let data = b"-5 0 R";
        let mut p = Parser::new(data);
        let obj = p.parse_object().unwrap().unwrap();
        // Negative number should not be parsed as reference
        assert_eq!(obj, PdfObject::Integer(-5));
    }

    #[test]
    fn test_parser_deeply_nested_structures_capped() {
        let mut deeply_nested = Vec::new();
        for _ in 0..600 {
            deeply_nested.extend_from_slice(b"[ ");
        }
        for _ in 0..600 {
            deeply_nested.extend_from_slice(b"] ");
        }
        let mut p = Parser::new(&deeply_nested);
        assert!(p.parse_object().is_err());
    }
}
