use std::io;
use openssl::ec::{EcGroup, EcKey, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::Private;
use openssl::{bn::BigNumContext, error::ErrorStack as OpenSslError, pkey::PKey};
use super::VapidKeyError;

pub struct VapidKey {
    p_key: PKey<Private>,
    public_key: Vec<u8>,
}

lazy_static! {
    static ref GROUP: EcGroup =
        EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).expect("EC Prime256v1 not supported");
}

impl VapidKey {
    pub fn new(ec_key: EcKey<Private>) -> Result<VapidKey, OpenSslError> {
        let mut ctx = BigNumContext::new().unwrap();
        let key = ec_key.public_key();
        let public_key = key
            .to_bytes(&*GROUP, PointConversionForm::UNCOMPRESSED, &mut ctx)
            .unwrap();

        let p_key = PKey::from_ec_key(ec_key)?;
        Ok(VapidKey {
            p_key,
            public_key,
        })
    }

    /// Creates a new key from a PEM formatted private key.
    pub fn from_pem<R: io::Read>(mut pk_pem: R) -> Result<Self, VapidKeyError> {
        let mut pem_key: Vec<u8> = Vec::new();
        pk_pem.read_to_end(&mut pem_key)?; //// io error

        Ok(Self::new(
            EcKey::private_key_from_pem(&pem_key)?, // openssl error
        )?)
    }

    /// Creates a new key from a DER formatted private key.
    pub fn from_der<R: io::Read>(mut pk_der: R) -> Result<Self, VapidKeyError> {
        let mut der_key: Vec<u8> = Vec::new();
        pk_der.read_to_end(&mut der_key)?;

        Ok(Self::new(EcKey::private_key_from_der(&der_key)?)?)
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn p_key(&self) -> &PKey<Private> {
        &self.p_key
    }
}

#[cfg(test)]
mod tests {
    use crate::vapid::key::VapidKey;
    use openssl::ec::EcKey;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_public_key_derivation() {
        let mut f = File::open("resources/vapid_test_key.pem").unwrap();
        let mut pem_key: Vec<u8> = Vec::new();
        f.read_to_end(&mut pem_key).unwrap();

        let ec = EcKey::private_key_from_pem(&pem_key).unwrap();
        let key = VapidKey::new(ec).unwrap();

        assert_eq!(
            vec![
                4, 202, 53, 30, 162, 133, 234, 201, 12, 101, 140, 164, 174, 215, 189, 118, 234,
                152, 192, 16, 244, 242, 96, 208, 41, 59, 167, 70, 66, 93, 15, 123, 19, 39, 209, 62,
                203, 35, 122, 176, 153, 79, 89, 58, 74, 54, 26, 126, 203, 98, 158, 75, 170, 0, 52,
                113, 126, 171, 124, 55, 237, 176, 165, 111, 181
            ],
            key.public_key()
        );
    }
}
