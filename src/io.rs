use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::PathBuf;

use exif::In;
use exif::Tag;
use image::DynamicImage;
use image::ImageBuffer;
use image::ImageDecoder;
use image::Luma;
use image::Rgba;
use jfifdump::SegmentKind;

// Open image file and include raw EXIF data, if any
pub(crate) fn open_image(
    file: &PathBuf,
) -> Result<(DynamicImage, Option<(u32, u32)>), Box<dyn std::error::Error>> {
    let file_contents = std::fs::read(file)?;
    let c = Cursor::new(file_contents.as_slice());
    let r = BufReader::new(c);
    let image_reader = image::ImageReader::new(r).with_guessed_format()?;
    let mut decoder = image_reader.into_decoder()?;
    let maybe_exif = decoder.exif_metadata()?;
    let image = DynamicImage::from_decoder(decoder)?;
    let maybe_dpi = read_dpi_from_metadata(file_contents.as_slice(), maybe_exif)?;
    Ok((image, maybe_dpi))
}

/// Read pixel density from file metadata
fn read_dpi_from_metadata(
    file_contents: &[u8],
    maybe_exif: Option<Vec<u8>>,
) -> Result<Option<(u32, u32)>, Box<dyn std::error::Error>> {
    let dpi = match maybe_exif {
        Some(exif) => read_dpi_from_exif(&exif)?,
        None => read_dpi_from_jfif(file_contents)?,
    };
    // TODO: Support reading PNG pixel density
    Ok(dpi)
}

/// Read pixel density from EXIF header
fn read_dpi_from_exif(
    exif_raw: &Vec<u8>,
) -> Result<Option<(u32, u32)>, Box<dyn std::error::Error>> {
    let reader = exif::Reader::new();
    let maybe_exif = reader.read_raw(exif_raw.to_vec());
    let exif = match maybe_exif {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };
    let unit = exif.get_field(Tag::ResolutionUnit, In::PRIMARY);
    let x_res = exif.get_field(Tag::XResolution, In::PRIMARY);
    let y_res = exif.get_field(Tag::YResolution, In::PRIMARY);
    let Some(unit) = unit else { return Ok(None) };
    let Some(x_res) = x_res else { return Ok(None) };
    let Some(y_res) = y_res else { return Ok(None) };
    let Some(unit) = unit.value.get_uint(0) else {
        return Ok(None);
    };
    let x_res = match x_res.value {
        exif::Value::Rational(ref vec) if !vec.is_empty() => vec[0].to_f32() as u32,
        _ => return Ok(None),
    };
    let y_res = match y_res.value {
        exif::Value::Rational(ref vec) if !vec.is_empty() => vec[0].to_f32() as u32,
        _ => return Ok(None),
    };
    // println!("EXIF: unit={:?}, xres={:?}, yres={:?}", unit, x_res, y_res);
    // https://www.media.mit.edu/pia/Research/deepview/exif.html#ExifTags
    // 1 means no-unit
    // 2 means inch
    // 3 means centimeter
    if unit == 2 {
        return Ok(Some((x_res, y_res)));
    }
    if unit == 3 {
        let x_res = (x_res as f32 * 2.54) as u32;
        let y_res = (y_res as f32 * 2.54) as u32;
        return Ok(Some((x_res, y_res)));
    }
    Ok(None)
}

/// Read pixel density from JPEG JFIF header
fn read_dpi_from_jfif(
    file_contents: &[u8],
) -> Result<Option<(u32, u32)>, Box<dyn std::error::Error>> {
    let c = Cursor::new(file_contents);
    let r = BufReader::new(c);
    let reader = jfifdump::Reader::new(r);
    let Ok(mut reader) = reader else {
        return Ok(None);
    };
    loop {
        let next_segment = reader.next_segment();
        let Ok(next_segment) = next_segment else {
            return Ok(None);
        };
        match next_segment.kind {
            SegmentKind::Eoi => break,
            SegmentKind::App0Jfif(jfif) => {
                // https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#JFIF_APP0_marker_segment
                // println!(
                //     "JFIF: unit={},x_density={},y_density={}",
                //     jfif.unit, jfif.x_density, jfif.y_density
                // );
                // unit=0 means pixel aspect ratio (y:x)
                // unit=1 means pixels per inch (2.54cm)
                // unit=2 means pixels per centimeter
                if jfif.unit == 1 {
                    return Ok(Some((jfif.x_density as u32, jfif.y_density as u32)));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Save grayscale image to file with suffix appended before extension
pub(crate) fn save_luma_image_as(
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    base_path: &PathBuf,
    suffix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("{}-{}.{}", base_path.display(), suffix, "png");
    img.save(&filename)?;
    println!("{filename}: saved");
    Ok(())
}

/// Save RGBA image to file with suffix appended before extension
pub(crate) fn save_rgba_image_as(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    base_path: &PathBuf,
    suffix: &str,
    dpi: u32,
) -> Result<(), Box<dyn std::error::Error>> {
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
        xppu: (dpi as f32 * 39.37) as u32, // 1 inch = 39.37 cm
        yppu: (dpi as f32 * 39.37) as u32,
        unit: png::Unit::Meter,
    }));
    encoder.write_header()?.write_image_data(&buffer)?;

    //img.save(&filename)?;

    println!("{filename}: saved");
    Ok(())
}
