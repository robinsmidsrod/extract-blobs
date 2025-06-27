use std::{
    // ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use image::Pixel; // for to_rgb() method
use image::{Luma, Rgba};
use imageproc::{distance_transform::Norm, geometric_transformations::Interpolation};
use itertools::Itertools; // for join() iterator method
use leptess::LepTess;

use crate::Args;

mod alpha_channel;
mod detection;
mod drawing;
mod extraction;
mod io;

pub struct BlobExtractor {
    file: PathBuf,
    // base_dir: PathBuf,
    // base_filename: OsString,
    base_path: PathBuf,
    chroma_key_color: Rgba<u8>,
    floodfill_fuzz: f32,
    trim_edges: u8,
    grow_edges: u8,
    floodfill_color: Rgba<u8>,
    border_thickness: u32,
    blur_edge_factor: f32,
    pub min_pixels_touching_line: u32,
    pub max_lines: usize,
    pub max_blob_rotation: f32,
    pub save_intermediary_images: bool,
    pub verbose: bool,
    dpi: u32,
    ignore_detected_dpi: bool,
    ocr_language: String,
    tessdata: PathBuf,
    ocr_psm: String,
}

impl BlobExtractor {
    pub fn new(file: PathBuf, args: &Args) -> Self {
        // Figure out path stuff
        let base_dir = Path::new(&file).parent().unwrap().to_owned();
        let base_filename = Path::new(&file).file_stem().unwrap().to_owned();
        let base_path = base_dir.join(&base_filename);
        Self {
            file: file,
            // base_dir,
            // base_filename,
            base_path,
            chroma_key_color: args.chroma_key_color,
            floodfill_fuzz: args.floodfill_fuzz,
            trim_edges: args.trim_edges,
            grow_edges: args.grow_edges,
            floodfill_color: Rgba([0, 0, 0, 0]), // transparent
            border_thickness: 1,
            blur_edge_factor: args.blur_edge_factor,
            min_pixels_touching_line: args.min_pixels_touching_line,
            max_lines: args.max_lines,
            max_blob_rotation: args.max_blob_rotation,
            dpi: args.dpi,
            save_intermediary_images: args.save_intermediary_images,
            verbose: args.verbose,
            ignore_detected_dpi: args.ignore_detected_dpi,
            ocr_language: args.ocr_language.to_owned(),
            ocr_psm: args.ocr_psm.to_owned(),
            tessdata: args.tessdata.to_owned(),
        }
    }
    pub fn process(self) -> Result<(), Box<dyn std::error::Error>> {
        // Open image and maybe get pixel density in dots per inch
        let (image, maybe_dpi) = io::open_image(&self.file)?;

        // Decide which DPI to use for output images
        let dpi = match maybe_dpi {
            Some(dpi) => {
                if self.verbose {
                    println!("{}: detected DPI is {:?}", self.file.display(), dpi);
                }
                match self.ignore_detected_dpi {
                    true => (self.dpi, self.dpi),
                    false => dpi,
                }
            }
            None => {
                if self.verbose {
                    println!("{}: unable to detect DPI", self.file.display());
                }
                (self.dpi, self.dpi)
            }
        };
        if self.verbose {
            println!("{}: using DPI {:?}", self.file.display(), dpi);
        }

        let width = image.width();
        let height = image.height();

        let mut image_rgba = image.to_rgba8();

        // Detect dominant color in image
        if self.verbose {
            let dominant_color_hex = find_dominant_color_hex(&image_rgba);
            println!(
                "{}: dominant color is #{}",
                self.file.display(),
                dominant_color_hex
            );
        }

        // Draw a thin border on color image with chroma key color
        drawing::draw_border(
            &mut image_rgba,
            self.chroma_key_color,
            0,
            0,
            width,
            height,
            self.border_thickness,
        );
        if self.save_intermediary_images {
            io::save_rgba_image_as(&image_rgba, &self.base_path, "a-border", dpi)?;
        }

        // Floodfill color image with chroma key color, making it transparent, with a fuzz factor
        drawing::flood_fill(
            &mut image_rgba,
            0,
            0,
            self.chroma_key_color,
            self.floodfill_color,
            self.floodfill_fuzz,
        );
        if self.save_intermediary_images {
            io::save_rgba_image_as(&image_rgba, &self.base_path, "b-floodfilled", dpi)?;
        }

        // Extract alpha channel from color image so we can clean it up
        let mut image_mask = alpha_channel::extract(&image_rgba);
        if self.save_intermediary_images {
            io::save_luma_image_as(&image_mask, &self.base_path, "c-mask")?;
        }

        // Remove specs and dust from alpha channel, trim/grow outer edges slightly
        imageproc::morphology::erode_mut(&mut image_mask, Norm::L1, self.trim_edges);
        imageproc::morphology::dilate_mut(&mut image_mask, Norm::L1, self.grow_edges);
        if self.save_intermediary_images {
            io::save_luma_image_as(&image_mask, &self.base_path, "d-mask-cleaned")?;
        }

        // Replace alpha channel in the color image with the cleaned one
        alpha_channel::replace(&mut image_rgba, &image_mask);
        if self.save_intermediary_images {
            io::save_rgba_image_as(&image_rgba, &self.base_path, "e-with-mask", dpi)?;
        }

        // Extract individual blobs from the alpha channel
        let blobs = extraction::extract_blobs(&image_mask);
        println!("{}: found {} blobs", self.file.display(), blobs.len());
        for (index, blob) in blobs.iter().enumerate() {
            let blob_number = index as u32 + 1;
            if self.save_intermediary_images {
                io::save_luma_image_as(
                    &blob,
                    &self.base_path,
                    &format!("mask-{blob_number}-a")[..],
                )?;
            }

            // Compute values needed for image rotation
            let bounding_box = detection::compute_bounding_box(&blob, &self);
            let center = detection::compute_center_from_rectangle(&bounding_box, &self);
            let deskew_angle = detection::compute_deskew_angle_for_rectangle(
                &blob,
                &self,
                &self.base_path,
                blob_number,
            )?;

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
            let blob = imageproc::filter::gaussian_blur_f32(&blob, self.blur_edge_factor);
            if self.save_intermediary_images {
                io::save_luma_image_as(
                    &blob,
                    &self.base_path,
                    &format!("mask-{blob_number}-d-deskewed")[..],
                )?;
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
            let bounding_box = detection::compute_bounding_box(&blob, &self);
            let blob_rgba = image::imageops::crop_imm(
                &blob_rgba,
                bounding_box.left() as u32,
                bounding_box.top() as u32,
                bounding_box.width(),
                bounding_box.height(),
            )
            .to_image();

            // Save final blob color image
            io::save_rgba_image_as(
                &blob_rgba,
                &self.base_path,
                &format!("{blob_number}")[..],
                dpi,
            )?;

            // Extract text from image using Tesseract OCR
            let mut lt = LepTess::new(Some(&self.tessdata.to_string_lossy()), &self.ocr_language)?;
            // https://houqp.github.io/leptess/leptess/enum.Variable.html#variant.TesseditPagesegMode
            lt.set_variable(leptess::Variable::TesseditPagesegMode, &self.ocr_psm)?;
            lt.set_variable( leptess::Variable::PreserveInterwordSpaces, "1")?;
            let img_filename = format!("{}-{}.{}", self.base_path.display(), blob_number, "png");
            let text_filename = format!("{}-{}.{}", self.base_path.display(), blob_number, "txt");
            lt.set_image(&img_filename)?;
            let text = lt.get_utf8_text()?;
            fs::write(&text_filename, &text)?;
            println!("{}: saved OCR text - {} bytes", &text_filename, &text.len());
        }

        Ok(())
    }
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
