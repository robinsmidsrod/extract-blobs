use image::ImageBuffer;
use image::Luma;
use image::Rgba;

/// Extract the alpha channel of a color image into a grayscale image
pub(crate) fn extract(
    image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let width = image.width();
    let height = image.height();
    // Create a new buffer to store the alpha channel
    let mut gray_image = ImageBuffer::new(width, height);
    // Iterate over each pixel in the original image
    for (x, y, pixel) in image.enumerate_pixels() {
        // Extract the alpha value from the Rgba pixel
        let alpha_value = pixel[3];
        // Set the corresponding pixel in the gray image
        *gray_image.get_pixel_mut(x, y) = Luma([alpha_value]);
    }
    gray_image
}

/// Replace the alpha channel of the specifid color image with the specified grayscale image
pub(crate) fn replace(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    gray_image: &ImageBuffer<Luma<u8>, Vec<u8>>,
) {
    // Iterate over each pixel in the original image
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        // Get the pixel from the gray image
        let gray_pixel = gray_image.get_pixel(x, y);
        // Set the alpha value of the original image to the value of the gray pixel
        *pixel = Rgba([pixel[0], pixel[1], pixel[2], gray_pixel[0]]);
    }
}
