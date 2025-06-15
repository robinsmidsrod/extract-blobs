use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use image::ImageBuffer;
use image::Luma;
use image::Rgba;

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
