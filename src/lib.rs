pub mod types;
pub mod table;
pub mod spatial;
pub mod ffi;

pub use types::*;
pub use spatial::parse_file;
pub use ffi::*;

/// Versão semântica atual do RasgaPDF
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
