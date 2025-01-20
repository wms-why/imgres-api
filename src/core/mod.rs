use std::io::Cursor;

use anyhow::Error;
use anyhow::Result;
use image::DynamicImage;
pub mod ai;
pub mod algorithm;

pub static SUPPORT_IMAGE_FORMATS: [image::ImageFormat; 3] = [
    image::ImageFormat::Png,
    image::ImageFormat::Jpeg,
    image::ImageFormat::WebP,
];

pub fn transform(image: &DynamicImage, target_format: image::ImageFormat) -> Result<Vec<u8>> {
    if !SUPPORT_IMAGE_FORMATS.contains(&target_format) {
        return Err(Error::msg(format!(
            "image format {} is not support",
            target_format.to_mime_type()
        )));
    }

    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, target_format)?;

    Ok(buffer.into_inner())
}
