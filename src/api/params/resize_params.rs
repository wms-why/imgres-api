use std::io::Cursor;

use anyhow::{Error, Result};
use image::{DynamicImage, ImageReader};
use poem::web::Multipart;
use serde::Deserialize;

pub struct ImageResizeParams {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
    pub target_img_type: image::ImageFormat,
    pub sizes: Vec<Size>,
}

#[derive(Deserialize, Debug)]
pub struct Size {
    pub scale: f32,
    pub use_ai: bool,
}

impl ImageResizeParams {
    pub fn validate(&self) -> bool {
        if self.sizes.is_empty() {
            return false;
        }

        for ele in &self.sizes {
            if ele.scale == 0f32 {
                return false;
            }
        }

        true
    }

    pub async fn from_multipart(mut multipart: Multipart) -> Result<ImageResizeParams> {
        let mut image = Option::None;

        let mut target_img_type = image::ImageFormat::Png;
        let mut sizes = vec![];
        let mut width = 0;
        let mut height = 0;

        while let Ok(Some(field)) = multipart.next_field().await {
            let name = field.name();

            if name.is_none() {
                continue;
            }

            let name = name.unwrap();

            match name {
                "blob" => {
                    let content_type = field.content_type();
                    if let Some(content_type) = content_type {
                        let content_type = content_type.to_string();
                        let i = content_type.find("/");
                        if let Some(i) = i {
                            let image_type = image::ImageFormat::from_extension(
                                content_type.to_string().split_at(i + 1).1,
                            );
                            if let Some(image_type) = image_type {
                                target_img_type = image_type;
                            }
                        }
                    }

                    let b = field.bytes().await;

                    if b.is_err() {
                        return Err(Error::msg("upload image is empty"));
                    }

                    let blob = b.unwrap();
                    let cursor = Cursor::new(blob);
                    let pic = ImageReader::new(cursor).with_guessed_format();

                    if pic.is_err() {
                        return Err(Error::msg("upload image format is unkown"));
                    }

                    let pic = pic.unwrap();
                    let format = pic.format();
                    if format.is_none() {
                        return Err(Error::msg("upload image format is unkown"));
                    }

                    image = Some(pic.decode()?);
                }
                "sizes" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let ss = serde_json::from_str::<Vec<Size>>(&text.unwrap());

                        if ss.is_ok() {
                            sizes = ss.unwrap();
                        }
                    }
                }
                "width" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let w = text.unwrap().parse::<u32>();
                        if w.is_ok() {
                            width = w.unwrap();
                        }
                    }
                }
                "height" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let h = text.unwrap().parse::<u32>();
                        if h.is_ok() {
                            height = h.unwrap();
                        }
                    }
                }
                &_ => continue,
            }
        }

        if image.is_none() {
            return Err(Error::msg("upload image is empty"));
        }

        Ok(ImageResizeParams {
            image: image.unwrap(),
            target_img_type,
            sizes,
            width,
            height,
        })
    }
}
