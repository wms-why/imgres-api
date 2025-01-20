use std::{
    borrow::Borrow,
    io::{Cursor, Read, Write},
    path::Path,
};

use anyhow::{Error, Result};
use image::{DynamicImage, ImageReader};
use poem::{handler, http::StatusCode, web::Multipart, Body, Request, Response};
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

use crate::{
    core::{ai, algorithm, transform},
    db::file::upload_temp,
};
struct ImageResizeParams {
    image: DynamicImage,
    width: u32,
    height: u32,
    target_img_type: image::ImageFormat,
    sizes: Vec<Size>,
}

#[derive(Deserialize, Debug)]
struct Size {
    scale: f32,
    use_ai: bool,
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

#[handler]
pub async fn resize_free(mut multipart: Multipart) -> Response {
    let params = ImageResizeParams::from_multipart(multipart).await;

    if params.is_err() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(params.err().unwrap().to_string());
    }

    let params = params.unwrap();

    if !params.validate() {
        return Response::builder().status(StatusCode::BAD_REQUEST).finish();
    }

    for ele in &params.sizes {
        if ele.use_ai {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .finish();
        }
    }

    let r = handle(&params).await;

    if let Err(e) = r {
        error!("{:?}", e);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish();
    }

    r.unwrap()
}

#[handler]
pub async fn resize(mut multipart: Multipart, _req: &Request) -> Response {
    let params = ImageResizeParams::from_multipart(multipart).await;

    if params.is_err() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(params.err().unwrap().to_string());
    }

    let params = params.unwrap();

    if !params.validate() {
        return Response::builder().status(StatusCode::BAD_REQUEST).finish();
    }

    // for ele in &params.sizes {
    //     if ele.use_ai {
    //         let user = get_current_user(req);
    //         if user.is_none() {
    //             return check_login_error().into_response();
    //         }
    //         break;
    //     }
    // }

    let r = handle(&params).await;

    if let Err(e) = r {
        error!("{:?}", e);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish();
    }

    r.unwrap()
}
async fn handle(params: &ImageResizeParams) -> Result<Response> {
    let temp_name = format!("{}.zip", Uuid::new_v4());

    let path: &Path = std::path::Path::new(&temp_name);
    let file = std::fs::File::create(path)?;

    defer::defer! {
        std::fs::remove_file(path).unwrap_or_else(|e| {
            warn!("remove file error: {:?}", e);
        })
    }

    let mut zip = zip::ZipWriter::new(&file);

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut img_url: Option<String> = Option::None;

    for ele in &params.sizes {
        let buf;

        if ele.use_ai {
            if img_url.is_none() {
                let filename = format!(
                    "{}.{}",
                    uuid::Uuid::new_v4(),
                    params.target_img_type.extensions_str()[0]
                );

                let buffer = transform(&params.image, params.target_img_type)?;

                let r = upload_temp(buffer, &filename).await?;

                img_url = Some(r);
            }

            // use ai
            buf = ai::AiScaleUp
                .resize(img_url.as_ref().unwrap(), ele.scale)
                .await?;
        } else {
            // use algorithm
            buf = algorithm::AlgorithmResize.resize(
                &params.image,
                params.target_img_type,
                ele.scale,
            )?;
        }

        let ext = params.target_img_type.extensions_str()[0];
        zip.start_file(
            generate_file_name(params.width, params.height, ele, ext),
            options,
        )?;
        zip.write_all(buf.borrow())?;
    }

    zip.finish()?;

    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(Response::builder()
        .content_type("application/octet-stream")
        .body(Body::from_vec(buffer)))
}

fn generate_file_name(width: u32, height: u32, size: &Size, ext: &str) -> String {
    format!(
        "@{}_{}.{}",
        (width as f32 * size.scale) as u32,
        (height as f32 * size.scale) as u32,
        ext
    )
}
