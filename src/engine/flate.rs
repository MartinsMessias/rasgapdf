use flate2::read::ZlibDecoder;
use std::collections::BTreeMap;
use std::io::Read;
use super::object::PdfObject;

pub const MAX_DECOMPRESSED_BYTES: u64 = 128 * 1024 * 1024; // 128 MB
pub const MAX_FILTER_CHAIN_DEPTH: usize = 8;
pub const MAX_PREDICTOR_COLUMNS: usize = 1_000_000;
pub const MAX_PREDICTOR_COLORS: usize = 32;

/// Decompresses a PDF stream using FlateDecode (zlib), applying predictor filters if requested
pub fn decompress_stream(
    data: &[u8],
    dict: &BTreeMap<String, PdfObject>,
) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let filter = dict.get("Filter");

    match filter {
        Some(PdfObject::Name(name)) if name == "FlateDecode" => {
            let decompressed = decompress_zlib(data)?;
            if let Some(decode_parms) = dict.get("DecodeParms").and_then(|p| p.as_dict()) {
                apply_predictor(&decompressed, decode_parms)
            } else {
                Ok(decompressed)
            }
        }
        Some(PdfObject::Array(filters)) => {
            if filters.len() > MAX_FILTER_CHAIN_DEPTH {
                return Err(format!(
                    "Filter pipeline exceeds maximum depth of {} (found {})",
                    MAX_FILTER_CHAIN_DEPTH,
                    filters.len()
                ));
            }
            // Pipeline of filters (e.g. [/ASCII85Decode /FlateDecode])
            let mut current_data = data.to_vec();
            for f in filters {
                if let Some(name) = f.as_name() {
                    if name == "FlateDecode" {
                        current_data = decompress_zlib(&current_data)?;
                        if (current_data.len() as u64) > MAX_DECOMPRESSED_BYTES {
                            return Err("Decompressed stream in filter pipeline exceeds maximum size".into());
                        }
                    }
                }
            }
            if let Some(decode_parms) = dict.get("DecodeParms").and_then(|p| p.as_dict()) {
                apply_predictor(&current_data, decode_parms)
            } else {
                Ok(current_data)
            }
        }
        None => Ok(data.to_vec()), // Uncompressed stream
        Some(_other) => {
            // Unsupported or raw passthrough (e.g. DCTDecode for JPEG images)
            Ok(data.to_vec())
        }
    }
}

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, String> {
    decompress_zlib_bounded(data, MAX_DECOMPRESSED_BYTES)
}

pub fn decompress_zlib_bounded(data: &[u8], max_bytes: u64) -> Result<Vec<u8>, String> {
    if data.is_empty() || max_bytes == 0 {
        return Ok(Vec::new());
    }
    let decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .take(max_bytes)
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Zlib decompression error: {}", e))?;
    Ok(decompressed)
}

/// Applies PNG/TIFF predictor decoding (ISO 32000-1, Section 7.4.4.4)
fn apply_predictor(
    data: &[u8],
    parms: &BTreeMap<String, PdfObject>,
) -> Result<Vec<u8>, String> {
    let predictor = parms
        .get("Predictor")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    if predictor <= 1 {
        return Ok(data.to_vec()); // No prediction
    }

    if !(10..=15).contains(&predictor) {
        // Non-PNG predictor (e.g. TIFF Predictor 2 or unsupported) passes through safely
        return Ok(data.to_vec());
    }

    if data.is_empty() {
        return Ok(Vec::new());
    }

    // Defensive validation of parameters (ISO 32000-1, Section 7.4.4.4)
    let columns = match parms.get("Columns").and_then(|v| v.as_i64()) {
        Some(c) if c > 0 && (c as usize) <= MAX_PREDICTOR_COLUMNS => c as usize,
        Some(c) if c <= 0 => return Err(format!("Invalid non-positive Columns in DecodeParms: {}", c)),
        Some(c) => return Err(format!("Columns {} exceeds limit {}", c, MAX_PREDICTOR_COLUMNS)),
        None => 1,
    };
    let colors = match parms.get("Colors").and_then(|v| v.as_i64()) {
        Some(c) if c > 0 && (c as usize) <= MAX_PREDICTOR_COLORS => c as usize,
        Some(c) if c <= 0 => return Err(format!("Invalid non-positive Colors in DecodeParms: {}", c)),
        Some(c) => return Err(format!("Colors {} exceeds limit {}", c, MAX_PREDICTOR_COLORS)),
        None => 1,
    };
    let bpc = match parms.get("BitsPerComponent").and_then(|v| v.as_i64()) {
        Some(b) if [1, 2, 4, 8, 16].contains(&b) => b as usize,
        Some(b) => return Err(format!("Invalid BitsPerComponent in DecodeParms: {}", b)),
        None => 8,
    };

    let bits_per_pixel = colors.checked_mul(bpc).ok_or("Integer overflow calculating bits_per_pixel")?;
    let bytes_per_pixel = ((bits_per_pixel + 7) / 8).max(1);

    let row_bits = columns.checked_mul(bits_per_pixel).ok_or("Integer overflow calculating row_bits")?;
    let row_len = (row_bits + 7) / 8;
    if row_len == 0 {
        return Err("Calculated row length is zero in PNG predictor".to_string());
    }

    let stride = row_len.checked_add(1).ok_or("Integer overflow calculating stride")?;

    let num_rows = data.len() / stride;
    if num_rows == 0 {
        return Ok(Vec::new());
    }

    let expected_out_len = num_rows.checked_mul(row_len).ok_or("Integer overflow calculating output size")?;
    if expected_out_len > MAX_DECOMPRESSED_BYTES as usize {
        return Err("Predictor output exceeds maximum allowed decompressed size".to_string());
    }

    let mut out = Vec::with_capacity(expected_out_len.min(data.len()));
    let mut prev_row = vec![0u8; row_len];

    for row_idx in 0..num_rows {
        let row_start = match row_idx.checked_mul(stride) {
            Some(s) => s,
            None => return Err("Row start index overflow".to_string()),
        };
        let row_end = match row_start.checked_add(stride) {
            Some(e) if e <= data.len() => e,
            _ => return Err("Row end exceeds data length".to_string()),
        };

        let filter_type = data[row_start];
        let raw_row = &data[row_start + 1..row_end];
        let mut current_row = vec![0u8; row_len];

        match filter_type {
            0 => {
                // None
                current_row.copy_from_slice(raw_row);
            }
            1 => {
                // Sub: Raw(x) + Recon(x - bpp)
                for i in 0..row_len {
                    let left = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] } else { 0 };
                    current_row[i] = raw_row[i].wrapping_add(left);
                }
            }
            2 => {
                // Up: Raw(x) + Prior(x)
                for i in 0..row_len {
                    let up = prev_row[i];
                    current_row[i] = raw_row[i].wrapping_add(up);
                }
            }
            3 => {
                // Average: Raw(x) + floor((Recon(x - bpp) + Prior(x)) / 2)
                for i in 0..row_len {
                    let left = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] as u16 } else { 0 };
                    let up = prev_row[i] as u16;
                    let avg = ((left + up) / 2) as u8;
                    current_row[i] = raw_row[i].wrapping_add(avg);
                }
            }
            4 => {
                // Paeth
                for i in 0..row_len {
                    let a = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] } else { 0 };
                    let b = prev_row[i];
                    let c = if i >= bytes_per_pixel { prev_row[i - bytes_per_pixel] } else { 0 };
                    let paeth = paeth_predictor(a, b, c);
                    current_row[i] = raw_row[i].wrapping_add(paeth);
                }
            }
            _ => {
                current_row.copy_from_slice(raw_row);
            }
        }

        out.extend_from_slice(&current_row);
        prev_row = current_row;
    }

    Ok(out)
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = (a as i32) + (b as i32) - (c as i32);
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_zlib_roundtrip() {
        let original = b"Hello Pure Rust PDF Parser! Spatial RAG without MuPDF!";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut dict = BTreeMap::new();
        dict.insert("Filter".into(), PdfObject::Name("FlateDecode".into()));

        let decompressed = decompress_stream(&compressed, &dict).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_zlib_decompression_bomb_capped() {
        // Compress 2 MB of zeros (expands massively from a few bytes)
        let uncompressed_size = 2 * 1024 * 1024;
        let original = vec![0u8; uncompressed_size];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        // 1. Verify bounded decompression caps at 64 KB
        let cap_64kb = 64 * 1024;
        let decompressed = decompress_zlib_bounded(&compressed, cap_64kb as u64).unwrap();
        assert_eq!(decompressed.len(), cap_64kb);

        // 2. Verify standard decompress_zlib is bounded by MAX_DECOMPRESSED_BYTES
        let decompressed_full = decompress_zlib(&compressed).unwrap();
        assert_eq!(decompressed_full.len(), uncompressed_size);
        assert!((decompressed_full.len() as u64) <= MAX_DECOMPRESSED_BYTES);
    }

    #[test]
    fn test_predictor_malicious_negative_or_overflow_parameters() {
        let fake_data = vec![0u8; 64];

        // 1. Negative Columns
        let mut parms = BTreeMap::new();
        parms.insert("Predictor".into(), PdfObject::Integer(12));
        parms.insert("Columns".into(), PdfObject::Integer(-1));
        assert!(apply_predictor(&fake_data, &parms).is_err(), "Negative columns must error");

        // 2. Exorbitant Columns (preventing arithmetic overflow)
        let mut parms2 = BTreeMap::new();
        parms2.insert("Predictor".into(), PdfObject::Integer(12));
        parms2.insert("Columns".into(), PdfObject::Integer(i64::MAX));
        assert!(apply_predictor(&fake_data, &parms2).is_err(), "Overflow columns must error");

        // 3. Negative Colors
        let mut parms3 = BTreeMap::new();
        parms3.insert("Predictor".into(), PdfObject::Integer(12));
        parms3.insert("Colors".into(), PdfObject::Integer(-5));
        assert!(apply_predictor(&fake_data, &parms3).is_err(), "Negative colors must error");

        // 4. Invalid BitsPerComponent
        let mut parms4 = BTreeMap::new();
        parms4.insert("Predictor".into(), PdfObject::Integer(12));
        parms4.insert("BitsPerComponent".into(), PdfObject::Integer(99999));
        assert!(apply_predictor(&fake_data, &parms4).is_err(), "Invalid bpc must error");
    }

    #[test]
    fn test_predictor_truncated_and_empty_data() {
        let mut parms = BTreeMap::new();
        parms.insert("Predictor".into(), PdfObject::Integer(12));
        parms.insert("Columns".into(), PdfObject::Integer(10));

        // Empty data
        let res_empty = apply_predictor(&[], &parms).unwrap();
        assert!(res_empty.is_empty());

        // Data shorter than a single stride
        let res_short = apply_predictor(&[0u8; 3], &parms).unwrap();
        assert!(res_short.is_empty());
    }

    #[test]
    fn test_filter_chain_depth_exhaustion_defense() {
        let mut dict = BTreeMap::new();
        let chain: Vec<PdfObject> = (0..20)
            .map(|_| PdfObject::Name("FlateDecode".into()))
            .collect();
        dict.insert("Filter".into(), PdfObject::Array(chain));

        let dummy_data = vec![1, 2, 3];
        let res = decompress_stream(&dummy_data, &dict);
        assert!(res.is_err(), "Exceeding filter pipeline depth must error safely");
    }
}
