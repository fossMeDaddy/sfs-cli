use orion::{
    aead::streaming::{self, StreamSealer},
    kdf,
};

use crate::shared_types;

// takes in stream_sealer or stream_opener as `T`
pub struct CryptoStream<T> {
    pub e: T,
    pub salt: kdf::Salt,
    pub nonce: streaming::Nonce,
}

impl CryptoStream<StreamSealer> {
    pub fn into_encryption_metadata(
        &self,
        block_size: Option<u32>,
    ) -> shared_types::EncryptionMetadata {
        shared_types::EncryptionMetadata {
            attempt_decryption: true,
            block_size,
            nonce: Some(self.nonce.as_ref().to_vec()),
            salt: Some(self.salt.as_ref().to_vec()),
        }
    }
}

pub fn derive_key_from_password(
    pwd: &[u8],
    salt: &kdf::Salt,
) -> Result<kdf::SecretKey, orion::errors::UnknownCryptoError> {
    let pwd = kdf::Password::from_slice(pwd)?;
    kdf::derive_key(&pwd, salt, 3, 8, 32)
}

pub fn new_encryptor(
    password: &str,
) -> Result<CryptoStream<streaming::StreamSealer>, orion::errors::UnknownCryptoError> {
    let salt = kdf::Salt::default();
    let (sealer, nonce) =
        streaming::StreamSealer::new(&derive_key_from_password(password.as_bytes(), &salt)?)?;
    Ok(CryptoStream {
        e: sealer,
        salt,
        nonce,
    })
}
