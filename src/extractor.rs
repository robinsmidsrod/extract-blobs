use std::{fs, path::PathBuf};

use image::{Luma, Rgba};
use imageproc::{distance_transform::Norm, geometric_transformations::Interpolation};
use leptess::LepTess;

use crate::{Args, Result, extractor::io::Dpi};

mod alpha_channel;
mod detection;
mod drawing;
mod extraction;
mod io;

pub struct BlobExtractor {
    file: PathBuf,
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
        let base_path = file.parent().unwrap().join(file.file_stem().unwrap());
        Self {
            file,
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
    pub fn process(self) -> Result<()> {
        // Open image and maybe get pixel density in dots per inch
        let (image, dpi) = io::open_image(&self.file)?;

        // Decide which DPI to use for output images
        let dpi = self.decide_output_dpi(dpi);
        if self.verbose {
            println!("{}: using DPI {:?}", self.file.display(), dpi);
        }

        let mut image_rgba = image.to_rgba8();

        // Detect dominant color in image
        if self.verbose {
            let color = detection::find_dominant_color_hex(&image_rgba);
            println!("{}: dominant color is {}", self.file.display(), color);
        }

        self.remove_chroma_key_color_from_image(&mut image_rgba, &dpi)?;
        let image_mask = self.cleanup_and_extract_image_mask(&mut image_rgba, &dpi)?;

        // Extract individual blobs from the alpha channel
        let blobs = extraction::extract_blobs(&image_mask);
        println!("{}: found {} blobs", self.file.display(), blobs.len());
        for (index, blob) in blobs.iter().enumerate() {
            let blob_number = index as u32 + 1;
            self.process_blob(blob_number, blob, &image_rgba, &dpi)?;
        }

        Ok(())
    }

    /// Decide image output DPI from detected input image metadata
    fn decide_output_dpi(&self, dpi: Option<Dpi>) -> Dpi {
        match dpi {
            Some(dpi) => {
                if self.verbose {
                    println!("{}: detected DPI is {:?}", self.file.display(), dpi);
                }
                match self.ignore_detected_dpi {
                    true => Dpi {
                        x: self.dpi,
                        y: self.dpi,
                    },
                    false => dpi,
                }
            }
            None => {
                if self.verbose {
                    println!("{}: unable to detect DPI", self.file.display());
                }
                Dpi {
                    x: self.dpi,
                    y: self.dpi,
                }
            }
        }
    }

    /// Remove color matching chroma key color by drawing a border and floodfilling with fuzz
    fn remove_chroma_key_color_from_image(
        &self,
        image: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
        dpi: &Dpi,
    ) -> Result<()> {
        let width = image.width();
        let height = image.height();
        drawing::draw_border(
            image,
            self.chroma_key_color,
            0,
            0,
            width,
            height,
            self.border_thickness,
        );
        if self.save_intermediary_images {
            io::save_rgba_image_as(image, &self.base_path, "a-border", dpi)?;
        }
        drawing::flood_fill(
            image,
            0,
            0,
            self.chroma_key_color,
            self.floodfill_color,
            self.floodfill_fuzz,
        );
        if self.save_intermediary_images {
            io::save_rgba_image_as(image, &self.base_path, "b-floodfilled", dpi)?;
        }
        Ok(())
    }

    /// Clean up alpha channel in color image and extract it
    fn cleanup_and_extract_image_mask(
        &self,
        image: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
        dpi: &Dpi,
    ) -> Result<image::ImageBuffer<Luma<u8>, Vec<u8>>> {
        let mut image_mask = alpha_channel::extract(image);
        if self.save_intermediary_images {
            io::save_luma_image_as(&image_mask, &self.base_path, "c-mask")?;
        }
        imageproc::morphology::erode_mut(&mut image_mask, Norm::L1, self.trim_edges);
        imageproc::morphology::dilate_mut(&mut image_mask, Norm::L1, self.grow_edges);
        if self.save_intermediary_images {
            io::save_luma_image_as(&image_mask, &self.base_path, "d-mask-cleaned")?;
        }
        alpha_channel::replace(image, &image_mask);
        if self.save_intermediary_images {
            io::save_rgba_image_as(&*image, &self.base_path, "e-with-mask", dpi)?;
        }
        Ok(image_mask)
    }

    /// Process a single blob from the image mask
    fn process_blob(
        &self,
        blob_number: u32,
        blob: &image::ImageBuffer<Luma<u8>, Vec<u8>>,
        image: &image::ImageBuffer<Rgba<u8>, Vec<u8>>,
        dpi: &Dpi,
    ) -> Result<()> {
        if self.save_intermediary_images {
            io::save_luma_image_as(blob, &self.base_path, &format!("mask-{blob_number}-a")[..])?;
        }
        let bounding_box = detection::compute_bounding_box(blob, self);
        let center = detection::compute_center_from_rectangle(&bounding_box, self);
        let deskew_angle = detection::compute_deskew_angle_for_rectangle(
            blob,
            self,
            &self.base_path,
            blob_number,
        )?;
        let black_luma = Luma([0u8]);
        let blob = imageproc::geometric_transformations::rotate(
            blob,
            point_to_tuple(center),
            angle_to_radians(deskew_angle),
            Interpolation::Bicubic,
            black_luma,
        );
        let blob = imageproc::filter::gaussian_blur_f32(&blob, self.blur_edge_factor);
        if self.save_intermediary_images {
            io::save_luma_image_as(
                &blob,
                &self.base_path,
                &format!("mask-{blob_number}-d-deskewed")[..],
            )?;
        }
        let black_rgba = Rgba([0, 0, 0, 0]);
        let mut blob_rgba = imageproc::geometric_transformations::rotate(
            image,
            point_to_tuple(center),
            angle_to_radians(deskew_angle),
            Interpolation::Bicubic,
            black_rgba,
        );
        alpha_channel::replace(&mut blob_rgba, &blob);
        let bounding_box = detection::compute_bounding_box(&blob, self);
        let blob_rgba = image::imageops::crop_imm(
            &blob_rgba,
            bounding_box.left() as u32,
            bounding_box.top() as u32,
            bounding_box.width(),
            bounding_box.height(),
        )
        .to_image();
        io::save_rgba_image_as(
            &blob_rgba,
            &self.base_path,
            &format!("{blob_number}")[..],
            dpi,
        )?;
        // Extract text from final image using OCR
        let mut lt = LepTess::new(Some(&self.tessdata.to_string_lossy()), &self.ocr_language)?;
        lt.set_variable(leptess::Variable::TesseditPagesegMode, &self.ocr_psm)?;
        lt.set_variable(leptess::Variable::PreserveInterwordSpaces, "1")?;
        let img_filename = format!("{}-{}.{}", self.base_path.display(), blob_number, "png");
        let text_filename = format!("{}-{}.{}", self.base_path.display(), blob_number, "txt");
        lt.set_image(&img_filename)?;
        let text = lt.get_utf8_text()?;
        fs::write(&text_filename, &text)?;
        println!("{}: saved OCR text - {} bytes", &text_filename, &text.len());
        Ok(())
    }
}

fn point_to_tuple(center: imageproc::point::Point<u32>) -> (f32, f32) {
    (center.x as f32, center.y as f32)
}
fn angle_to_radians(angle: f32) -> f32 {
    angle * std::f32::consts::PI / 180.0
}
