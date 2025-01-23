use std::{
    borrow::Borrow,
    io::{Read, Write},
    path::Path,
};

use anyhow::Result;
use poem::{handler, http::StatusCode, web::Multipart, Body, Response};
use tracing::{error, warn};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

use crate::{
    api::{gen_known_err_response, params::resize_params::ImageResizeParams},
    core::{ai, algorithm, transform},
    db::{file::upload_temp, user::update_credits},
    extractor::auth_user::AuthUser,
};

use super::params::resize_params::Size;

#[handler]
pub async fn resize_free(mut multipart: Multipart) -> Response {
    let params = ImageResizeParams::from_multipart(multipart).await;

    if params.is_err() {
        error!("{:?}", params.err());
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("params init fail");
    }

    let params = params.unwrap();

    if !params.validate() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("params validate fail");
    }

    for ele in &params.sizes {
        if ele.use_ai {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("resize free not support use_ai");
        }
    }

    let r = handle(&params, None).await;

    if let Err(e) = r {
        error!("{:?}", e);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(e.to_string());
    }

    r.unwrap()
}

#[handler]
pub async fn resize(mut multipart: Multipart, user: AuthUser) -> Response {
    let params = ImageResizeParams::from_multipart(multipart).await;

    if params.is_err() {
        error!("{:?}", params.err());
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("params init fail");
    }

    let params = params.unwrap();

    if !params.validate() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("params validate fail");
    }

    let r = handle(&params, Some(user)).await;

    if let Err(e) = r {
        error!("{:?}", e);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish();
    }

    r.unwrap()
}
async fn handle(params: &ImageResizeParams, user: Option<AuthUser>) -> Result<Response> {
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

    let mut use_ai_count = 0;

    for ele in &params.sizes {
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

            use_ai_count += 1;
        }
    }

    if use_ai_count > 0 {
        if let Some(ref user) = user {
            if user.user.credit <= 0 {
                return Ok(gen_known_err_response("credits is not enough"));
            }
        }
    }
    for ele in &params.sizes {
        let buf = if ele.use_ai {
            // use ai
            ai::resize(img_url.as_ref().unwrap(), ele.scale).await?
        } else {
            // use algorithm
            algorithm::resize(&params.image, params.target_img_type, ele.scale)?
        };

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

    if user.is_some() && use_ai_count > 0 {
        let user = user.unwrap();
        update_credits(user.user, -use_ai_count).await?;
    }

    Ok(Response::builder()
        .content_type("application/octet-stream")
        .body(Body::from_vec(buffer)))
}

fn generate_file_name(width: u32, height: u32, size: &Size, ext: &str) -> String {
    format!(
        "@{}x{}.{}",
        (width as f32 * size.scale) as u32,
        (height as f32 * size.scale) as u32,
        ext
    )
}
