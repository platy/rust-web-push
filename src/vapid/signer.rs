use super::VapidKey;
use super::VapidSignError;
use base64::{self, URL_SAFE_NO_PAD};
use openssl::hash::MessageDigest;
use openssl::sign::Signer as SslSigner;
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use url::Url;

lazy_static! {
    static ref JWT_HEADERS: String = base64::encode_config(
        &serde_json::to_string(&json!({
            "typ": "JWT",
            "alg": "ES256"
        }))
        .unwrap(),
        URL_SAFE_NO_PAD
    );
}

/// A struct representing a VAPID signature. Should be generated using the
/// [VapidSignatureBuilder](struct.VapidSignatureBuilder.html).
#[derive(Debug)]
pub struct VapidSignature {
    /// The signature
    pub auth_t: String,
    /// The public key
    pub auth_k: String,
}

impl<'a> Into<String> for &'a VapidSignature {
    fn into(self) -> String {
        format!("WebPush {}", self.auth_t)
    }
}

pub struct VapidSigner {}

impl VapidSigner {
    /// Create a signature with a given key. Sets the default audience from the
    /// endpoint host and sets the expiry in twelve hours. Values can be
    /// overwritten by adding the `aud` and `exp` claims.
    pub fn sign(
        key: &VapidKey,
        endpoint: &str,
        mut claims: BTreeMap<&str, Value>,
    ) -> Result<VapidSignature, VapidSignError> {
        if !claims.contains_key("aud") {
            let endpoint: Url = Url::parse(endpoint).unwrap();
            let audience = format!("{}://{}", endpoint.scheme(), endpoint.host().unwrap());
            claims.insert("aud", Value::String(audience));
        }

        if !claims.contains_key("exp") {
            let expiry = time::OffsetDateTime::now_utc() + time::Duration::hours(12);
            let number = Number::from(expiry.unix_timestamp());
            claims.insert("exp", Value::Number(number));
        }

        let signing_input = format!(
            "{}.{}",
            *JWT_HEADERS,
            base64::encode_config(&serde_json::to_string(&claims)?, URL_SAFE_NO_PAD)
        );

        let public_key = key.public_key();
        let auth_k = base64::encode_config(public_key, URL_SAFE_NO_PAD);

        let mut signer = SslSigner::new(MessageDigest::sha256(), key.p_key())?;
        signer.update(signing_input.as_bytes())?;

        let signature = signer.sign_to_vec()?;

        let r_off: usize = 3;
        let r_len = signature[r_off] as usize;
        let s_off: usize = r_off + r_len + 2;
        let s_len = signature[s_off] as usize;

        let mut r_val = &signature[(r_off + 1)..(r_off + 1 + r_len)];
        let mut s_val = &signature[(s_off + 1)..(s_off + 1 + s_len)];

        if r_len == 33 && r_val[0] == 0 {
            r_val = &r_val[1..];
        }

        if s_len == 33 && s_val[0] == 0 {
            s_val = &s_val[1..];
        }

        let mut sigval: Vec<u8> = Vec::with_capacity(64);
        sigval.extend(r_val);
        sigval.extend(s_val);

        trace!("Public key: {}", auth_k);

        let auth_t = format!(
            "{}.{}",
            signing_input,
            base64::encode_config(&sigval, URL_SAFE_NO_PAD)
        );

        Ok(VapidSignature { auth_t, auth_k })
    }
}

#[cfg(test)]
mod tests {
    use crate::vapid::VapidSignature;

    #[test]
    fn test_vapid_signature_aesgcm_format() {
        let vapid_signature = &VapidSignature {
            auth_t: String::from("foo"),
            auth_k: String::from("bar"),
        };

        let header_value: String = vapid_signature.into();

        assert_eq!("WebPush foo", &header_value);
    }
}
