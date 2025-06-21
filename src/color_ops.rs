use color::{AlphaColor, ParseError};
use image::Rgba;
use palette::{FromColor, Lab};

/// Parse a string into a color, with format like this #RRGGBB
pub(crate) fn parse_color(color: &str) -> Result<Rgba<u8>, ParseError> {
    let color = color::parse_color(color)?;
    let color: AlphaColor<color::Srgb> = color.to_alpha_color();
    let color = color.to_rgba8();
    let color = Rgba(color.to_u8_array());
    Ok(color)
}

/// Figure out how similar two colors are based on euclidean distance in Lab colorspace
pub(crate) fn color_similarity(a: &palette::Srgb<f32>, b: &palette::Srgb<f32>) -> f32 {
    // Convert colors to Lab space for better perceptual similarity
    let lab_a = Lab::from_color(*a);
    let lab_b = Lab::from_color(*b);

    // Calculate Euclidean distance in Lab space
    let delta_e =
        (lab_a.l - lab_b.l).powi(2) + (lab_a.a - lab_b.a).powi(2) + (lab_a.b - lab_b.b).powi(2);
    let diff = delta_e.sqrt();
    // println!("Colors {:?} and {:?} has a difference of {}", a, b, diff);
    diff
}

/// Convert from image::Rgba color to palette::Srgb color
pub(crate) fn image_rgba_to_palette_srgb(color: &image::Rgba<u8>) -> palette::rgb::Rgb {
    palette::Srgb::new(
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    )
}
