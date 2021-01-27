#[cfg(feature = "ureq")]
use ureq::Agent;

use crate::error::WebPushError;
use crate::message::WebPushMessage;

#[cfg(feature = "hyper")]
use futures::stream::StreamExt;
#[cfg(feature = "hyper")]
use hyper::{
    client::{Client, HttpConnector},
    header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, RETRY_AFTER},
    Request, StatusCode,
};
#[cfg(feature = "hyper")]
use hyper_tls::HttpsConnector;

/// An client for sending the notification payload.
pub struct WebPushClient {
    #[cfg(feature = "ureq")]
    client: Agent,
    #[cfg(feature = "hyper")]
    client: Client<HttpsConnector<HttpConnector>>,
}

impl WebPushClient {
    pub fn new() -> WebPushClient {
        WebPushClient {
            #[cfg(feature = "ureq")]
            client: Agent::new(),
            #[cfg(feature = "hyper")]
            client: Client::builder().build(HttpsConnector::new()),
        }
    }

    #[cfg(feature = "ureq")]
    /// Sends a notification. Blocking. Never times out.
    pub fn send(&self, message: WebPushMessage) -> Result<(), WebPushError> {
        let mut builder = self
            .client
            .post(&message.endpoint)
            .set("TTL", &message.ttl.to_string());

        let body = if let Some(payload) = message.payload {
            builder = builder
                .set("Content-Encoding", payload.content_encoding)
                .set(
                    "Content-Length",
                    &format!("{}", payload.content.len() as u64),
                )
                .set("Content-Type", "application/octet-stream");

            for (k, v) in payload.crypto_headers.into_iter() {
                let v: &str = v.as_ref();
                builder = builder.set(k, v);
            }

            payload.content
        } else {
            vec![]
        };
        let response = builder.send_bytes(&body);

        trace!("Response: {:?}", response);
        Ok(())
    }

    #[cfg(feature = "hyper")]
    /// Sends a notification. Asynchronous. Never times out.
    pub async fn send(&self, message: WebPushMessage) -> Result<(), WebPushError> {
        let mut builder = Request::builder()
            .method("POST")
            .uri(message.endpoint)
            .header("TTL", format!("{}", message.ttl).as_bytes());

        let request = if let Some(payload) = message.payload {
            builder = builder
                .header(CONTENT_ENCODING, payload.content_encoding)
                .header(
                    CONTENT_LENGTH,
                    format!("{}", payload.content.len() as u64).as_bytes(),
                )
                .header(CONTENT_TYPE, "application/octet-stream");

            for (k, v) in payload.crypto_headers.into_iter() {
                let v: &str = v.as_ref();
                builder = builder.header(k, v);
            }

            builder.body(payload.content.into()).unwrap()
        } else {
            builder.body("".into()).unwrap()
        };
        trace!("request headers {:?}", request.headers());

        let response = self.client.request(request).await?;

        trace!("response status {}", response.status());

        match response.status() {
            status if status.is_success() => Ok(()),
            status if status.is_server_error() => {
                let retry_after = response
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(|s| s.to_str().ok())
                    .and_then(|ra| crate::error::retry_after_from_str(ra));
                Err(WebPushError::ServerError(retry_after))
            }

            StatusCode::UNAUTHORIZED => Err(WebPushError::Unauthorized),
            StatusCode::GONE => Err(WebPushError::EndpointNotValid),
            StatusCode::NOT_FOUND => Err(WebPushError::EndpointNotFound),
            StatusCode::PAYLOAD_TOO_LARGE => Err(WebPushError::PayloadTooLarge),

            StatusCode::BAD_REQUEST => {
                let content_length: usize = response
                    .headers()
                    .get(CONTENT_LENGTH)
                    .and_then(|s| s.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                let mut body: Vec<u8> = Vec::with_capacity(content_length);
                let mut chunks = response.into_body();

                while let Some(chunk) = chunks.next().await {
                    body.extend_from_slice(&chunk?);
                }

                match String::from_utf8(body) {
                    Err(_) => Err(WebPushError::BadRequest(None)),
                    Ok(body_str) => {
                        match serde_json::from_str::<crate::error::ErrorInfo>(&body_str) {
                            Ok(error_info) => Err(WebPushError::BadRequest(Some(error_info.error))),
                            Err(_) if body_str != "" => {
                                Err(WebPushError::BadRequest(Some(body_str)))
                            }
                            Err(_) => Err(WebPushError::BadRequest(None)),
                        }
                    }
                }
            }

            e => Err(WebPushError::Other(format!("{:?}", e))),
        }
    }
}
impl Default for WebPushClient {
    fn default() -> Self {
        Self::new()
    }
}
