use std::convert::TryFrom;

use actix_web::{http, HttpRequest, HttpResponse};
use columnq::ColumnQ;
use log::info;

use crate::config::Config;
use crate::encoding;
use crate::error::ApiErrResp;

pub struct HandlerContext {
    pub cq: ColumnQ,
    // TODO: store pre serialized schema in handler context
}

impl HandlerContext {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let mut cq = ColumnQ::new();

        if config.tables.is_empty() {
            anyhow::bail!("No table found in tables config");
        }

        for t in config.tables.iter() {
            info!("loading `{}` as table `{}`", t.uri, t.name);
            cq.load_table(t).await?;
            info!("registered `{}` as table `{}`", t.uri, t.name);
        }

        Ok(Self { cq })
    }
}

pub fn encode_type_from_req(req: HttpRequest) -> Result<encoding::ContentType, ApiErrResp> {
    match req.headers().get(http::header::ACCEPT) {
        None => Ok(encoding::ContentType::Json),
        Some(hdr_value) => {
            encoding::ContentType::try_from(hdr_value.as_bytes()).map_err(|_| ApiErrResp {
                code: http::StatusCode::BAD_REQUEST,
                error: "unsupported_content_type".to_string(),
                message: format!("{:?} is not a supported response content type", hdr_value),
            })
        }
    }
}

pub fn encode_record_batches(
    content_type: encoding::ContentType,
    batches: &[arrow::record_batch::RecordBatch],
) -> Result<HttpResponse, ApiErrResp> {
    let mut resp = HttpResponse::Ok();
    let builder = resp.content_type(content_type.to_str());

    match content_type {
        encoding::ContentType::Json => {
            let payload = encoding::json::record_batches_to_bytes(batches)
                .map_err(ApiErrResp::json_serialization)?;
            Ok(builder.body(payload))
        }
        encoding::ContentType::ArrowStream => {
            let payload = encoding::arrow::record_batches_to_stream_bytes(batches)
                .map_err(ApiErrResp::arrow_stream_serialization)?;
            Ok(builder.body(payload))
        }
    }
}

pub mod graphql;
pub mod rest;
pub mod routes;
pub mod schema;
pub mod sql;

pub use routes::register_app_routes;
