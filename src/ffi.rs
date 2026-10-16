use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use crate::types::ParserOptions;
use crate::spatial::parse_file;

/// Processa um arquivo PDF e retorna o JSON espacial como string C.
/// O chamador deve liberar a memória chamando `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_file(path_ptr: *const c_char) -> *mut c_char {
    rasgapdf_parse_file_options(path_ptr, std::ptr::null())
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

/// Retorna a versão da biblioteca
#[no_mangle]
pub extern "C" fn rasgapdf_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

fn create_error_json(err_msg: &str) -> *mut c_char {
    let err_obj = serde_json::json!({
        "error": true,
        "message": err_msg
    });
    let s = serde_json::to_string(&err_obj).unwrap_or_else(|_| "{\"error\":true}".to_string());
    CString::new(s).unwrap().into_raw()
}
