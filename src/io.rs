use std::path::PathBuf;

use image::ImageBuffer;
use image::Rgba;
use image::Luma;

/// Save grayscale image to file with suffix appended before extension
pub(crate) fn save_luma_image_as(
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    base_path: &PathBuf,
    suffix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("{}-{}.{}", base_path.display(), suffix, "png");
    img.save(&filename)?;
    println!("{filename}: saved in gray");
    Ok(())
}

/// Save RGBA image to file with suffix appended before extension
pub(crate) fn save_rgba_image_as(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    base_path: &PathBuf,
    suffix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("{}-{}.{}", base_path.display(), suffix, "png");
    img.save(&filename)?;
    println!("{filename}: saved in color");
    Ok(())
}
