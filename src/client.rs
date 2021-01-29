use std::borrow::Cow;

#[cfg(feature = "ureq")]
use ureq::Agent;

use crate::error::WebPushError;
use crate::message::WebPushMessage;

#[cfg(feature = "hyper")]
use futures::stream::StreamExt;
#[cfg(feature = "hyper")]
use hyper::{
    client::HttpConnector,
    header::{CONTENT_LENGTH, RETRY_AFTER},
    Request, StatusCode,
};
#[cfg(feature = "hyper")]
use hyper_tls::HttpsConnector;

/// An client for sending the notification payload.
pub struct WebPushClient<Client> {
    client: Client,
}

impl<Client> WebPushClient<Client> {
    fn headers(message: &WebPushMessage) -> impl IntoIterator<Item = (&'static str, Cow<'_, str>)> {
        let ttl_header: (&'static str, Cow<'_, str>) = ("TTL", message.ttl.to_string().into());
        if let Some(payload) = &message.payload {
            let mut headers = vec![
                ttl_header,
                ("Content-Encoding", payload.content_encoding.into()),
                (
                    "Content-Length",
                    format!("{}", payload.content.len() as u64).into(),
                ),
                ("Content-Type", "application/octet-stream".into()),
            ];
            headers.extend(
                payload
                    .crypto_headers
                    .iter()
                    .map(|(k, v)| (*k, Cow::Borrowed(v.as_str()))),
            );
            headers
        } else {
            vec![ttl_header]
        }
    }
}

/// Client for web push which blocks (using the ureq client)
#[cfg(feature = "ureq")]
pub type BlockingWebPushClient = WebPushClient<ureq::Agent>;

#[cfg(feature = "ureq")]
impl BlockingWebPushClient {
    pub fn new() -> Self {
        Self {
            client: Agent::new(),
        }
    }

    /// Sends a notification. Blocking. Never times out.
    pub fn send(&self, message: WebPushMessage) -> Result<(), WebPushError> {
        let mut request = self.client.post(&message.endpoint);
        for (header, value) in Self::headers(&message) {
            request = request.set(header, &value);
        }

        let body = if let Some(payload) = message.payload {
            payload.content
        } else {
            vec![]
        };
        let response = request.send_bytes(&body)?;

        trace!("Response: {:?}", response);
        Ok(())
    }
}

impl Default for BlockingWebPushClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Client for web push for use in the tokio runtime (using the hyper client)
#[cfg(feature = "hyper")]
pub type TokioWebPushClient = WebPushClient<hyper::Client<HttpsConnector<HttpConnector>>>;

#[cfg(feature = "hyper")]
impl TokioWebPushClient {
    pub fn new() -> Self {
        Self {
            client: hyper::Client::builder().build(HttpsConnector::new()),
        }
    }

    /// Sends a notification. Asynchronous. Never times out.
    pub async fn send(&self, message: WebPushMessage) -> Result<(), WebPushError> {
        let mut request = Request::builder().method("POST").uri(&message.endpoint);
        for (header, value) in Self::headers(&message) {
            request = request.header(header, value.as_ref());
        }

        let body = if let Some(payload) = message.payload {
            payload.content
        } else {
            vec![]
        };
        let request = request.body(body.into()).unwrap();
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

impl Default for TokioWebPushClient {
    fn default() -> Self {
        Self::new()
    }
}
