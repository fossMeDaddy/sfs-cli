use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::{
    digest::{
        consts::{B0, B1},
        typenum::{UInt, UTerm},
    },
    Sha256,
};
use std::num::NonZeroU32;

pub const NONCE_LENGTH: usize = 12;
pub const SALT_LENGTH: usize = 16;
pub const KEY_LENGTH: usize = 32;
pub const HEADER_LENGTH: usize = NONCE_LENGTH + SALT_LENGTH;

fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; KEY_LENGTH] {
    let mut key = [0u8; KEY_LENGTH];
    let iterations = NonZeroU32::new(100_000).unwrap();
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations.into(), &mut key);
    key
}

type CipherNonce = Nonce<UInt<UInt<UInt<UInt<UTerm, B1>, B1>, B0>, B0>>;

pub struct Decrypter {
    pub nonce: CipherNonce,
    pub key: [u8; KEY_LENGTH],
}

impl Decrypter {
    pub fn new(
        password: &str,
        cipher_header: [u8; NONCE_LENGTH + SALT_LENGTH],
    ) -> anyhow::Result<Self> {
        let (salt, nonce_bytes) = cipher_header.split_at(SALT_LENGTH);
        let nonce = GenericArray::from_slice(nonce_bytes);

        Ok(Self {
            nonce: nonce.to_owned().try_into()?,
            key: derive_key_from_password(password, salt),
        })
    }

    /// receives a ciphertext, the ciphertext should NOT contain nonce or salt or any other metadata
    pub fn decrypt(&self, ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;
        let plaintext = match cipher.decrypt(&self.nonce, ciphertext) {
            Ok(ciphertext) => ciphertext,
            Err(err) => {
                return Err(anyhow::anyhow!(err));
            }
        };

        Ok(plaintext)
    }
}

pub struct Encrypter {
    pub key: [u8; KEY_LENGTH],
    pub nonce: CipherNonce,
    pub salt: [u8; SALT_LENGTH],
}

impl Encrypter {
    pub fn new(password: &str) -> Self {
        let mut salt = [0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt);

        let key = derive_key_from_password(password, &salt);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        return Self { key, salt, nonce };
    }

    pub fn encrypt_buffer(&self, plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;

        let ciphertext = match cipher.encrypt(&self.nonce, plaintext) {
            Ok(ciphertext) => ciphertext,
            Err(err) => return Err(anyhow::anyhow!("Error encrypting message: {}", err)),
        };

        let mut encrypted_message = Vec::new();
        encrypted_message.extend_from_slice(&self.salt);
        encrypted_message.extend_from_slice(&self.nonce);
        encrypted_message.extend_from_slice(&ciphertext);

        Ok(encrypted_message)
    }
}
