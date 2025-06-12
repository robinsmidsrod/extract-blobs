use std::path::PathBuf;

use image::ImageBuffer;
use image::Luma;
use imageproc::contours::Contour;
use imageproc::hough::LineDetectionOptions;
use imageproc::point::Point;
use imageproc::rect::Rect;

use itertools::Itertools; // for sorted() iterator function

/// Find edges in the image and extract a list of points
pub(crate) fn find_contour_points(image: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<Point<u32>> {
    let mut contours: Vec<Contour<u32>> = imageproc::contours::find_contours(image);
    let points = match contours.pop() {
        Some(contour) => contour.points,
        None => vec![],
    };
    points
}

/// Compute bounding box from grayscale image, any non-black color is considered part of the bounding box
// TODO: Just use enumerate_pixels() to find top, left, right and bottom-most non-zero pixels
pub(crate) fn compute_bounding_box(image: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Rect {
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

/// Compute the deskew angle for a rectangular blob (max 10 degree off-axis allowed)
pub(crate) fn compute_deskew_angle_for_rectangle(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    base_path: &PathBuf,
    index: u32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let mut image = imageproc::edges::canny(&image, 1.0, 1.0);
    crate::io::save_luma_image_as(&image, base_path, &format!("{index}-canny")[..])?;

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
    crate::io::save_luma_image_as(&image, base_path, &format!("{index}-canny-lines")[..])?;

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

/// Compute skew angle, bounding box and rotation center from luma image
///
/// There should only be one blob in the specified image
// TODO: Get rid of this entirely, and create a new function that finds rotation center based on bounding box function instead
pub(crate) fn compute_skew_angle_and_rotation_center(
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
