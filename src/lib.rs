pub mod types;
pub mod table;
pub mod drawing;
pub mod image;
pub mod spatial;
pub mod ffi;
pub mod engine;

pub use types::*;
pub use spatial::{extract_text_bytes, parse_file, parse_raw_bytes, parse_raw_file};
pub use drawing::map_drawings;
pub use image::{
    build_manifest, extract_images, extract_images_from_bytes, write_images_to_dir, write_manifest, ExtractedImageData,
};
pub use ffi::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
