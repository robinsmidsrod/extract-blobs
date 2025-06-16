use std::path::{Path, PathBuf};

use clap::Parser;
use image::{Luma, Pixel, Rgba};
use imageproc::{distance_transform::Norm, geometric_transformations::Interpolation};

use itertools::Itertools; // for join() iterator method

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
    #[arg(short, long, default_value = "#71AA5D")]
    chroma_key_color: String,
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
    #[arg(short('b'), long, default_value_t = 2.0, value_parser = validate_blur_edge_factor)]
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

fn validate_blur_edge_factor(value: &str) -> Result<f32, String> {
    let num = value
        .parse::<f32>()
        .map_err(|_| format!("Not a valid floating point number"))?;
    if num <= 0.0 {
        return Err(format!("Number must be greater than 0"));
    }
    Ok(num)
}

struct Config {
    chroma_key_color: Rgba<u8>,
    floodfill_fuzz: f32,
    trim_edges: u8,
    grow_edges: u8,
    floodfill_color: Rgba<u8>,
    border_thickness: u32,
    blur_edge_factor: f32,
    min_pixels_touching_line: u32,
    max_lines: usize,
    max_blob_rotation: f32,
    save_intermediary_images: bool,
    verbose: bool,
    dpi: u32,
    ignore_detected_dpi: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config {
        chroma_key_color: color_ops::parse_color(&cli.chroma_key_color)?,
        floodfill_fuzz: cli.floodfill_fuzz,
        trim_edges: cli.trim_edges,
        grow_edges: cli.grow_edges,
        floodfill_color: Rgba([0, 0, 0, 0]), // transparent
        border_thickness: 1,
        blur_edge_factor: cli.blur_edge_factor,
        min_pixels_touching_line: cli.min_pixels_touching_line,
        max_lines: cli.max_lines,
        max_blob_rotation: cli.max_blob_rotation,
        dpi: cli.dpi,
        save_intermediary_images: cli.save_intermediary_images,
        verbose: cli.verbose,
        ignore_detected_dpi: cli.ignore_detected_dpi,
    };
    for file_pattern in &cli.files {
        for file_glob_result in glob::glob(file_pattern)? {
            let file_path = match file_glob_result {
                Ok(f) => f,
                Err(e) => panic!("Problem globbing the file pattern {file_pattern}: {e:?}"),
            };
            process_file(&file_path, &config)?;
            println!("");
        }
    }
    Ok(())
}

fn process_file(file: &PathBuf, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}: processing...", file.display());

    // Figure out path stuff
    let base_dir = Path::new(&file).parent().unwrap();
    let base_filename = Path::new(&file).file_stem().unwrap();
    let base_path = base_dir.join(base_filename);

    // Open image and maybe get pixel density in dots per inch
    let (image, maybe_dpi) = io::open_image(&file)?;

    // Decide which DPI to use for output images
    let dpi = match maybe_dpi {
        Some(dpi) => {
            println!("{}: detected DPI is {:?}", file.display(), dpi);
            match config.ignore_detected_dpi {
                true => (config.dpi, config.dpi),
                false => dpi,
            }
        }
        None => {
            println!("{}: unable to detect DPI", file.display());
            (config.dpi, config.dpi)
        }
    };
    println!("{}: using DPI {:?}", file.display(), dpi);

    let width = image.width();
    let height = image.height();

    let mut image_rgba = image.to_rgba8();
    let dominant_color_hex = find_dominant_color_hex(&image_rgba);
    println!(
        "{}: dominant color is #{}",
        file.display(),
        dominant_color_hex
    );

    // Draw a thin border on color image with chroma key color
    drawing::draw_border(
        &mut image_rgba,
        config.chroma_key_color,
        0,
        0,
        width,
        height,
        config.border_thickness,
    );
    if config.save_intermediary_images {
        io::save_rgba_image_as(&image_rgba, &base_path, "a-border", dpi)?;
    }

    // Floodfill color image with chroma key color, making it transparent, with a fuzz factor
    drawing::flood_fill(
        &mut image_rgba,
        0,
        0,
        config.chroma_key_color,
        config.floodfill_color,
        config.floodfill_fuzz,
    );
    if config.save_intermediary_images {
        io::save_rgba_image_as(&image_rgba, &base_path, "b-floodfilled", dpi)?;
    }

    // Extract alpha channel from color image so we can clean it up
    let mut image_mask = alpha_channel::extract(&image_rgba);
    if config.save_intermediary_images {
        io::save_luma_image_as(&image_mask, &base_path, "c-mask")?;
    }

    // Remove specs and dust from alpha channel, trim/grow outer edges slightly
    imageproc::morphology::erode_mut(&mut image_mask, Norm::L1, config.trim_edges);
    imageproc::morphology::dilate_mut(&mut image_mask, Norm::L1, config.grow_edges);
    if config.save_intermediary_images {
        io::save_luma_image_as(&image_mask, &base_path, "d-mask-cleaned")?;
    }

    // Replace alpha channel in the color image with the cleaned one
    alpha_channel::replace(&mut image_rgba, &image_mask);
    if config.save_intermediary_images {
        io::save_rgba_image_as(&image_rgba, &base_path, "e-with-mask", dpi)?;
    }

    // Extract individual blobs from the alpha channel
    let blobs = extraction::extract_blobs(&image_mask);
    println!("{}: found {} blobs", file.display(), blobs.len());
    let mut counter = 1u32;
    for blob in &blobs {
        if config.save_intermediary_images {
            io::save_luma_image_as(&blob, &base_path, &format!("mask-{counter}-a")[..])?;
        }

        // Compute values needed for image rotation
        let bounding_box = detection::compute_bounding_box(&blob, config);
        let center = detection::compute_center_from_rectangle(&bounding_box, config);
        let deskew_angle =
            detection::compute_deskew_angle_for_rectangle(&blob, &config, &base_path, counter)?;

        // Rotate mask image
        let black_luma = Luma([0u8]);
        let blob = imageproc::geometric_transformations::rotate(
            &blob,
            point_to_tuple(center),
            angle_to_radians(deskew_angle),
            Interpolation::Bicubic,
            black_luma,
        );

        // Blur mask image
        let blob = imageproc::filter::gaussian_blur_f32(&blob, config.blur_edge_factor);
        if config.save_intermediary_images {
            io::save_luma_image_as(&blob, &base_path, &format!("mask-{counter}-d-deskewed")[..])?;
        }

        // Rotate color image
        let black_rgba = Rgba([0, 0, 0, 0]);
        let mut blob_rgba = imageproc::geometric_transformations::rotate(
            &image_rgba,
            point_to_tuple(center),
            angle_to_radians(deskew_angle),
            Interpolation::Bicubic,
            black_rgba,
        );

        // Crop color image with mask set as new alpha channel
        alpha_channel::replace(&mut blob_rgba, &blob);
        let bounding_box = detection::compute_bounding_box(&blob, config);
        let blob_rgba = image::imageops::crop_imm(
            &blob_rgba,
            bounding_box.left() as u32,
            bounding_box.top() as u32,
            bounding_box.width(),
            bounding_box.height(),
        )
        .to_image();

        // Save final blob color image
        io::save_rgba_image_as(&blob_rgba, &base_path, &format!("{counter}")[..], dpi)?;

        counter += 1;
    }

    Ok(())
}

fn find_dominant_color_hex(image_rgba: &image::ImageBuffer<Rgba<u8>, Vec<u8>>) -> String {
    detection::find_dominant_color(image_rgba)
        .to_rgb()
        .channels()
        .iter()
        .map(|f| format!("{:X}", f))
        .join("")
}

fn point_to_tuple(center: imageproc::point::Point<u32>) -> (f32, f32) {
    (center.x as f32, center.y as f32)
}

fn angle_to_radians(angle: f32) -> f32 {
    angle * std::f32::consts::PI / 180.0
}
