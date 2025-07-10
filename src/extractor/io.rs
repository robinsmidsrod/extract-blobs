use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

use image::DynamicImage;
use image::ImageBuffer;
use image::ImageDecoder;
use image::Luma;
use image::Rgba;
use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata;

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
    is_debugging: bool,
}

impl ImageSaver {
    /// Construct a new ImageSaver with the specified base path and DPI
    pub fn new(base_path: &Path, dpi: Dpi, is_debugging: bool) -> Self {
        Self {
            base_path: base_path.to_owned(),
            dpi,
            is_debugging,
        }
    }
    /// Save RGBA image to PNG file with suffix appended before extension (includes pixel density header)
    pub fn save_rgba_image_as(
        &self,
        img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        suffix: &str,
    ) -> Result<()> {
        let filename = self.compute_path(suffix);
        let file = File::create(&filename)?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), img.width(), img.height());

        // Set image metadata
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // https://www.w3.org/TR/2003/REC-PNG-20031110/#11pHYs
        encoder.set_pixel_dims(Some((&self.dpi).into()));

        // Convert image buffer to raw bytes
        let mut buffer = Vec::new();
        for pixel in img.pixels() {
            buffer.extend_from_slice(&pixel.0);
        }
        encoder.write_header()?.write_image_data(&buffer)?;

        println!("{}: saved", filename.display());
        Ok(())
    }

    /// Save RGBA image to PNG file with suffix appended before extension (includes pixel density header and text blocks)
    pub fn save_rgba_image_with_text_as(
        &self,
        img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        suffix: &str,
        text: &str,
    ) -> Result<()> {
        let filename = self.compute_path(suffix);
        let file = File::create(&filename)?;

        // Set extended image metadata
        let mut info = png::Info::default();
        info.width = img.width();
        info.height = img.height();
        info.color_type = png::ColorType::Rgba;
        info.compression = png::Compression::Best;
        // https://www.w3.org/TR/2003/REC-PNG-20031110/#11pHYs
        info.pixel_dims = Some((&self.dpi).into());
        info.utf8_text = vec![png::text_metadata::ITXtChunk::new(
            "Content".to_owned(),
            text.to_owned(),
        )];
        // TODO: Figure out a crate that can generate EXIF chunk with comment
        info.exif_metadata = None;

        let encoder = png::Encoder::with_info(BufWriter::new(file), info)?;

        // Convert image buffer to raw bytes
        let mut buffer = Vec::new();
        for pixel in img.pixels() {
            buffer.extend_from_slice(&pixel.0);
        }
        encoder.write_header()?.write_image_data(&buffer)?;

        // Write text content as EXIF image description as well
        let mut exif = Metadata::new();
        exif.set_tag(ExifTag::Software(
            format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        ));
        exif.set_tag(ExifTag::ImageDescription(text.to_owned()));
        exif.write_to_file(&filename)?;

        println!("{}: saved with metadata", filename.display());
        Ok(())
    }

    /// Save grayscale image to file with suffix appended before extension
    pub fn save_luma_image_as(
        &self,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        suffix: &str,
    ) -> Result<()> {
        let filename = self.compute_path(suffix);
        img.save(&filename)?;
        println!("{}: saved", filename.display());
        Ok(())
    }

    /// Save debug RGBA image to PNG file with suffix appended before extension (includes pixel density header)
    /// Do nothing if we've been asked to not save intermediaries
    pub fn save_debug_rgba_image_as(
        &self,
        img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        suffix: &str,
    ) -> Result<()> {
        if self.is_debugging {
            return self.save_rgba_image_as(img, suffix);
        }
        Ok(())
    }

    /// Save debug grayscale image to file with suffix appended before extension
    /// Do nothing if we've been asked to not save intermediaries
    pub fn save_debug_luma_image_as(
        &self,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        suffix: &str,
    ) -> Result<()> {
        if self.is_debugging {
            return self.save_luma_image_as(img, suffix);
        }
        Ok(())
    }
    /// Compute full file path from base path and suffix
    pub fn compute_path(&self, suffix: &str) -> PathBuf {
        format!("{}-{suffix}.png", self.base_path.display()).into()
    }
}
