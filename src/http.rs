//! HTTP response helpers and the error model.
//!
//! `handle_http_request` must never return `Err` for domain failures (the host
//! turns that into a plain-text 500), so every error becomes a JSON response
//! here. Error shape follows the GameAP plugin convention:
//! `{"code": "UPPER_SNAKE", "message": "..."}`.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;
use serde::Serialize;

use crate::host_api::HostApiError;

pub type ApiResult = Result<pb::HttpResponse, ApiError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: i32,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: i32, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, "INVALID_INPUT", message)
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(404, code, message)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(409, code, message)
    }

    pub fn unprocessable(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(422, code, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(500, "INTERNAL_ERROR", message)
    }

    pub fn into_response(self) -> pb::HttpResponse {
        #[derive(Serialize)]
        struct ErrorBody<'a> {
            code: &'a str,
            message: &'a str,
        }
        json_response(
            self.status,
            &ErrorBody {
                code: self.code,
                message: &self.message,
            },
        )
    }
}

impl From<HostApiError> for ApiError {
    fn from(err: HostApiError) -> Self {
        match err {
            HostApiError::Call(message) => Self::new(502, "HOST_CALL_FAILED", message),
            HostApiError::Op(message) => Self::new(502, "NODE_OPERATION_FAILED", message),
        }
    }
}

pub fn json_response<T: Serialize>(status: i32, value: &T) -> pb::HttpResponse {
    match serde_json::to_vec(value) {
        Ok(body) => pb::HttpResponse {
            status_code: status,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body,
        },
        Err(err) => pb::HttpResponse {
            status_code: 500,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body: format!(
                "{{\"code\":\"INTERNAL_ERROR\",\"message\":\"failed to serialize response: {err}\"}}"
            )
            .into_bytes(),
        },
    }
}

pub fn parse_json_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    if body.is_empty() {
        return Err(ApiError::bad_request("request body is empty"));
    }
    serde_json::from_slice(body)
        .map_err(|err| ApiError::bad_request(format!("invalid request body: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_shape() {
        let resp = ApiError::conflict("ALREADY_REGISTERED", "plugin already registered")
            .into_response();
        assert_eq!(resp.status_code, 409);
        let body: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid json");
        assert_eq!(body["code"], "ALREADY_REGISTERED");
        assert_eq!(body["message"], "plugin already registered");
        assert_eq!(
            resp.headers.get("Content-Type").map(String::as_str),
            Some("application/json")
        );
    }

    #[test]
    fn parse_body_errors() {
        assert!(parse_json_body::<serde_json::Value>(b"").is_err());
        assert!(parse_json_body::<serde_json::Value>(b"{oops").is_err());
        assert_eq!(
            parse_json_body::<serde_json::Value>(b"{\"a\":1}")
                .expect("valid json")["a"],
            1
        );
    }
}
