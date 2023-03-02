use jsonwebtoken::{DecodingKey, EncodingKey};
use std::borrow::Borrow;

pub fn parse_encoding_key<S: Borrow<str>>(rsa_pem: S) -> EncodingKey {
    let rsa_pem = rsa_pem.borrow().replace('_', "\n");
    EncodingKey::from_rsa_pem(rsa_pem.as_bytes()).unwrap()
}

pub fn parse_decoding_key<S: Borrow<str>>(rsa_pem: S) -> DecodingKey {
    let rsa_pem = rsa_pem.borrow().replace('_', "\n");
    DecodingKey::from_rsa_pem(rsa_pem.as_bytes()).unwrap()
}
