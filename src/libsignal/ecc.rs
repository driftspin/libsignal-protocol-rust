use crate::helpers;
use crate::libsignal::{ecc, protocol, Curve25519};
use std::convert::TryInto;

static DJB_TYPE: u8 = 0x05;

pub struct Curve;

pub struct InvalidKeyError(pub String);

impl Curve {
    pub fn generate_key_pair() -> KeyPair {
        Curve25519::generate_key_pair()
    }

    pub fn decode_point(bytes: &[u8], offset: usize) -> Result<impl ECPublicKey, InvalidKeyError> {
        if bytes.len() == 0 || bytes.len() - offset < 1 {
            Err(InvalidKeyError("No key type identifier".to_string()))
        } else {
            // Truncate the number to last 8 bits
            let type_ = &bytes[offset] & 0xff;

            if type_ == DJB_TYPE.try_into().unwrap() {
                if bytes.len() - offset < 33 {
                    Err(InvalidKeyError(
                        format!("Bad key length: {}", bytes.len()).to_string(),
                    ))
                } else {
                    let mut key_bytes = &[0; 32][..];
                    let start_pos = offset + 1;
                    let result =
                        helpers::slices::copy(&bytes, offset + 1, key_bytes, 0, key_bytes.len());
                    match result {
                        Ok(v) => match helpers::slices::to_array32(&(&v)) {
                            Ok(arr) => Ok(PublicKey(arr)),
                            Err(_) => Err(InvalidKeyError(format!("Bad key type: {}", type_))),
                        },
                        Err(_) => Err(InvalidKeyError(format!("Bad key type: {}", type_))),
                    }
                }
            } else {
                Err(InvalidKeyError(format!("Bad key type: {}", type_)))
            }
        }
    }

    pub fn decode_private_point(bytes: &[u8]) -> Result<impl ECPrivateKey, InvalidKeyError> {
        match helpers::slices::to_array32(bytes) {
            Ok(b) => Ok(PrivateKey(b)),
            Err(_) => Err(InvalidKeyError("Error decoding private point".to_string())),
        }
    }

    pub fn calculate_agreement<PublicKey: ECPublicKey, PrivateKey: ECPrivateKey>(
        public_key: &PublicKey,
        private_key: &PrivateKey,
    ) -> Result<[u8; 32], InvalidKeyError> {
        let (a, b) = (public_key.get_type(), private_key.get_type());

        if a != b || a != DJB_TYPE {
            return Err(InvalidKeyError(
                "Public and private keys must be of the same type!".to_string(),
            ));
        }

        Ok(Curve25519::calculate_agreement(
            public_key.get_public_key(),
            private_key.get_private_key(),
        ))
    }
}

pub struct KeyPair {
    pub public_key: PublicKey,
    pub private_key: PrivateKey,
}

impl KeyPair {
    pub fn new(public: PublicKey, private: PrivateKey) -> Self {
        Self {
            public_key: public,
            private_key: private,
        }
    }
}

pub trait ECPublicKey {
    fn from(bytes: [u8; 32]) -> Self;
    fn serialize(&self) -> [u8; 32];
    fn get_type(&self) -> u8;
    fn get_public_key(&self) -> [u8; 32];
}

pub trait ECPrivateKey {
    fn serialize(&self) -> [u8; 32];
    fn get_type(&self) -> u8;
    fn get_private_key(&self) -> [u8; 32];
}

pub struct PrivateKey(pub [u8; 32]);

impl PartialEq for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PrivateKey {}

impl PrivateKey {
    pub fn new(bytes: &mut [u8; 32]) -> Self {
        let mut buf: [u8; 32] = [0; 32];
        buf.clone_from_slice(bytes);
        Self(buf)
    }
}

impl ECPrivateKey for PrivateKey {
    fn serialize(&self) -> [u8; 32] {
        self.0
    }

    fn get_type(&self) -> u8 {
        DJB_TYPE
    }

    fn get_private_key(&self) -> [u8; 32] {
        self.0
    }
}

pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PublicKey {}

impl ECPublicKey for PublicKey {
    fn from(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }

    fn serialize(&self) -> [u8; 32] {
        self.0
    }

    fn get_type(&self) -> u8 {
        DJB_TYPE
    }

    fn get_public_key(&self) -> [u8; 32] {
        self.0
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    pub fn test_keypair_decode_point() {
        let b1 = &[];
        let b2 = &[0; 10];
        let offset1 = 2;
        let offset2 = 10;
        let b3 = &helpers::slices::concat_2(&[0x00, 0x08, 0x05], &[0x00; 64][..]);

        match Curve::decode_point(b1, 2) {
            Ok(_) => panic!("Expected Error"),
            Err(InvalidKeyError(s)) => assert_eq!(s, "No key type identifier".to_string()),
        }

        match Curve::decode_point(b2, 10) {
            Ok(_) => panic!("Expected Error"),
            Err(InvalidKeyError(s)) => assert_eq!(s, "No key type identifier".to_string()),
        }

        match Curve::decode_point(b3, 2) {
            Ok(_) => assert!(true),
            Err(_) => panic!("Expect Ok, found Err"),
        }
    }

    #[test]
    pub fn test_keypair_decode_private_point() {
        let too_short = &[0; 10];
        let too_long = &[0x08; 64];
        let good_size = &[0x05; 32];

        match Curve::decode_private_point(too_short) {
            Ok(_) => panic!("Expect to fail"),
            Err(_) => assert!(true),
        }

        match Curve::decode_private_point(too_long) {
            Ok(_) => panic!("Expect to fail"),
            Err(_) => assert!(true),
        }

        match Curve::decode_private_point(good_size) {
            Ok(_) => assert!(true),
            Err(_) => panic!("Expect Ok"),
        }
    }

    #[test]
    pub fn test_keypair_calculate_agreement() {
        let alice = Curve::generate_key_pair();
        let bob = Curve::generate_key_pair();

        let shared_key_1 = Curve::calculate_agreement(&alice.public_key, &bob.private_key);
        let shared_key_2 = Curve::calculate_agreement(&alice.public_key, &bob.private_key);

        if let Ok(k1) = shared_key_1 {
            if let Ok(k2) = shared_key_2 {
                assert!(k1.len() > 0);
                assert!(k2.len() > 0);
                assert_eq!(k1, k2);
                println!("HOOO");
                return ();
            }
        }
        panic!("Expected Ok, got Error");
    }
}
