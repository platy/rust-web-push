use base64::DecodeError;
use ring::error;
use serde_json::error::Error as JsonError;
use std::string::FromUtf8Error;
use std::time::{Duration, SystemTime};
use std::{convert::From, error::Error, fmt};

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
            err => Self::Other(err.to_string()),
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
            status if status >= 500 => Self::ServerError(retry_after),

            401 => Self::Unauthorized,
            410 => Self::EndpointNotValid,
            404 => Self::EndpointNotFound,
            413 => Self::PayloadTooLarge,

            400 => match read_body_as_error_info_json() {
                Ok(error_info) => Self::BadRequest(Some(error_info.error)),
                Err(_) => Self::BadRequest(None),
            },

            e => Self::Other(e.to_string()),
        }
    }
}

impl From<JsonError> for WebPushError {
    fn from(_: JsonError) -> Self {
        Self::InvalidResponse
    }
}

impl From<FromUtf8Error> for WebPushError {
    fn from(_: FromUtf8Error) -> Self {
        Self::InvalidResponse
    }
}

impl From<error::Unspecified> for WebPushError {
    fn from(_: error::Unspecified) -> Self {
        Self::Unspecified
    }
}

impl From<DecodeError> for WebPushError {
    fn from(_: DecodeError) -> Self {
        Self::InvalidCryptoKeys
    }
}

impl WebPushError {
    pub fn short_description(&self) -> &'static str {
        match *self {
            Self::Unspecified => "unspecified",
            Self::Unauthorized => "unauthorized",
            Self::BadRequest(_) => "bad_request",
            Self::ServerError(_) => "server_error",
            Self::NotImplemented => "not_implemented",
            Self::InvalidUri => "invalid_uri",
            Self::EndpointNotValid => "endpoint_not_valid",
            Self::EndpointNotFound => "endpoint_not_found",
            Self::PayloadTooLarge => "payload_too_large",
            Self::TlsError => "tls_error",
            Self::InvalidTtl => "invalid_ttl",
            Self::InvalidResponse => "invalid_response",
            Self::MissingCryptoKeys => "missing_crypto_keys",
            Self::InvalidCryptoKeys => "invalid_crypto_keys",
            Self::Other(_) => "other",
        }
    }
}

impl Error for WebPushError {
    fn description(&self) -> &str {
        match *self {
            Self::Unspecified => "An unknown error happened encrypting the message",
            Self::Unauthorized => "Please provide valid credentials to send the notification",
            Self::BadRequest(_) => "Request was badly formed",
            Self::ServerError(_) => {
                "Server was unable to process the request, please try again later"
            }
            Self::PayloadTooLarge => "Maximum allowed payload size is 3070 characters",
            Self::InvalidUri => "The provided URI is invalid",
            Self::NotImplemented => "The feature is not implemented yet",
            Self::EndpointNotValid => {
                "The URL specified is no longer valid and should no longer be used"
            }
            Self::EndpointNotFound => "The URL specified is invalid and should not be used again",
            Self::TlsError => "Could not initialize a TLS connection",
            Self::InvalidTtl => "The TTL value provided was not valid or was not provided",
            Self::InvalidResponse => "The response data couldn't be parses",
            Self::MissingCryptoKeys => "The request is missing cryptographic keys",
            Self::InvalidCryptoKeys => "The request is having invalid cryptographic keys",
            Self::Other(_) => "An unknown error when connecting the notification service",
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
