use openssl::error::ErrorStack;
use serde_json::error::Error as JsonError;
use std::{convert::From, error::Error, fmt, io::Error as IoError};

#[derive(PartialEq, Debug)]
pub enum VapidKeyError {
    /// Error in SSL signing
    SslError,
    /// Error in reading the key file
    IoError,
}

impl From<IoError> for VapidKeyError {
    fn from(_: IoError) -> Self {
        Self::IoError
    }
}

impl From<ErrorStack> for VapidKeyError {
    fn from(_: ErrorStack) -> Self {
        Self::SslError
    }
}

impl Error for VapidKeyError {
    fn description(&self) -> &str {
        match *self {
            Self::SslError => "Error signing with SSL",
            Self::IoError => "Error opening a file",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for VapidKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VapidKeyError: {}", self)
    }
}

#[derive(PartialEq, Debug)]
pub enum VapidSignError {
    /// Error in SSL signing
    SslError,
    ClaimSerialization,
}

impl From<ErrorStack> for VapidSignError {
    fn from(_: ErrorStack) -> Self {
        Self::SslError
    }
}

impl From<JsonError> for VapidSignError {
    fn from(_: JsonError) -> Self {
        Self::ClaimSerialization
    }
}

impl Error for VapidSignError {
    fn description(&self) -> &str {
        match *self {
            Self::SslError => "Error signing with SSL",
            Self::ClaimSerialization => "Error serializing claims",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for VapidSignError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VapidSignError: {}", self)
    }
}
