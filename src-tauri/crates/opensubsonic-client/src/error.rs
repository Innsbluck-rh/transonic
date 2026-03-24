use reqwest::StatusCode;

use crate::envelope::ResponseMeta;

#[derive(Debug)]
pub enum ApiError {
    InvalidUrl(String),
    ClientBuild(String),
    Transport(reqwest::Error),
    HttpStatus {
        status: StatusCode,
        body_preview: Option<String>,
    },
    Decode {
        message: String,
        body_preview: Option<String>,
    },
    Api {
        code: u32,
        message: Option<String>,
        help_url: Option<String>,
        meta: ResponseMeta,
    },
    UnsupportedExtension {
        extension: String,
    },
    Protocol(String),
}
