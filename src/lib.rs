pub mod drawing;
pub mod engine;
pub mod ffi;
pub mod image;
pub mod spatial;
pub mod table;
pub mod types;

pub use drawing::map_drawings;
pub use ffi::*;
pub use image::{
    build_manifest, extract_images, extract_images_from_bytes, write_images_to_dir, write_manifest,
    ExtractedImageData,
};
pub use spatial::{extract_text_bytes, parse_bytes, parse_file, parse_raw_bytes, parse_raw_file};
pub use types::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
