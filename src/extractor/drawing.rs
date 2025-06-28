use std::collections::HashSet;

use image::ImageBuffer;
use image::Rgba;
use imageproc::rect::Rect;

mod color_ops;

/// Draws a border into the specified image buffer with the specified color and thickness
/// NB: The image canvas is not made bigger
pub(crate) fn draw_border(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    thickness: u32,
) {
    for offset in 0..thickness {
        let border = Rect::at(x + offset as i32, y + offset as i32)
            .of_size(width - offset * 2, height - offset * 2);
        imageproc::drawing::draw_hollow_rect_mut(image, border, color);
    }
}

/// Flood fill the replacemnt color where the target color fuzzed with tolerance is found, starting at coordinate
pub(crate) fn flood_fill(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    target_color: Rgba<u8>,
    replacement_color: Rgba<u8>,
    fuzz: f32,
) {
    let (width, height) = image.dimensions();
    let target_color = color_ops::image_rgba_to_palette_srgb(&target_color);
    let mut stack = vec![(x, y)];
    let mut visisted = HashSet::new();
    while let Some((cx, cy)) = stack.pop() {
        // println!("Checking pixel at ({cx}, {cy}): stack is {stack:?}");
        if cx < 0
            || cx >= width as i32
            || cy < 0
            || cy >= height as i32
            || !visisted.insert((cx, cy))
        {
            continue;
        }

        let pixel = image.get_pixel(cx as u32, cy as u32);
        let current_color = color_ops::image_rgba_to_palette_srgb(pixel);

        if color_ops::color_similarity(&current_color, &target_color) > fuzz {
            continue;
        }

        image.put_pixel(cx as u32, cy as u32, replacement_color);

        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        for (dx, dy) in directions {
            stack.push((cx + dx, cy + dy));
        }
    }
}
