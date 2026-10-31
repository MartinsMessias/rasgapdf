use super::flate::decompress_stream;
use super::lexer::{Lexer, Token};
use super::object::{ObjectId, PdfObject};
use super::parser::Parser;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub enum XRefEntry {
    /// Object at byte offset in the PDF file
    Uncompressed { offset: usize, generation: u16 },
    /// Object compressed inside an Object Stream (/Type /ObjStm)
    Compressed { stream_num: u32, index: u16 },
}

pub struct XRefTable {
    pub entries: HashMap<u32, XRefEntry>,
    pub trailer: BTreeMap<String, PdfObject>,
}

impl XRefTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            trailer: BTreeMap::new(),
        }
    }

    /// Parses the entire cross-reference table and trailer starting from the end of the file
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let mut table = XRefTable::new();
        let startxref_offset = find_startxref(data)?;
        let mut current_offset = Some(startxref_offset);
        let mut visited_offsets = std::collections::HashSet::new();

        while let Some(offset) = current_offset {
            if offset >= data.len() || !visited_offsets.insert(offset) {
                // Break out on out-of-bounds or cyclic /Prev reference
                break;
            }

            let mut parser = Parser::with_pos(data, offset);
            let first_tok = parser.next_token()?;

            match first_tok {
                Some(Token::Keyword(kw)) if kw == "xref" => {
                    // Classic ASCII xref table
                    read_ascii_xref_subsections(&mut parser, &mut table)?;
                    // Read trailer
                    if let Ok(Some(Token::Keyword(tr_kw))) = parser.next_token() {
                        if tr_kw == "trailer" {
                            if let Ok(Some(PdfObject::Dictionary(tr_dict))) = parser.parse_object() {
                                // Chain previous xref table if /Prev exists
                                let prev = tr_dict
                                    .get("Prev")
                                    .and_then(|p| p.as_i64())
                                    .filter(|&i| i >= 0 && (i as usize) < data.len())
                                    .map(|i| i as usize);
                                for (k, v) in tr_dict {
                                    table.trailer.entry(k).or_insert(v);
                                }
                                current_offset = prev;
                                continue;
                            }
                        }
                    }
                    break;
                }
                Some(Token::Integer(_obj_num)) => {
                    // PDF 1.5+ Cross-Reference Stream: obj_num gen obj << /Type /XRef ... >> stream
                    let _gen_num = match parser.next_token()? {
                        Some(Token::Integer(g)) if g >= 0 => g as u16,
                        _ => return Err("Expected generation number in XRef stream".into()),
                    };
                    match parser.next_token()? {
                        Some(Token::Keyword(ref k)) if k == "obj" => {}
                        _ => return Err("Expected 'obj' in XRef stream".into()),
                    };

                    let xref_stream_obj = match parser.parse_object()? {
                        Some(PdfObject::Stream(s)) => s,
                        _ => return Err("Expected stream for XRef stream object".into()),
                    };

                    let decompressed = decompress_stream(&xref_stream_obj.data, &xref_stream_obj.dict)?;
                    read_binary_xref_stream(&decompressed, &xref_stream_obj.dict, &mut table)?;

                    let prev = xref_stream_obj
                        .dict
                        .get("Prev")
                        .and_then(|p| p.as_i64())
                        .filter(|&i| i >= 0 && (i as usize) < data.len())
                        .map(|i| i as usize);
                    for (k, v) in xref_stream_obj.dict {
                        table.trailer.entry(k).or_insert(v);
                    }
                    current_offset = prev;
                }
                _ => return Err(format!("Unexpected token at startxref offset {}", offset)),
            }
        }

        Ok(table)
    }

    /// Dereferences an indirect object from the PDF file or object streams
    pub fn get_object<'a>(
        &self,
        data: &'a [u8],
        id: ObjectId,
        obj_stream_cache: &mut HashMap<u32, BTreeMap<u32, PdfObject>>,
    ) -> Result<PdfObject, String> {
        let entry = self
            .entries
            .get(&id.number)
            .ok_or_else(|| format!("Object {} not found in XRef table", id))?;

        match entry {
            XRefEntry::Uncompressed { offset, .. } => {
                if *offset >= data.len() {
                    return Err(format!("Offset {} beyond EOF for object {}", offset, id));
                }
                let mut parser = Parser::with_pos(data, *offset);
                // Expect: num gen obj
                let obj_num = parser.next_token()?;
                let gen_num = parser.next_token()?;
                let obj_kw = parser.next_token()?;

                match (obj_num, gen_num, obj_kw) {
                    (Some(Token::Integer(n)), Some(Token::Integer(_)), Some(Token::Keyword(ref k)))
                        if n == id.number as i64 && k == "obj" =>
                    {
                        parser
                            .parse_object()?
                            .ok_or_else(|| format!("Failed to parse object {}", id))
                    }
                    _ => Err(format!("Corrupt object header for {} at offset {}", id, offset)),
                }
            }
            XRefEntry::Compressed { stream_num, index: _ } => {
                if *stream_num == id.number {
                    return Err(format!("Cyclic object stream reference for object {}", id));
                }
                // Disallow nested object streams per ISO 32000-1 section 7.5.7
                if let Some(XRefEntry::Compressed { .. }) = self.entries.get(stream_num) {
                    return Err(format!("Nested object stream detected for stream {}", stream_num));
                }

                if !obj_stream_cache.contains_key(stream_num) {
                    let stream_obj = self.get_object(data, ObjectId::new(*stream_num, 0), obj_stream_cache)?;
                    let parsed_objects = parse_object_stream(&stream_obj)?;
                    obj_stream_cache.insert(*stream_num, parsed_objects);
                }

                let stream_objects = obj_stream_cache.get(stream_num).unwrap();
                stream_objects
                    .get(&id.number)
                    .cloned()
                    .ok_or_else(|| format!("Object {} not found in Object Stream {}", id, stream_num))
            }
        }
    }
}

/// Locates the `startxref` keyword in the last 4096 bytes of the PDF
fn find_startxref(data: &[u8]) -> Result<usize, String> {
    if data.is_empty() {
        return Err("Cannot find startxref in empty data".into());
    }
    let scan_len = data.len().min(4096);
    let start = data.len() - scan_len;
    let window = &data[start..];

    let pattern = b"startxref";
    if let Some(pos) = window.windows(pattern.len()).rposition(|w| w == pattern) {
        let abs_pos = start + pos + pattern.len();
        let mut lexer = Lexer::with_pos(data, abs_pos);
        if let Ok(Some(Token::Integer(offset))) = lexer.next_token() {
            if offset >= 0 && (offset as usize) < data.len() {
                return Ok(offset as usize);
            }
        }
    }

    Err("Could not find 'startxref' in PDF trailer".into())
}

/// Reads classic ASCII xref subsections
fn read_ascii_xref_subsections(parser: &mut Parser, table: &mut XRefTable) -> Result<(), String> {
    loop {
        let saved_pos = parser.pos();
        let tok1 = match parser.next_token()? {
            Some(t) => t,
            None => break,
        };

        if let Token::Keyword(ref kw) = tok1 {
            if kw == "trailer" {
                parser.set_pos(saved_pos);
                break;
            }
        }

        let first_id = match tok1 {
            Token::Integer(i) if i >= 0 && i <= u32::MAX as i64 => i as u32,
            _ => {
                parser.set_pos(saved_pos);
                break;
            }
        };

        let count = match parser.next_token()? {
            Some(Token::Integer(c)) if c >= 0 && c <= u32::MAX as i64 => c as u32,
            _ => return Err("Expected count in xref subsection".into()),
        };

        for i in 0..count {
            let offset_tok = parser.next_token()?;
            if offset_tok.is_none() {
                break;
            }
            let gen_tok = parser.next_token()?;
            let type_tok = parser.next_token()?;

            if let (Some(Token::Integer(offset)), Some(Token::Integer(gen)), Some(Token::Keyword(typ))) =
                (offset_tok, gen_tok, type_tok)
            {
                if offset < 0 || gen < 0 || gen > u16::MAX as i64 {
                    continue;
                }
                if let Some(obj_id) = first_id.checked_add(i) {
                    if typ == "n" {
                        table.entries.entry(obj_id).or_insert(
                            XRefEntry::Uncompressed {
                                offset: offset as usize,
                                generation: gen as u16,
                            },
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Reads PDF 1.5+ compressed binary XRef stream
fn read_binary_xref_stream(
    stream_data: &[u8],
    dict: &BTreeMap<String, PdfObject>,
    table: &mut XRefTable,
) -> Result<(), String> {
    let w_array = dict
        .get("W")
        .and_then(|w| w.as_array())
        .ok_or("Missing /W in XRef stream")?;
    if w_array.len() < 3 {
        return Err("Invalid /W array length in XRef stream".into());
    }

    let w0 = match w_array[0].as_i64() {
        Some(w) if w >= 0 => w as usize,
        _ => return Err("Invalid W[0] in XRef stream".into()),
    };
    let w1 = match w_array[1].as_i64() {
        Some(w) if w >= 0 => w as usize,
        _ => return Err("Invalid W[1] in XRef stream".into()),
    };
    let w2 = match w_array[2].as_i64() {
        Some(w) if w >= 0 => w as usize,
        _ => return Err("Invalid W[2] in XRef stream".into()),
    };
    let entry_size = w0
        .checked_add(w1)
        .and_then(|s| s.checked_add(w2))
        .ok_or("Entry size overflow in XRef stream")?;
    if entry_size == 0 {
        return Err("XRef stream entry size is 0".into());
    }

    let size = dict.get("Size").and_then(|s| s.as_i64()).unwrap_or(0) as u32;

    // Index array: pairs of [first_id count first_id count ...]
    let index_ranges: Vec<(u32, u32)> = if let Some(index_arr) = dict.get("Index").and_then(|i| i.as_array()) {
        let mut ranges = Vec::new();
        for chunk in index_arr.chunks(2) {
            if chunk.len() == 2 {
                let first = chunk[0].as_i64().unwrap_or(0) as u32;
                let count = chunk[1].as_i64().unwrap_or(0) as u32;
                ranges.push((first, count));
            }
        }
        ranges
    } else {
        vec![(0, size)]
    };

    let mut stream_pos = 0;
    for (first_id, count) in index_ranges {
        let max_possible = (stream_data.len().saturating_sub(stream_pos)) / entry_size;
        let safe_count = (count as usize).min(max_possible) as u32;

        for i in 0..safe_count {
            if stream_pos + entry_size > stream_data.len() {
                break;
            }

            let entry_bytes = &stream_data[stream_pos..stream_pos + entry_size];
            stream_pos += entry_size;

            let entry_type = if w0 > 0 { read_int(&entry_bytes[0..w0]) } else { 1 };
            let f1 = read_int(&entry_bytes[w0..w0 + w1]);
            let f2 = read_int(&entry_bytes[w0 + w1..entry_size]);

            if let Some(obj_id) = first_id.checked_add(i) {
                match entry_type {
                    1 => {
                        // Type 1: uncompressed object at file byte offset
                        table.entries.entry(obj_id).or_insert(
                            XRefEntry::Uncompressed {
                                offset: f1 as usize,
                                generation: f2 as u16,
                            },
                        );
                    }
                    2 => {
                        // Type 2: object in object stream (f1 = stream obj num, f2 = index)
                        table.entries.entry(obj_id).or_insert(
                            XRefEntry::Compressed {
                                stream_num: f1 as u32,
                                index: f2 as u16,
                            },
                        );
                    }
                    _ => {} // Type 0: free object
                }
            }
        }
    }

    Ok(())
}

fn read_int(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes.iter().take(8) {
        val = (val << 8) | (b as u64);
    }
    val
}

/// Parses an Object Stream (/Type /ObjStm)
fn parse_object_stream(stream_obj: &PdfObject) -> Result<BTreeMap<u32, PdfObject>, String> {
    let stream = stream_obj.as_stream().ok_or("Expected stream for ObjStm")?;
    let decompressed = decompress_stream(&stream.data, &stream.dict)?;

    let raw_n = stream.dict.get("N").and_then(|n| n.as_i64()).unwrap_or(0);
    let raw_first = stream.dict.get("First").and_then(|f| f.as_i64()).unwrap_or(0);
    if raw_n <= 0 || raw_first < 0 {
        return Ok(BTreeMap::new());
    }
    let num_objects = raw_n as usize;
    let first_offset = raw_first as usize;

    let mut parser = Parser::new(&decompressed);
    let safe_capacity = num_objects.min(decompressed.len() / 2).min(100_000);
    let mut index_entries = Vec::with_capacity(safe_capacity);

    for _ in 0..num_objects {
        let obj_num = match parser.next_token()? {
            Some(Token::Integer(n)) if n >= 0 => n as u32,
            _ => return Err("Failed to read object number in ObjStm index".into()),
        };
        let rel_offset = match parser.next_token()? {
            Some(Token::Integer(o)) if o >= 0 => o as usize,
            _ => return Err("Failed to read relative offset in ObjStm index".into()),
        };
        index_entries.push((obj_num, rel_offset));
    }

    let mut result = BTreeMap::new();
    for (obj_num, rel_offset) in index_entries {
        if let Some(abs_pos) = first_offset.checked_add(rel_offset) {
            if abs_pos < decompressed.len() {
                let mut obj_parser = Parser::with_pos(&decompressed, abs_pos);
                if let Some(obj) = obj_parser.parse_object()? {
                    result.insert(obj_num, obj);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_startxref_and_parse_ascii_xref() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Count 0 >>\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
                o1, o2, xref_pos
            )
            .as_bytes(),
        );

        let table = XRefTable::parse(&pdf).unwrap();
        assert_eq!(table.entries.len(), 2);
        assert_eq!(table.trailer.get("Size").unwrap().as_i64(), Some(3));
        assert_eq!(
            table.trailer.get("Root").unwrap().as_reference(),
            Some(ObjectId::new(1, 0))
        );

        let mut cache = HashMap::new();
        let cat = table.get_object(&pdf, ObjectId::new(1, 0), &mut cache).unwrap();
        assert_eq!(cat.dict_get("Type").unwrap().as_name(), Some("Catalog"));
    }

    #[test]
    fn test_xref_cyclic_prev_does_not_infinite_loop() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 /Prev {} >>\nstartxref\n{}\n%%EOF",
                xref_pos, xref_pos
            )
            .as_bytes(),
        );

        // Must terminate quickly rather than hanging in infinite loop
        let table = XRefTable::parse(&pdf);
        assert!(table.is_ok());
    }

    #[test]
    fn test_xref_corrupt_negative_prev() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 /Prev -500 >>\nstartxref\n{}\n%%EOF",
                xref_pos
            )
            .as_bytes(),
        );

        let table = XRefTable::parse(&pdf);
        assert!(table.is_ok());
    }

    #[test]
    fn test_xref_negative_w_array_rejected() {
        let mut dict = BTreeMap::new();
        let w_arr = vec![
            PdfObject::Integer(-1),
            PdfObject::Integer(2),
            PdfObject::Integer(1),
        ];
        dict.insert("W".into(), PdfObject::Array(w_arr));
        let mut table = XRefTable::new();
        let res = read_binary_xref_stream(&[0u8; 16], &dict, &mut table);
        assert!(res.is_err());
    }

    #[test]
    fn test_xref_self_referential_objstm_prevented() {
        let mut table = XRefTable::new();
        table.entries.insert(
            5,
            XRefEntry::Compressed {
                stream_num: 5,
                index: 0,
            },
        );
        let mut cache = HashMap::new();
        let res = table.get_object(b"", ObjectId::new(5, 0), &mut cache);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Cyclic object stream"));
    }
}
