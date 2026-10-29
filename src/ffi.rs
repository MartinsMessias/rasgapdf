use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use crate::types::ParserOptions;
use crate::spatial::{extract_text_bytes, parse_file, parse_raw_bytes, parse_raw_file};

/// Processa um arquivo PDF e retorna o JSON espacial estruturado como string C.
/// O chamador deve liberar a memória chamando `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_file(path_ptr: *const c_char) -> *mut c_char {
    rasgapdf_parse_file_options(path_ptr, std::ptr::null())
}

/// Modo 1: Retorna exclusivamente a lista plana de coordenadas brutas X/Y
#[no_mangle]
pub extern "C" fn rasgapdf_parse_raw(path_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() {
        return create_error_json("Path pointer is null");
    }

    let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => return create_error_json("Invalid UTF-8 in path"),
    };

    match parse_raw_file(path_str) {
        Ok(doc) => match serde_json::to_string(&doc) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => create_error_json("Failed to create CString"),
            },
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        },
        Err(e) => create_error_json(&format!("Parse error: {}", e)),
    }
}

/// Processa um PDF com opções JSON customizadas
#[no_mangle]
pub extern "C" fn rasgapdf_parse_file_options(path_ptr: *const c_char, options_json_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() {
        return create_error_json("Path pointer is null");
    }

    let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => return create_error_json("Invalid UTF-8 in path"),
    };

    let options: ParserOptions = if !options_json_ptr.is_null() {
        match unsafe { CStr::from_ptr(options_json_ptr) }.to_str() {
            Ok(opts_str) => serde_json::from_str(opts_str).unwrap_or_default(),
            Err(_) => ParserOptions::default(),
        }
    } else {
        ParserOptions::default()
    };

    if options.mode == "raw" {
        return rasgapdf_parse_raw(path_ptr);
    }

    match parse_file(path_str, &options) {
        Ok(doc) => match serde_json::to_string(&doc) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => create_error_json("Failed to create CString"),
            },
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        },
        Err(e) => create_error_json(&format!("Parse error: {}", e)),
    }
}

/// Libera a string alocada pelo Rust
#[no_mangle]
pub extern "C" fn rasgapdf_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Extrai as imagens embutidas e grava os streams originais em `dir_ptr`.
///
/// Retorna um manifesto JSON:
/// `{"total_images": n, "written": m, "images": [{"page": 1, "image": {...}}]}`.
/// O chamador deve liberar a memória chamando `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_images(path_ptr: *const c_char, dir_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() || dir_ptr.is_null() {
        return create_error_json("Path/Dir pointer is null");
    }

    let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => return create_error_json("Invalid UTF-8 in path"),
    };

    let dir_str = match unsafe { CStr::from_ptr(dir_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => return create_error_json("Invalid UTF-8 in dir"),
    };

    let mut found = match crate::image::extract_images(path_str) {
        Ok(found) => found,
        Err(e) => return create_error_json(&format!("Extract error: {}", e)),
    };

    let dir = std::path::Path::new(dir_str);
    let file_name = std::path::Path::new(path_str)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.pdf".to_string());

    let written = match crate::image::write_images_to_dir(&mut found, dir) {
        Ok(written) => written,
        Err(e) => return create_error_json(&format!("Write error: {}", e)),
    };

    if let Err(e) = crate::image::write_manifest(&found, written, &file_name, dir) {
        return create_error_json(&format!("Manifest error: {}", e));
    }

    match serde_json::to_string(&crate::image::build_manifest(&found, written, &file_name)) {
        Ok(json_str) => match CString::new(json_str) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => create_error_json("Failed to create CString"),
        },
        Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
    }
}

/// Retorna a versão da biblioteca
#[no_mangle]
pub extern "C" fn rasgapdf_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

fn create_error_json(err_msg: &str) -> *mut c_char {
    let err_obj = serde_json::json!({
        "error": true,
        "message": err_msg
    });
    let s = serde_json::to_string(&err_obj).unwrap_or_else(|_| "{\"error\":true}".to_string());
    CString::new(s).unwrap().into_raw()
}


/// Modo 1 (Em Memória): Retorna lista plana de coordenadas brutas X/Y a partir de bytes em RAM.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_raw_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    if bytes_ptr.is_null() || len == 0 {
        return create_error_json("Bytes pointer is null or empty");
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
    match parse_raw_bytes(slice) {
        Ok(doc) => match serde_json::to_string(&doc) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => create_error_json("Failed to create CString"),
            },
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        },
        Err(e) => create_error_json(&format!("Parse error: {}", e)),
    }
}

/// Extrai texto puro paralelizado via Rayon diretamente de um slice de bytes em RAM.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_text_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    if bytes_ptr.is_null() || len == 0 {
        return CString::new("").unwrap().into_raw();
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
    match extract_text_bytes(slice) {
        Ok(text) => {
            // Remove eventuais bytes nulos internos para compatibilidade CString
            let clean = text.replace('\0', "");
            CString::new(clean).unwrap_or_default().into_raw()
        }
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

/// Extrai imagens embutidas diretamente de bytes em RAM e grava em `dir_ptr`.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_images_bytes(
    bytes_ptr: *const u8,
    len: usize,
    dir_ptr: *const c_char,
) -> *mut c_char {
    if bytes_ptr.is_null() || len == 0 || dir_ptr.is_null() {
        return create_error_json("Bytes/Dir pointer is null");
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
    let dir_str = match unsafe { CStr::from_ptr(dir_ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => return create_error_json("Invalid UTF-8 in dir"),
    };
    let mut found = match crate::image::extract_images_from_bytes(slice) {
        Ok(found) => found,
        Err(e) => return create_error_json(&format!("Extract error: {}", e)),
    };
    let dir = std::path::Path::new(dir_str);
    let written = match crate::image::write_images_to_dir(&mut found, dir) {
        Ok(w) => w,
        Err(e) => return create_error_json(&format!("Write error: {}", e)),
    };
    match serde_json::to_string(&crate::image::build_manifest(&found, written, "memory.pdf")) {
        Ok(json_str) => match CString::new(json_str) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => create_error_json("Failed to create CString"),
        },
        Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
    }
}


/// Modo 2 (Em Memoria / WebAssembly): Processa bytes em RAM e retorna JSON estruturado.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    rasgapdf_parse_bytes_options(bytes_ptr, len, std::ptr::null())
}

/// Modo 2 (Em Memoria / WebAssembly): Processa bytes em RAM com opcoes customizadas.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_bytes_options(
    bytes_ptr: *const u8,
    len: usize,
    options_json_ptr: *const c_char,
) -> *mut c_char {
    if bytes_ptr.is_null() || len == 0 {
        return create_error_json("Bytes pointer is null or empty");
    }

    let options: ParserOptions = if !options_json_ptr.is_null() {
        match unsafe { CStr::from_ptr(options_json_ptr) }.to_str() {
            Ok(opts_str) => serde_json::from_str(opts_str).unwrap_or_default(),
            Err(_) => ParserOptions::default(),
        }
    } else {
        ParserOptions::default()
    };

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };

    if options.mode == "raw" {
        return rasgapdf_parse_raw_bytes(bytes_ptr, len);
    }

    match crate::spatial::parse_bytes(slice, &options) {
        Ok(doc) => match serde_json::to_string(&doc) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => create_error_json("Failed to create CString"),
            },
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        },
        Err(e) => create_error_json(&format!("Parse error: {}", e)),
    }
}

/// Funcoes utilitarias de alocacao de memoria para WebAssembly no Browser:
#[no_mangle]
pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn wasm_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, 0, size);
        }
    }
}
