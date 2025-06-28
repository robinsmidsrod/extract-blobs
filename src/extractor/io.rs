use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::Path;

use exif::In;
use exif::Tag;
use exif::Value;
use image::DynamicImage;
use image::ImageBuffer;
use image::ImageDecoder;
use image::ImageResult;
use image::Luma;
use image::Rgba;
use jfifdump::SegmentKind;

use crate::Result;

/// Open image file and include raw EXIF data, if any
pub(crate) fn open_image(file: &Path) -> Result<(DynamicImage, Option<(u32, u32)>)> {
    let file_contents = std::fs::read(file)?;
    let c = Cursor::new(file_contents.as_slice());
    let r = BufReader::new(c);
    let image_reader = image::ImageReader::new(r).with_guessed_format()?;
    let mut decoder = image_reader.into_decoder()?;
    let maybe_exif = decoder.exif_metadata()?;
    let image = DynamicImage::from_decoder(decoder)?;
    let maybe_dpi = read_dpi_from_metadata(file_contents.as_slice(), maybe_exif);
    Ok((image, maybe_dpi))
}

/// Read pixel density from file metadata
fn read_dpi_from_metadata(file_contents: &[u8], maybe_exif: Option<Vec<u8>>) -> Option<(u32, u32)> {
    match maybe_exif {
        Some(exif) => read_dpi_from_exif(&exif),
        None => read_dpi_from_jfif(file_contents),
    }
    // TODO: Support reading PNG pixel density
}

/// Read pixel density from EXIF header
fn read_dpi_from_exif(exif_raw: &[u8]) -> Option<(u32, u32)> {
    let reader = exif::Reader::new();
    let exif = reader.read_raw(exif_raw.to_vec()).ok()?;
    let unit = exif
        .get_field(Tag::ResolutionUnit, In::PRIMARY)
        .and_then(|unit| unit.value.get_uint(0))?;
    let x_res = match &exif.get_field(Tag::XResolution, In::PRIMARY)?.value {
        Value::Rational(vec) => vec.first().map(|value| value.to_f32() as u32),
        _ => return None,
    }?;
    let y_res = match &exif.get_field(Tag::YResolution, In::PRIMARY)?.value {
        Value::Rational(vec) => vec.first().map(|value| value.to_f32() as u32),
        _ => return None,
    }?;
    // println!("EXIF: unit={:?}, xres={:?}, yres={:?}", unit, x_res, y_res);
    // https://www.media.mit.edu/pia/Research/deepview/exif.html#ExifTags
    match unit {
        // 1 means no-unit (aspect ratio)
        1 => None,
        // 2 means inch
        2 => Some((x_res, y_res)),
        // 3 means centimeter
        3 => Some(((x_res as f32 * 2.54) as u32, (y_res as f32 * 2.54) as u32)),
        _ => None,
    }
}

/// Read pixel density from JPEG JFIF header
fn read_dpi_from_jfif(file_contents: &[u8]) -> Option<(u32, u32)> {
    let c = Cursor::new(file_contents);
    let r = BufReader::new(c);
    let mut reader = jfifdump::Reader::new(r).ok()?;
    loop {
        match reader.next_segment().ok()?.kind {
            // https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#File_format_structure
            SegmentKind::Eoi => break,
            // https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#JFIF_APP0_marker_segment
            SegmentKind::App0Jfif(jfif) => {
                // println!(
                //     "JFIF: unit={},x_density={},y_density={}",
                //     jfif.unit, jfif.x_density, jfif.y_density
                // );
                match jfif.unit {
                    // unit=0 means pixel aspect ratio (y:x)
                    0 => return None,
                    // unit=1 means pixels per inch (2.54cm)
                    1 => return Some((jfif.x_density as u32, jfif.y_density as u32)),
                    // unit=2 means pixels per centimeter
                    2 => return None, // TODO
                    _ => return None,
                }
            }
            _ => {}
        }
    }
    None
}

/// Save grayscale image to file with suffix appended before extension
pub(crate) fn save_luma_image_as(
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    base_path: &Path,
    suffix: &str,
) -> ImageResult<()> {
    let filename = format!("{}-{}.{}", base_path.display(), suffix, "png");
    img.save(&filename)?;
    println!("{filename}: saved");
    Ok(())
}

/// Save RGBA image to PNG file with suffix appended before extension (includes pixel density header)
pub(crate) fn save_rgba_image_as(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    base_path: &Path,
    suffix: &str,
    dpi: (u32, u32),
) -> Result<()> {
    let filename = format!("{}-{}.{}", base_path.display(), suffix, "png");

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
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: (dpi.0 as f32 * 39.37) as u32, // 1 inch = 39.37 cm
        yppu: (dpi.1 as f32 * 39.37) as u32,
        unit: png::Unit::Meter,
    }));
    encoder.write_header()?.write_image_data(&buffer)?;
    println!("{filename}: saved");
    Ok(())
}
