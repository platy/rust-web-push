use base64::DecodeError;
use openssl::error::ErrorStack;
use ring::error;
use serde_json::error::Error as JsonError;
use std::string::FromUtf8Error;
use std::time::{Duration, SystemTime};
use std::{convert::From, error::Error, fmt, io::Error as IoError};

#[derive(PartialEq, Debug)]
pub enum WebPushError {
    /// An unknown error happened encrypting the message,
    Unspecified,
    /// Please provide valid credentials to send the notification
    Unauthorized,
    /// Request was badly formed
    BadRequest(Option<String>),
    /// Contains an optional `Duration`, until the user can retry the request
    ServerError(Option<Duration>),
    /// The feature is not implemented yet
    NotImplemented,
    /// The provided URI is invalid
    InvalidUri,
    /// The URL specified is no longer valid and should no longer be used
    EndpointNotValid,
    /// The URL specified is invalid and should not be used again
    EndpointNotFound,
    /// Maximum allowed payload size is 3800 characters
    PayloadTooLarge,
    /// Could not initialize a TLS connection
    TlsError,
    /// Error in SSL signing
    SslError,
    /// Error in reading a file
    IoError,
    /// The TTL value provided was not valid or was not provided
    InvalidTtl,
    /// The request was missing required crypto keys
    MissingCryptoKeys,
    /// One or more of the crypto key elements are invalid.
    InvalidCryptoKeys,
    /// Corrupted response data
    InvalidResponse,
    Other(String),
}

// impl fmt::Display for WebPushError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         <Self as fmt::Debug>::fmt(self, f)
//     }
// }

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ErrorInfo {
    code: u16,
    errno: u16,
    pub error: String,
    message: String,
}

#[cfg(feature = "hyper")]
impl From<hyper::Error> for WebPushError {
    fn from(err: hyper::Error) -> Self {
        debug!("{}", err);
        Self::Unspecified
    }
}

#[cfg(feature = "hyper")]
impl From<http::Error> for WebPushError {
    fn from(err: http::Error) -> Self {
        debug!("{}", err);
        Self::Unspecified
    }
}

#[cfg(feature = "ureq")]
impl From<ureq::Error> for WebPushError {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Status(status, response) => {
                let retry_after = response
                    .header("Retry-After")
                    .and_then(retry_after_from_str);
                Self::from_error_response(status, retry_after, || response.into_json::<ErrorInfo>())
            }
            err => WebPushError::Other(err.to_string()),
        }
    }
}

impl WebPushError {
    pub fn from_error_response<ReadJsonError>(
        status: u16,
        retry_after: Option<Duration>,
        read_body_as_error_info_json: impl FnOnce() -> Result<ErrorInfo, ReadJsonError>,
    ) -> Self {
        match status {
            status if status >= 500 => WebPushError::ServerError(retry_after),

            401 => WebPushError::Unauthorized,
            410 => WebPushError::EndpointNotValid,
            404 => WebPushError::EndpointNotFound,
            413 => WebPushError::PayloadTooLarge,

            400 => match read_body_as_error_info_json() {
                Ok(error_info) => WebPushError::BadRequest(Some(error_info.error)),
                Err(_) => WebPushError::BadRequest(None),
            },

            e => WebPushError::Other(e.to_string()),
        }
    }
}

impl From<JsonError> for WebPushError {
    fn from(_: JsonError) -> WebPushError {
        WebPushError::InvalidResponse
    }
}

impl From<FromUtf8Error> for WebPushError {
    fn from(_: FromUtf8Error) -> WebPushError {
        WebPushError::InvalidResponse
    }
}

impl From<error::Unspecified> for WebPushError {
    fn from(_: error::Unspecified) -> WebPushError {
        WebPushError::Unspecified
    }
}

impl From<IoError> for WebPushError {
    fn from(_: IoError) -> WebPushError {
        WebPushError::IoError
    }
}

impl From<ErrorStack> for WebPushError {
    fn from(_: ErrorStack) -> WebPushError {
        WebPushError::SslError
    }
}

impl From<DecodeError> for WebPushError {
    fn from(_: DecodeError) -> WebPushError {
        WebPushError::InvalidCryptoKeys
    }
}

impl WebPushError {
    pub fn short_description(&self) -> &'static str {
        match *self {
            WebPushError::Unspecified => "unspecified",
            WebPushError::Unauthorized => "unauthorized",
            WebPushError::BadRequest(_) => "bad_request",
            WebPushError::ServerError(_) => "server_error",
            WebPushError::NotImplemented => "not_implemented",
            WebPushError::InvalidUri => "invalid_uri",
            WebPushError::EndpointNotValid => "endpoint_not_valid",
            WebPushError::EndpointNotFound => "endpoint_not_found",
            WebPushError::PayloadTooLarge => "payload_too_large",
            WebPushError::TlsError => "tls_error",
            WebPushError::InvalidTtl => "invalid_ttl",
            WebPushError::InvalidResponse => "invalid_response",
            WebPushError::MissingCryptoKeys => "missing_crypto_keys",
            WebPushError::InvalidCryptoKeys => "invalid_crypto_keys",
            WebPushError::SslError => "ssl_error",
            WebPushError::IoError => "io_error",
            WebPushError::Other(_) => "other",
        }
    }
}

impl Error for WebPushError {
    fn description(&self) -> &str {
        match *self {
            WebPushError::Unspecified => "An unknown error happened encrypting the message",
            WebPushError::Unauthorized => {
                "Please provide valid credentials to send the notification"
            }
            WebPushError::BadRequest(_) => "Request was badly formed",
            WebPushError::ServerError(_) => {
                "Server was unable to process the request, please try again later"
            }
            WebPushError::PayloadTooLarge => "Maximum allowed payload size is 3070 characters",
            WebPushError::InvalidUri => "The provided URI is invalid",
            WebPushError::NotImplemented => "The feature is not implemented yet",
            WebPushError::EndpointNotValid => {
                "The URL specified is no longer valid and should no longer be used"
            }
            WebPushError::EndpointNotFound => {
                "The URL specified is invalid and should not be used again"
            }
            WebPushError::TlsError => "Could not initialize a TLS connection",
            WebPushError::SslError => "Error signing with SSL",
            WebPushError::IoError => "Error opening a file",
            WebPushError::InvalidTtl => "The TTL value provided was not valid or was not provided",
            WebPushError::InvalidResponse => "The response data couldn't be parses",
            WebPushError::MissingCryptoKeys => "The request is missing cryptographic keys",
            WebPushError::InvalidCryptoKeys => "The request is having invalid cryptographic keys",
            WebPushError::Other(_) => "An unknown error when connecting the notification service",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for WebPushError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WebPushError: {}", self)
    }
}

pub fn retry_after_from_str(header_value: &str) -> Option<Duration> {
    if let Ok(seconds) = header_value.parse::<u64>() {
        Some(Duration::from_secs(seconds))
    } else {
        time::OffsetDateTime::parse(header_value, "%a, %d %b %Y %H:%M:%S %z")
            .map(|date_time| {
                let systime: SystemTime = date_time.into();

                systime
                    .duration_since(SystemTime::now())
                    .unwrap_or_else(|_| Duration::new(0, 0))
            })
            .ok()
    }
}
