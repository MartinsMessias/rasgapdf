use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use crate::types::ParserOptions;
use crate::spatial::{extract_text_bytes, parse_file, parse_raw_bytes, parse_raw_file};

/// Processa um arquivo PDF e retorna o JSON espacial estruturado como string C.
/// O chamador deve liberar a memória chamando `rasgapdf_free_string`.
///
/// # Safety
/// O ponteiro `path_ptr` deve ser não-nulo e apontar para uma string C válida terminada em NUL.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_file(path_ptr: *const c_char) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        rasgapdf_parse_file_options(path_ptr, std::ptr::null())
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_file"))
}

/// Modo 1: Retorna exclusivamente a lista plana de coordenadas brutas X/Y.
///
/// # Safety
/// O ponteiro `path_ptr` deve ser não-nulo e apontar para uma string C válida terminada em NUL.
/// O ponteiro retornado deve ser liberado via `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_raw(path_ptr: *const c_char) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if path_ptr.is_null() {
            return create_error_json("Path pointer is null");
        }

        let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
            Ok(s) => s,
            Err(_) => return create_error_json("Invalid UTF-8 in path"),
        };

        match parse_raw_file(path_str) {
            Ok(doc) => match serde_json::to_string(&doc) {
                Ok(json_str) => success_json_to_c_char(json_str),
                Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
            },
            Err(e) => create_error_json(&format!("Parse error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_raw"))
}

/// Processa um PDF com opções JSON customizadas.
///
/// # Safety
/// `path_ptr` e `options_json_ptr` (se não-nulo) devem apontar para strings C válidas terminadas em NUL.
/// O ponteiro retornado deve ser liberado via `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_file_options(path_ptr: *const c_char, options_json_ptr: *const c_char) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
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
                Ok(json_str) => success_json_to_c_char(json_str),
                Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
            },
            Err(e) => create_error_json(&format!("Parse error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_file_options"))
}

/// Libera a string alocada pelo Rust.
///
/// # Safety
/// `ptr` deve ser um ponteiro previamente retornado por uma função C-ABI do RasgaPDF ou nulo.
#[no_mangle]
pub extern "C" fn rasgapdf_free_string(ptr: *mut c_char) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !ptr.is_null() {
            unsafe {
                let _ = CString::from_raw(ptr);
            }
        }
    }));
}

/// Extrai as imagens embutidas e grava os streams originais em `dir_ptr`.
///
/// Retorna um manifesto JSON:
/// `{"total_images": n, "written": m, "images": [{"page": 1, "image": {...}}]}`.
/// O chamador deve liberar a memória chamando `rasgapdf_free_string`.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_images(path_ptr: *const c_char, dir_ptr: *const c_char) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
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
            Ok(json_str) => success_json_to_c_char(json_str),
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_extract_images"))
}

/// Retorna a versão da biblioteca.
///
/// # Safety
/// Retorna um ponteiro estático de tempo de vida estático que NUNCA deve ser liberado pelo chamador.
#[no_mangle]
pub extern "C" fn rasgapdf_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

fn create_error_json(err_msg: &str) -> *mut c_char {
    let sanitized_msg = err_msg.replace('\0', " ");
    let err_obj = serde_json::json!({
        "error": true,
        "message": sanitized_msg
    });
    let s = serde_json::to_string(&err_obj).unwrap_or_else(|_| "{\"error\":true}".to_string());
    let clean_s = s.replace('\0', " ");
    CString::new(clean_s)
        .unwrap_or_else(|_| CString::new("{\"error\":true}").unwrap())
        .into_raw()
}

fn success_json_to_c_char(json_str: String) -> *mut c_char {
    let clean_json = if json_str.contains('\0') {
        json_str.replace('\0', " ")
    } else {
        json_str
    };
    match CString::new(clean_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => create_error_json("Failed to create CString from JSON result"),
    }
}

/// Modo 1 (Em Memória): Retorna lista plana de coordenadas brutas X/Y a partir de bytes em RAM.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_raw_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if bytes_ptr.is_null() || len == 0 {
            return create_error_json("Bytes pointer is null or empty");
        }
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
        match parse_raw_bytes(slice) {
            Ok(doc) => match serde_json::to_string(&doc) {
                Ok(json_str) => success_json_to_c_char(json_str),
                Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
            },
            Err(e) => create_error_json(&format!("Parse error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_raw_bytes"))
}

/// Extrai texto puro paralelizado via Rayon diretamente de um slice de bytes em RAM.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_text_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if bytes_ptr.is_null() || len == 0 {
            return CString::new("").unwrap().into_raw();
        }
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
        match extract_text_bytes(slice) {
            Ok(text) => {
                // Remove eventuais bytes nulos internos para compatibilidade CString
                let clean = text.replace('\0', "");
                CString::new(clean).unwrap_or_else(|_| CString::new("").unwrap()).into_raw()
            }
            Err(_) => CString::new("").unwrap().into_raw(),
        }
    }))
    .unwrap_or_else(|_| CString::new("").unwrap().into_raw())
}

/// Extrai imagens embutidas diretamente de bytes em RAM e grava em `dir_ptr`.
#[no_mangle]
pub extern "C" fn rasgapdf_extract_images_bytes(
    bytes_ptr: *const u8,
    len: usize,
    dir_ptr: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
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
            Ok(json_str) => success_json_to_c_char(json_str),
            Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_extract_images_bytes"))
}

/// Modo 2 (Em Memória / WebAssembly): Processa bytes em RAM e retorna JSON estruturado.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_bytes(bytes_ptr: *const u8, len: usize) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        rasgapdf_parse_bytes_options(bytes_ptr, len, std::ptr::null())
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_bytes"))
}

/// Modo 2 (Em Memória / WebAssembly): Processa bytes em RAM com opcoes customizadas.
#[no_mangle]
pub extern "C" fn rasgapdf_parse_bytes_options(
    bytes_ptr: *const u8,
    len: usize,
    options_json_ptr: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
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
                Ok(json_str) => success_json_to_c_char(json_str),
                Err(e) => create_error_json(&format!("JSON serialization error: {}", e)),
            },
            Err(e) => create_error_json(&format!("Parse error: {}", e)),
        }
    }))
    .unwrap_or_else(|_| create_error_json("Panic caught across FFI boundary in rasgapdf_parse_bytes_options"))
}

/// Funcoes utilitarias de alocacao de memoria para WebAssembly no Browser:
#[no_mangle]
pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
    catch_unwind(AssertUnwindSafe(|| {
        if size == 0 {
            return std::ptr::null_mut();
        }
        let layout = match Layout::from_size_align(size, 1) {
            Ok(l) => l,
            Err(_) => return std::ptr::null_mut(),
        };
        unsafe { alloc(layout) }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn wasm_free(ptr: *mut u8, size: usize) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !ptr.is_null() && size > 0 {
            if let Ok(layout) = Layout::from_size_align(size, 1) {
                unsafe {
                    dealloc(ptr, layout);
                }
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_error_json_with_null_byte() {
        let err_ptr = create_error_json("Test error with \0 nul byte inside");
        assert!(!err_ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(err_ptr) };
        let str_slice = c_str.to_str().unwrap();
        assert!(str_slice.contains("\"error\":true"));
        assert!(str_slice.contains("Test error with   nul byte inside"));
        rasgapdf_free_string(err_ptr);
    }

    #[test]
    fn test_success_json_with_null_character() {
        let json_with_null = "{\"file_name\":\"test\\u0000file\",\"text\":\"hello\0world\"}".to_string();
        let ptr = success_json_to_c_char(json_with_null);
        assert!(!ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let s = c_str.to_str().unwrap();
        assert!(s.contains("hello world"));
        rasgapdf_free_string(ptr);
    }

    #[test]
    fn test_null_pointers_do_not_crash_ffi() {
        let p1 = rasgapdf_parse_file(std::ptr::null());
        assert!(!p1.is_null());
        rasgapdf_free_string(p1);

        let p2 = rasgapdf_parse_raw(std::ptr::null());
        assert!(!p2.is_null());
        rasgapdf_free_string(p2);

        let p3 = rasgapdf_parse_file_options(std::ptr::null(), std::ptr::null());
        assert!(!p3.is_null());
        rasgapdf_free_string(p3);

        let p4 = rasgapdf_extract_images(std::ptr::null(), std::ptr::null());
        assert!(!p4.is_null());
        rasgapdf_free_string(p4);

        let p5 = rasgapdf_parse_raw_bytes(std::ptr::null(), 0);
        assert!(!p5.is_null());
        rasgapdf_free_string(p5);

        let p6 = rasgapdf_extract_text_bytes(std::ptr::null(), 0);
        assert!(!p6.is_null());
        rasgapdf_free_string(p6);

        let p7 = rasgapdf_extract_images_bytes(std::ptr::null(), 0, std::ptr::null());
        assert!(!p7.is_null());
        rasgapdf_free_string(p7);

        let p8 = rasgapdf_parse_bytes(std::ptr::null(), 0);
        assert!(!p8.is_null());
        rasgapdf_free_string(p8);

        let p9 = rasgapdf_parse_bytes_options(std::ptr::null(), 0, std::ptr::null());
        assert!(!p9.is_null());
        rasgapdf_free_string(p9);

        // Freeing null pointer should be safe and no-op
        rasgapdf_free_string(std::ptr::null_mut());
    }

    #[test]
    fn test_wasm_alloc_and_free() {
        // Zero size should return null without UB
        let null_ptr = wasm_alloc(0);
        assert!(null_ptr.is_null());
        wasm_free(null_ptr, 0);

        // Freeing null with non-zero size or non-null with zero size should be safe
        wasm_free(std::ptr::null_mut(), 100);

        // Normal allocation
        let size = 1024;
        let ptr = wasm_alloc(size);
        assert!(!ptr.is_null());
        unsafe {
            std::ptr::write_bytes(ptr, 0xAA, size);
            assert_eq!(*ptr, 0xAA);
            assert_eq!(*ptr.add(size - 1), 0xAA);
        }
        wasm_free(ptr, size);
    }
}
