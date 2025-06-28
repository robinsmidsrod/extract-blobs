use std::collections::HashMap;
use std::path::Path;

use image::ImageBuffer;
use image::ImageResult;
use image::Luma;
use image::Pixel; // for to_rgb() method
use image::Rgba;
use imageproc::hough::LineDetectionOptions;
use imageproc::point::Point;
use imageproc::rect::Rect;
use itertools::Itertools; // for sorted() and join() iterator function

use super::io;
use crate::BlobExtractor;

/// Compute bounding box from grayscale image, any non-black color is considered part of the bounding box
pub(crate) fn compute_bounding_box(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    config: &BlobExtractor,
) -> Rect {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        // Black pixels are skipped
        if pixel[0] == 0 {
            continue;
        }
        if x < left {
            left = x;
        }
        if x > right {
            right = x;
        }
        if y < top {
            top = y;
        }
        if y > bottom {
            bottom = y;
        }
    }
    let bounding_box = Rect::at(left as i32, top as i32).of_size(right - left, bottom - top);
    if config.verbose {
        println!("Computed bounding box: {left}x{top} - {right}x{bottom}");
    }
    bounding_box
}

/// Find center point in a rectangle
pub(crate) fn compute_center_from_rectangle(rect: &Rect, config: &BlobExtractor) -> Point<u32> {
    let center = imageproc::point::Point::new(
        rect.left() as u32 + rect.width() / 2,
        rect.top() as u32 + rect.height() / 2,
    );
    if config.verbose {
        println!("Computed center of rectangle: {}x{}", center.x, center.y);
    }
    center
}

/// Compute the deskew angle for a rectangular blob
pub(crate) fn compute_deskew_angle_for_rectangle(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    config: &BlobExtractor,
    base_path: &Path,
    index: u32,
) -> ImageResult<f32> {
    // Detect edges in image
    // NB: I have no idea how low/high thresholds work, but a value of 1.0 for both seems to do the trick
    let mut image = imageproc::edges::canny(image, 1.0, 1.0);
    if config.save_intermediary_images {
        io::save_luma_image_as(&image, base_path, &format!("mask-{index}-b-edges")[..])?;
    }

    // Find lines matching edges
    let options = LineDetectionOptions {
        vote_threshold: config.min_pixels_touching_line, // understood as number of pixels that should be on the line
        suppression_radius: 50,
    };
    let mut lines = imageproc::hough::detect_lines(&image, options);
    if lines.is_empty() {
        return Ok(0.0);
    }
    lines.truncate(config.max_lines);
    let grey_luma = Luma([128u8]);
    imageproc::hough::draw_polar_lines_mut(&mut image, &lines[..], grey_luma);
    if config.save_intermediary_images {
        io::save_luma_image_as(&image, base_path, &format!("mask-{index}-c-lines")[..])?;
    }

    // Rotate lines so that they all point in the same direction
    // Sort all the values from low to high
    // PolarLines angles are between 0-180 degrees
    let angles: Vec<i32> = lines
        .iter()
        .map(|pl| pl.angle_in_degrees as i32)
        .map(|a| {
            if a > config.max_blob_rotation as i32 {
                a - 90
            } else {
                a
            }
        })
        .map(|a| {
            if a > config.max_blob_rotation as i32 {
                a - 90
            } else {
                a
            }
        })
        .sorted()
        .collect();

    // Find median angle (mean value between the two in the middle if an even number of lines)
    let mid = angles.len() / 2;
    let angle: f32 = if angles.len() % 2 == 0 {
        (angles[mid - 1] as f32 + angles[mid] as f32) / 2.0
    } else {
        angles[mid] as f32
    };

    // Invert angle so that the returned value can be used to straighten
    let inverted_angle = angle * -1.0;
    if config.verbose {
        println!("Computed deskew angle: {inverted_angle}");
    }

    Ok(inverted_angle)
}

/// Find the color that occurs the most in the specified image
pub(crate) fn find_dominant_color(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Rgba<u8> {
    let mut color_map: HashMap<Rgba<u8>, u32> = HashMap::new();
    for (_x, _y, pixel) in image.enumerate_pixels() {
        let counter = color_map.entry(*pixel).or_insert(0);
        *counter += 1;
    }
    // Sort items in the hash by value reverse
    let sorted_colors: Vec<_> = color_map
        .iter()
        .sorted_by(|a, b| a.1.cmp(b.1).reverse())
        .collect();
    let dominant_color_tuple = sorted_colors[0];
    *dominant_color_tuple.0
}

/// Return the dominant color in the image as hex #RRGGBB
pub(crate) fn find_dominant_color_hex(
    image_rgba: &image::ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> String {
    format!(
        "#{}",
        find_dominant_color(image_rgba)
            .to_rgb()
            .channels()
            .iter()
            .map(|f| format!("{:X}", f))
            .join(""),
    )
}
