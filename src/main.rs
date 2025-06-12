use std::path::{Path, PathBuf};

use clap::Parser;
use image::{Luma, Rgba};
use imageproc::{distance_transform::Norm, geometric_transformations::Interpolation};

mod alpha_channel;
mod color_ops;
mod detection;
mod drawing;
mod extraction;
mod io;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input image files
    #[arg()]
    files: Vec<String>,
    /// Chroma key color
    #[arg(short, long, default_value = "#72B34B")]
    chroma_key_color: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    for file_pattern in &cli.files {
        for file_glob_result in glob::glob(file_pattern)? {
            let file_path = match file_glob_result {
                Ok(f) => f,
                Err(e) => panic!("Problem globbing the file: {e:?}"),
            };
            process_file(&file_path, &cli.chroma_key_color)?;
        }
    }
    Ok(())
}

fn process_file(file: &PathBuf, chroma_key_color: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Figure out path stuff
    let base_dir = Path::new(&file).parent().unwrap();
    let base_filename = Path::new(&file).file_stem().unwrap();
    let base_path = base_dir.join(base_filename);
    // println!("{}: (stem)", base_path.display());

    let img = image::open(&file)?;
    println!("{}: {}x{}", file.display(), img.width(), img.height());
    let mut imgb = img.to_rgba8();

    // Draw a thin border on color image with chroma key color
    let chroma_key_color = color_ops::parse_color(&chroma_key_color)?;
    drawing::draw_border(
        &mut imgb,
        chroma_key_color,
        0,
        0,
        img.width(),
        img.height(),
        1,
    );
    io::save_rgba_image_as(&imgb, &base_path, "a-border")?;

    // Flood fill color image with chroma key color, making it transparent, with a fuzz factor
    let transparent = image::Rgba([0, 0, 0, 0]);
    drawing::flood_fill(&mut imgb, 0, 0, chroma_key_color, transparent, 25.0);
    io::save_rgba_image_as(&imgb, &base_path, "b-floodfilled")?;

    // Extract alpha channel from color image so we can clean it up
    let mut img_alpha = alpha_channel::extract(&imgb);
    io::save_luma_image_as(&img_alpha, &base_path, "b-mask")?;

    // Remove specs and dust from alpha channel, trim outer edges slightly
    imageproc::morphology::erode_mut(&mut img_alpha, Norm::L1, 5);
    imageproc::morphology::dilate_mut(&mut img_alpha, Norm::L1, 3);
    io::save_luma_image_as(&img_alpha, &base_path, "b-mask-cleaned")?;

    // Replace alpha channel in the color image with the cleaned one
    alpha_channel::replace(&mut imgb, &img_alpha);
    io::save_rgba_image_as(&imgb, &base_path, "c-with-mask")?;

    // Extract individual blobs from the alpha channel
    let blobs = extraction::extract_blobs(&img_alpha);
    let mut counter = 0u32;
    for blob in &blobs {
        let deskew_angle =
            detection::compute_deskew_angle_for_rectangle(&blob, &base_path, counter)?;
        io::save_luma_image_as(&blob, &base_path, &format!("mask-{counter}")[..])?;
        let (_skew_angle, center) = detection::compute_skew_angle_and_rotation_center(&blob);
        let deskew_theta = deskew_angle * std::f32::consts::PI / 180.0;
        println!("Computed deskew angle: {deskew_angle}");
        println!("Rotation center: {center:?}");
        let img_mask_rotated = imageproc::geometric_transformations::rotate(
            &blob,
            (center.x as f32, center.y as f32),
            deskew_theta,
            Interpolation::Bicubic,
            Luma([0u8]),
        );
        let img_mask_rotated_and_blurred =
            imageproc::filter::gaussian_blur_f32(&img_mask_rotated, 3.0);
        io::save_luma_image_as(
            &img_mask_rotated_and_blurred,
            &base_path,
            &format!("mask-{counter}-deskewed")[..],
        )?;

        let mut imgb_rotated = imageproc::geometric_transformations::rotate(
            &imgb,
            (center.x as f32, center.y as f32),
            deskew_theta,
            Interpolation::Bicubic,
            Rgba([0, 0, 0, 0]),
        );
        alpha_channel::replace(&mut imgb_rotated, &img_mask_rotated_and_blurred);
        let bounding_box = detection::compute_bounding_box(&img_mask_rotated_and_blurred);
        let imgb_cropped = image::imageops::crop_imm(
            &imgb_rotated,
            bounding_box.left() as u32,
            bounding_box.top() as u32,
            bounding_box.width(),
            bounding_box.height(),
        )
        .to_image();
        io::save_rgba_image_as(&imgb_cropped, &base_path, &format!("{counter}")[..])?;
        counter += 1;
    }

    Ok(())
}
