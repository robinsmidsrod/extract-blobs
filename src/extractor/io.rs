use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

use image::DynamicImage;
use image::ImageBuffer;
use image::ImageDecoder;
use image::ImageResult;
use image::Luma;
use image::Rgba;

use super::dpi::Dpi;
use super::dpi::decoder;
use crate::Result;

/// Open image file and decode DPI from file metadata, if any
pub(crate) fn open_image(file: &Path) -> Result<(DynamicImage, Option<Dpi>)> {
    let file_contents = std::fs::read(file)?;
    let c = Cursor::new(file_contents.as_slice());
    let r = BufReader::new(c);
    let image_reader = image::ImageReader::new(r).with_guessed_format()?;
    let mut decoder = image_reader.into_decoder()?;
    let exif = decoder.exif_metadata()?.unwrap_or_default();
    let image = DynamicImage::from_decoder(decoder)?;
    let dpi = decoder::read_from_bytes(file_contents.as_slice(), exif.as_slice());
    Ok((image, dpi))
}

/// Helper to avoid having to specify common information for saving images over and over again
pub struct ImageSaver {
    base_path: PathBuf,
    dpi: Dpi,
}

impl ImageSaver {
    /// Construct a new ImageSaver with the specified base path and DPI
    pub fn new(base_path: &Path, dpi: Dpi) -> Self {
        Self {
            base_path: base_path.to_owned(),
            dpi,
        }
    }
    /// Save RGBA image to PNG file with suffix appended before extension (includes pixel density header)
    pub fn save_rgba_image_as(
        &self,
        img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        suffix: &str,
    ) -> Result<()> {
        let filename = format!("{}-{}.{}", self.base_path.display(), suffix, "png");

        // Convert image buffer to raw bytes
        let mut buffer = Vec::new();
        for pixel in img.pixels() {
            buffer.extend_from_slice(&pixel.0);
        }

        let file = File::create(&filename)?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), img.width(), img.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // https://www.w3.org/TR/2003/REC-PNG-20031110/#11pHYs
        encoder.set_pixel_dims(Some((&self.dpi).into()));
        encoder.write_header()?.write_image_data(&buffer)?;
        println!("{filename}: saved");
        Ok(())
    }

    /// Save grayscale image to file with suffix appended before extension
    pub fn save_luma_image_as(
        &self,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        suffix: &str,
    ) -> ImageResult<()> {
        let filename = format!("{}-{}.{}", self.base_path.display(), suffix, "png");
        img.save(&filename)?;
        println!("{filename}: saved");
        Ok(())
    }
}
