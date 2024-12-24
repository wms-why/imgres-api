use std::{
    borrow::Borrow,
    io::{Read, Write},
    path::Path,
};

use anyhow::Result;
use image::{load_from_memory_with_format, DynamicImage};
use poem::{
    handler, http::StatusCode, session::Session, web::Multipart, Body, IntoResponse, Response,
};
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

use crate::{
    core::{ai, algorithm},
    db::file::upload_temp,
};
struct ImageResizeParams {
    blob: Vec<u8>,
    width: u32,
    height: u32,
    img_type: image::ImageFormat,
    sizes: Vec<Size>,
}

#[derive(Deserialize, Debug)]
struct Size {
    scale: f32,
    use_ai: bool,
}

impl ImageResizeParams {
    pub fn validate(&self) -> bool {
        if self.blob.len() == 0 || self.sizes.len() == 0 {
            return false;
        }

        for ele in &self.sizes {
            if ele.scale == 0f32 {
                return false;
            }
        }

        return true;
    }

    pub async fn from_multipart(mut multipart: Multipart) -> ImageResizeParams {
        let mut params = ImageResizeParams {
            blob: vec![],
            img_type: image::ImageFormat::Png,
            sizes: vec![],
            width: 0,
            height: 0,
        };

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
                                params.img_type = image_type;
                            }
                        }
                    }

                    let b = field.bytes().await;

                    if b.is_err() {
                        continue;
                    }

                    params.blob = b.unwrap().to_vec();
                }
                "sizes" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let ss = serde_json::from_str::<Vec<Size>>(&text.unwrap());

                        if ss.is_ok() {
                            params.sizes = ss.unwrap();
                        }
                    }
                }
                "width" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let w = text.unwrap().parse::<u32>();
                        if w.is_ok() {
                            params.width = w.unwrap();
                        }
                    }
                }
                "height" => {
                    let text = field.text().await;
                    if text.is_ok() {
                        let h = text.unwrap().parse::<u32>();
                        if h.is_ok() {
                            params.height = h.unwrap();
                        }
                    }
                }
                &_ => continue,
            }
        }

        return params;
    }
}

#[handler]
pub async fn resize(mut multipart: Multipart, _session: &Session) -> impl IntoResponse {
    let params = ImageResizeParams::from_multipart(multipart).await;

    if !params.validate() {
        return Response::builder().status(StatusCode::BAD_REQUEST).finish();
    }

    let r = handle(&params).await;

    if r.is_err() {
        error!("{}", r.err().unwrap().root_cause().to_string());
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish();
    } else {
        let path = r.unwrap();
        let path = std::path::Path::new(&(path.path));
        let file = std::fs::File::open(path);

        if file.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(file.err().unwrap().to_string());
        }

        let mut buffer = Vec::new();
        let r = file.unwrap().read_to_end(&mut buffer);

        if r.is_err() {
            error!("read handled file error {:?}", r.err());
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .finish();
        }
        let r = Response::builder()
            .content_type("application/octet-stream")
            .body(Body::from_vec(buffer));

        return r;
    }
}
async fn handle(params: &ImageResizeParams) -> Result<ZipFileWrapper> {
    let temp_name = format!("{}.zip", Uuid::new_v4().to_string());

    let path: &Path = std::path::Path::new(&temp_name);
    let file = std::fs::File::create(path)?;

    let mut zip = zip::ZipWriter::new(file);

    let result = ZipFileWrapper {
        path: temp_name.to_string(),
    };

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut img_url: Option<String> = Option::None;
    let mut img_obj: Option<DynamicImage> = Option::None;

    for ele in &params.sizes {
        let buf;

        if ele.use_ai {
            if img_url.is_none() {
                let filename = format!(
                    "{}.{}",
                    uuid::Uuid::new_v4().to_string(),
                    params.img_type.extensions_str()[0]
                );

                let r = upload_temp(params.blob.clone(), &filename).await?;

                img_url = Some(r);
            }

            // use ai
            buf = ai::AiScaleUp
                .resize(img_url.as_ref().unwrap(), ele.scale)
                .await?;
        } else {
            if img_obj.as_ref().is_none() {
                let src_image = load_from_memory_with_format(&params.blob, params.img_type)?;
                img_obj = Some(src_image);
            }

            // use algorithm
            buf = algorithm::AlgorithmResize.resize(
                &(img_obj.as_ref().unwrap()),
                params.img_type,
                ele.scale,
            )?;
        }

        let ext = params.img_type.extensions_str()[0];
        zip.start_file(
            generate_file_name(params.width, params.height, &ele, ext),
            options,
        )?;
        zip.write_all(buf.borrow())?;
    }

    zip.finish()?;

    return Ok(result);
}

fn generate_file_name(width: u32, height: u32, size: &Size, ext: &str) -> String {
    return format!(
        "@{}_{}.{}",
        (width as f32 * size.scale) as u32,
        (height as f32 * size.scale) as u32,
        ext
    );
}

struct ZipFileWrapper {
    path: String,
}

impl Drop for ZipFileWrapper {
    fn drop(&mut self) {
        let path = std::path::Path::new(&self.path);
        let re = std::fs::remove_file(path);
        if re.is_err() {
            warn!("{:?}", re);
        }
    }
}
