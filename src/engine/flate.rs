use flate2::read::ZlibDecoder;
use std::collections::BTreeMap;
use std::io::Read;
use super::object::PdfObject;

/// Decompresses a PDF stream using FlateDecode (zlib), applying predictor filters if requested
pub fn decompress_stream(
    data: &[u8],
    dict: &BTreeMap<String, PdfObject>,
) -> Result<Vec<u8>, String> {
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
            // Pipeline of filters (e.g. [/ASCII85Decode /FlateDecode])
            let mut current_data = data.to_vec();
            for f in filters {
                if let Some(name) = f.as_name() {
                    if name == "FlateDecode" {
                        current_data = decompress_zlib(&current_data)?;
                    }
                }
            }
            Ok(current_data)
        }
        None => Ok(data.to_vec()), // Uncompressed stream
        Some(_other) => {
            // Unsupported or raw passthrough (e.g. DCTDecode for JPEG images)
            Ok(data.to_vec())
        }
    }
}

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
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

    let columns = parms
        .get("Columns")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as usize;
    let colors = parms
        .get("Colors")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as usize;
    let bpc = parms
        .get("BitsPerComponent")
        .and_then(|v| v.as_i64())
        .unwrap_or(8) as usize;

    let bytes_per_pixel = ((colors * bpc) + 7) / 8;
    let bytes_per_pixel = bytes_per_pixel.max(1);
    let row_len = ((columns * colors * bpc) + 7) / 8;

    if (10..=15).contains(&predictor) {
        // PNG Predictors (Each row has 1 prefix filter byte + row_len data bytes)
        let stride = row_len + 1;
        let num_rows = data.len() / stride;
        let mut out = Vec::with_capacity(num_rows * row_len);
        let mut prev_row = vec![0u8; row_len];

        for row_idx in 0..num_rows {
            let row_start = row_idx * stride;
            let filter_type = data[row_start];
            let raw_row = &data[row_start + 1..row_start + stride];
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
    } else {
        Ok(data.to_vec())
    }
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
}
