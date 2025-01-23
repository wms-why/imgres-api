use std::io::BufWriter;

use bytes::Bytes;
use fast_image_resize::{images::Image, IntoImageView, Resizer};

use anyhow::Result;
use image::{
    codecs::{jpeg, png, webp},
    error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind},
    DynamicImage, ImageEncoder, ImageError, ImageFormat,
};

pub fn resize(
    src_image: &DynamicImage,
    target_type: image::ImageFormat,
    scale_factor: f32,
) -> Result<Bytes> {
    // Create container for data of destination image
    let target_width = (src_image.width() as f32 * scale_factor) as u32;
    let target_height = (src_image.height() as f32 * scale_factor) as u32;

    let mut dst_image = Image::new(target_width, target_height, src_image.pixel_type().unwrap());

    // Create Resizer instance and resize source image
    // into buffer of destination image
    let mut resizer = Resizer::new();
    resizer.resize(src_image, &mut dst_image, None).unwrap();

    // Write destination image as PNG-file
    let mut writer = BufWriter::new(Vec::new());
    match target_type {
        ImageFormat::Png => {
            png::PngEncoder::new(&mut writer)
                .write_image(
                    dst_image.buffer(),
                    target_width,
                    target_height,
                    src_image.color().into(),
                )
                .unwrap();
        }
        ImageFormat::Jpeg => {
            jpeg::JpegEncoder::new(&mut writer)
                .write_image(
                    dst_image.buffer(),
                    target_width,
                    target_height,
                    src_image.color().into(),
                )
                .unwrap();
        }
        ImageFormat::WebP => {
            webp::WebPEncoder::new_lossless(&mut writer)
                .write_image(
                    dst_image.buffer(),
                    target_width,
                    target_height,
                    src_image.color().into(),
                )
                .unwrap();
        }
        _ => Err(ImageError::Unsupported(
            UnsupportedError::from_format_and_kind(
                ImageFormatHint::Unknown,
                UnsupportedErrorKind::Format(ImageFormatHint::Name(format!("{target_type:?}"))),
            ),
        ))?,
    };

    // 将 writer  写入到本地的image.png文件中

    let bs = Bytes::from(writer.into_inner()?);

    Ok(bs)
}
