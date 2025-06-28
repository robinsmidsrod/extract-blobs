use image::ImageBuffer;
use image::Luma;
use imageproc::region_labelling::Connectivity;

/// Split gray image into a list of gray images, with each blob by itself
pub(crate) fn extract_blobs(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
    let (width, height) = image.dimensions();
    let image_components =
        imageproc::region_labelling::connected_components(image, Connectivity::Four, Luma([0u8]));
    let mut blobs: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = Vec::new();
    for (x, y, pixel) in image_components.enumerate_pixels() {
        let index = pixel[0] as usize;
        // Skip background color
        if index == 0 {
            continue;
        }
        let index = index - 1;
        let blob_option = blobs.get_mut(index);
        if blob_option.is_none() {
            let blob = ImageBuffer::new(width, height);
            blobs.push(blob);
        };
        let blob_option = blobs.get_mut(index);
        if let Some(blob) = blob_option {
            blob.put_pixel(x, y, Luma([255u8]));
        };
    }
    blobs
}
