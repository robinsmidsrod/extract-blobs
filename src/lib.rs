pub use self::error::{Error, Result};

use std::path::PathBuf;

use clap::Parser;
use image::Rgba;
use wild::ArgsOs;

use extractor::BlobExtractor;

mod arg_validators;
mod error;
mod extractor;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input image files
    #[arg(required(true))]
    files: Vec<PathBuf>,
    /// Chroma key color
    #[arg(short, long, default_value = "#71AA5D", value_parser = arg_validators::validate_chroma_key_color)]
    chroma_key_color: Rgba<u8>,
    /// Floodfill fuzz (euclidean distance)
    #[arg(short('f'), long, default_value_t = 17.0)]
    floodfill_fuzz: f32,
    /// Trim edges (pixels)
    #[arg(short('t'), long, default_value_t = 10)]
    trim_edges: u8,
    /// Grow edges (pixels)
    #[arg(short('g'), long, default_value_t = 6)]
    grow_edges: u8,
    /// Blur edge factor
    #[arg(short('b'), long, default_value_t = 2.0, value_parser = arg_validators::validate_blur_edge_factor)]
    blur_edge_factor: f32,
    /// Minimum pixels touching detected line
    #[arg(short('p'), long, default_value_t = 225)]
    min_pixels_touching_line: u32,
    /// Maximum detected lines
    #[arg(short('l'), long, default_value_t = 4)]
    max_lines: usize,
    /// Maximum blob rotation
    #[arg(short('r'), long, default_value_t = 10.0)]
    max_blob_rotation: f32,
    /// Output image pixel density in inches
    #[arg(short('d'), long, default_value_t = 150)]
    dpi: u32,
    /// Tesseract OCR language
    #[arg(short('L'), long, default_value = "nor")]
    ocr_language: String,
    /// Tesseract OCR page-segmentation-mode
    #[arg(short('P'), long, default_value = "3")]
    ocr_psm: String,
    /// Tesseract OCR data directory
    #[arg(short('D'), long, default_value = "../tessdata_best")]
    tessdata: PathBuf,
    /// Ignore detected DPI in input images
    #[arg(short('i'), long, default_value_t = false)]
    ignore_detected_dpi: bool,
    /// Save intermediary images
    #[arg(short('s'), long, default_value_t = false)]
    save_intermediary_images: bool,
    /// Verbose messages
    #[arg(short('v'), long, default_value_t = false)]
    verbose: bool,
}

pub fn run(args: ArgsOs) -> Result<()> {
    let args = Args::parse_from(args);
    for file in &args.files {
        let blob_extractor = BlobExtractor::new(file.to_owned(), &args);
        blob_extractor.process()?;
        println!("");
    }
    Ok(())
}
