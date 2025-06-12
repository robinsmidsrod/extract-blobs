use std::path::{Path, PathBuf};

use clap::Parser;
use image::{ImageBuffer, Luma, Rgba, imageops::crop_imm};
use imageproc::{
    contours::Contour, distance_transform::Norm, geometric_transformations::Interpolation,
    hough::LineDetectionOptions, point::Point, rect::Rect,
};
use itertools::Itertools;

mod alpha_channel;
mod color_ops;
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
    io::save_rgba_image_as(&imgb, &base_path, "border")?;

    // Flood fill color image with chroma key color, making it transparent, with a fuzz factor
    let transparent = image::Rgba([0, 0, 0, 0]);
    drawing::flood_fill(&mut imgb, 0, 0, chroma_key_color, transparent, 25.0);
    io::save_rgba_image_as(&imgb, &base_path, "floodfilled")?;

    // Extract alpha channel from color image so we can clean it up
    let mut img_alpha = alpha_channel::extract(&imgb);
    io::save_luma_image_as(&img_alpha, &base_path, "alpha")?;

    // Remove specs and dust from alpha channel, trim outer edges slightly
    imageproc::morphology::erode_mut(&mut img_alpha, Norm::L1, 5);
    imageproc::morphology::dilate_mut(&mut img_alpha, Norm::L1, 3);
    io::save_luma_image_as(&img_alpha, &base_path, "alpha-eroded")?;

    // Replace alpha channel in the color image with the cleaned one
    alpha_channel::replace(&mut imgb, &img_alpha);
    io::save_rgba_image_as(&imgb, &base_path, "floodfilled-with-clean-alpha")?;

    // Extract individual blobs from the alpha channel
    let blobs = extraction::extract_blobs(&img_alpha);
    let mut counter = 0u32;
    for blob in &blobs {
        let skew_angle = experiment_with_mask_image(&blob, &base_path, counter)?;
        io::save_luma_image_as(&blob, &base_path, &format!("blob-{counter}")[..])?;
        let (_skew_angle, center) = compute_skew_angle_and_rotation_center(&blob);
        let skew_theta = skew_angle * std::f32::consts::PI / 180.0;
        println!("Computed skew angle: {skew_angle}");
        println!("Rotation center: {center:?}");
        let img_mask_rotated = imageproc::geometric_transformations::rotate(
            &blob,
            (center.x as f32, center.y as f32),
            skew_theta,
            Interpolation::Bicubic,
            Luma([0u8]),
        );
        let img_mask_rotated_and_blurred =
            imageproc::filter::gaussian_blur_f32(&img_mask_rotated, 3.0);
        io::save_luma_image_as(
            &img_mask_rotated_and_blurred,
            &base_path,
            &format!("blob-{counter}-deskewed")[..],
        )?;

        let mut imgb_rotated = imageproc::geometric_transformations::rotate(
            &imgb,
            (center.x as f32, center.y as f32),
            skew_theta,
            Interpolation::Bicubic,
            Rgba([0, 0, 0, 0]),
        );
        alpha_channel::replace(&mut imgb_rotated, &img_mask_rotated_and_blurred);
        let bounding_box = compute_bounding_box(&img_mask_rotated_and_blurred);
        let imgb_cropped = crop_imm(
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

/// Compute skew angle, bounding box and rotation center from luma image
///
/// There should only be one blob in the specified image
fn compute_skew_angle_and_rotation_center(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
) -> (f32, Point<u32>) {
    let points = imageproc::geometry::convex_hull(find_contour_points(image));

    // Can't compute an angle with less than two points
    if points.len() < 2 {
        let bounding_box = Rect::at(0, 0).of_size(image.width(), image.height());
        let center = imageproc::point::Point::new(
            bounding_box.left() as u32 + bounding_box.width() / 2,
            bounding_box.top() as u32 + bounding_box.height() / 2,
        );
        return (0.0, center);
    }

    // Find the leftmost, topmost and bottommost points in the list
    // The topmost and bottommost points should match on the lowest x value
    // The leftmost point should match on the y value closer to the top
    // The rightmost point isn't really used to determine the skew
    let mut leftmost_point = points[0];
    let mut rightmost_point = points[0];
    let mut topmost_point = points[0];
    let mut bottommost_point = points[0];
    for i in 1..points.len() {
        let p = points[i];
        if p.x < leftmost_point.x {
            leftmost_point = p;
        }
        if p.x == leftmost_point.x && p.y < leftmost_point.y {
            leftmost_point = p;
        }
        if p.x > rightmost_point.x {
            rightmost_point = p;
        }
        if p.y < topmost_point.y {
            topmost_point = p;
        }
        if p.y == topmost_point.y && p.x < topmost_point.x {
            topmost_point = p;
        }
        if p.y > bottommost_point.y {
            bottommost_point = p;
        }
        if p.y == bottommost_point.y && p.x < bottommost_point.x {
            bottommost_point = p;
        }
    }
    println!("Topmost    point: {topmost_point:?}");
    println!("Leftmost   point: {leftmost_point:?}");
    println!("Rightmost  point: {rightmost_point:?}");
    println!("Bottommost point: {bottommost_point:?}");

    let top_horizontal_line_length = topmost_point.x - leftmost_point.x;
    let top_vertical_line_length = leftmost_point.y - topmost_point.y;
    let bottom_horizontal_line_length = bottommost_point.x - leftmost_point.x;
    let bottom_vertical_line_length = bottommost_point.y - leftmost_point.y;
    println!("Top    horizontal line length: {top_horizontal_line_length}");
    println!("Top    vertical   line length: {top_vertical_line_length}");
    println!("Bottom horizontal line length: {bottom_horizontal_line_length}");
    println!("Bottom vertical   line length: {bottom_vertical_line_length}");

    // Figure out which triangle to use to ensure the smallest angle is calculated between the longest lines
    // If the top triangle vertical line is longer than the bottom vertical line, then the angle should be negative
    let a;
    let b;
    let direction_factor: f32;
    if top_vertical_line_length > bottom_vertical_line_length {
        println!("Skewed to the right");
        direction_factor = -1.0;
        a = topmost_point.x as f32 - leftmost_point.x as f32;
        b = leftmost_point.y as f32 - topmost_point.y as f32;
    } else {
        println!("Skewed to the left");
        direction_factor = 1.0;
        a = bottommost_point.x as f32 - leftmost_point.x as f32;
        b = bottommost_point.y as f32 - leftmost_point.y as f32;
    }

    // Calculate the smallest angle in a right-angled triangle where the two points are between the hypothenus
    let c = (a.powi(2) + b.powi(2)).sqrt();
    let angle1_rad = (a / c).asin();
    let angle2_rad = (b / c).asin();
    let smallest_angle = if angle1_rad < angle2_rad {
        angle1_rad
    } else {
        angle2_rad
    };
    let mut angle = smallest_angle * 180.0 / std::f32::consts::PI;

    angle = angle * direction_factor;
    // Avoid excessive rotation
    if angle > 10.0 || angle < -10.0 {
        angle = 0.0;
    }
    let bounding_box = Rect::at(leftmost_point.x as i32, topmost_point.y as i32).of_size(
        rightmost_point.x - leftmost_point.x,
        bottommost_point.y - topmost_point.y,
    );
    println!("Blob bounding box (before deskew): {bounding_box:?}");
    let center = imageproc::point::Point::new(
        bounding_box.left() as u32 + bounding_box.width() / 2,
        bounding_box.top() as u32 + bounding_box.height() / 2,
    );
    (angle, center)
}

/// Find edges in the image and extract a list of points
fn find_contour_points(image: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<Point<u32>> {
    let mut contours: Vec<Contour<u32>> = imageproc::contours::find_contours(image);
    let points = match contours.pop() {
        Some(contour) => contour.points,
        None => vec![],
    };
    points
}

/// Compute bounding box from grayscale image, any non-black color is considered part of the bounding box
fn compute_bounding_box(image: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Rect {
    let points = find_contour_points(image);
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    for i in 0..points.len() {
        let p = points[i];
        if p.x < left {
            left = p.x;
        }
        if p.x > right {
            right = p.x;
        }
        if p.y < top {
            top = p.y;
        }
        if p.y > bottom {
            bottom = p.y;
        }
    }
    let bounding_box = Rect::at(left as i32, top as i32).of_size(right - left, bottom - top);
    println!("Blob bounding box: {bounding_box:?}");
    bounding_box
}

fn experiment_with_mask_image(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    base_path: &std::path::PathBuf,
    index: u32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let mut image = imageproc::edges::canny(&image, 1.0, 1.0);
    io::save_luma_image_as(&image, base_path, &format!("{index}-canny")[..])?;

    // imageproc::morphology::dilate_mut(&mut image, Norm::LInf, 5);
    // //let mut image = imageproc::filter::gaussian_blur_f32(&image, 3.0);
    // imageproc::morphology::erode_mut(&mut image, Norm::LInf, 4);
    // imageproc::morphology::dilate_mut(&mut image, Norm::LInf, 5);
    // imageproc::morphology::erode_mut(&mut image, Norm::LInf, 5);
    // save_luma_image_as(&image, base_path, &format!("{index}-canny-adjusted")[..])?;

    let options = LineDetectionOptions {
        vote_threshold: 250, // understood as number of pixels that should be on the line
        suppression_radius: 50,
    };
    let mut lines = imageproc::hough::detect_lines(&image, options);
    println!("Number of lines detected: {}", lines.len());
    if lines.is_empty() {
        return Ok(0.0);
    }
    lines.truncate(4);
    imageproc::hough::draw_polar_lines_mut(&mut image, &lines[..], Luma([128u8]));
    io::save_luma_image_as(&image, base_path, &format!("{index}-canny-lines")[..])?;

    let angles: Vec<i32> = lines
        .iter()
        .map(|pl| pl.angle_in_degrees as i32)
        .map(|a| if a > 10 { a - 90 } else { a })
        .map(|a| if a > 10 { a - 90 } else { a })
        .sorted()
        .collect();
    println!("{angles:?}");
    // Find median angle
    let mid = angles.len() / 2;
    let angle: f32 = if angles.len() % 2 == 0 {
        (angles[mid - 1] as f32 + angles[mid] as f32) / 2.0
    } else {
        angles[mid] as f32
    };
    let inverted_angle = angle * -1.0;
    println!("Median inverted skew angle: {inverted_angle}");

    Ok(inverted_angle)
}
