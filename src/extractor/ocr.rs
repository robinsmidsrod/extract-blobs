use std::{fs, path::Path};

use crate::Result;
use leptess::{LepTess, Variable};

pub(crate) struct TextExtractor {
    lt: LepTess,
}

impl TextExtractor {
    /// Contruct an instance from our parameters
    pub(crate) fn new(
        language: &str,
        page_segmentation_mode: &str,
        tessdata: &Path,
    ) -> Result<Self> {
        // Create an instance of LepTess we can use to run OCR on images
        let mut lt = LepTess::new(Some(&tessdata.to_string_lossy()), language)?;
        lt.set_variable(Variable::TesseditPagesegMode, page_segmentation_mode)?;
        lt.set_variable(Variable::PreserveInterwordSpaces, "1")?;

        Ok(Self { lt })
    }
    /// Extract text using OCR from specified image file
    pub(crate) fn extract_text(&mut self, image_filename: &Path) -> Result<String> {
        self.lt.set_image(image_filename)?;
        Ok(self.lt.get_utf8_text()?)
    }
    /// Save text using OCR from specified image file into specified text file
    /// Returns text for further processing
    pub(crate) fn extract_and_save_text(
        &mut self,
        input_image_filename: &Path,
        output_text_filename: &Path,
    ) -> Result<String> {
        let text = self.extract_text(input_image_filename)?;
        fs::write(output_text_filename, &text)?;
        println!(
            "{}: saved OCR text - {} bytes",
            output_text_filename.display(),
            &text.len()
        );
        Ok(text)
    }
}
