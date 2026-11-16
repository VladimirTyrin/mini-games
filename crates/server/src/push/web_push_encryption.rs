//! Message encryption for Web Push (RFC 8291, `aes128gcm` content coding of RFC 8188): push
//! services only relay ciphertext that the subscribed browser can decrypt.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, KeyInit};
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToSec1Point;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

/// One record carries the whole message; push services accept at most 4096 bytes of body.
const RECORD_SIZE: u32 = 4096;
const HEADER_BYTES: usize = 16 + 4 + 1 + 65;
const TAG_BYTES: usize = 16;
/// The longest plaintext that fits one record next to the padding delimiter and the tag.
pub(crate) const MAX_PLAINTEXT_BYTES: usize = RECORD_SIZE as usize - HEADER_BYTES - TAG_BYTES - 1;

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncryptionError {
    #[error("push message of {0} bytes is too long")]
    TooLong(usize),
    #[error("key derivation failed")]
    KeyDerivation,
    #[error("AES-GCM encryption failed")]
    Cipher,
}

/// Encrypts `plaintext` for the subscription's `client_key` and `auth_secret` with a fresh
/// server key pair and salt.
pub(crate) fn encrypt(
    plaintext: &[u8],
    client_key: &PublicKey,
    auth_secret: &[u8; 16],
) -> Result<Vec<u8>, EncryptionError> {
    let server_key = random_secret_key();
    let salt: [u8; 16] = rand::random();
    encrypt_with(plaintext, client_key, auth_secret, &server_key, &salt)
}

fn random_secret_key() -> SecretKey {
    // A random 32-byte string is a valid scalar unless it is zero or above the group order
    // (chance about 2^-32), so this loop practically always runs once.
    loop {
        if let Ok(key) = SecretKey::from_slice(&rand::random::<[u8; 32]>()) {
            return key;
        }
    }
}

pub(crate) fn encrypt_with(
    plaintext: &[u8],
    client_key: &PublicKey,
    auth_secret: &[u8; 16],
    server_key: &SecretKey,
    salt: &[u8; 16],
) -> Result<Vec<u8>, EncryptionError> {
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(EncryptionError::TooLong(plaintext.len()));
    }
    let client_public = client_key.to_sec1_point(false);
    let server_public = server_key.public_key().to_sec1_point(false);
    let shared = p256::ecdh::diffie_hellman(server_key.to_nonzero_scalar(), client_key.as_affine());

    let mut key_info = Vec::with_capacity(14 + 65 + 65);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(client_public.as_bytes());
    key_info.extend_from_slice(server_public.as_bytes());
    let mut input_key = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth_secret), shared.raw_secret_bytes())
        .expand(&key_info, &mut input_key)
        .map_err(|_| EncryptionError::KeyDerivation)?;

    let content = Hkdf::<Sha256>::new(Some(salt), &input_key);
    let mut content_key = [0u8; 16];
    content
        .expand(b"Content-Encoding: aes128gcm\0", &mut content_key)
        .map_err(|_| EncryptionError::KeyDerivation)?;
    let mut nonce = [0u8; 12];
    content
        .expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|_| EncryptionError::KeyDerivation)?;

    let mut padded = Vec::with_capacity(plaintext.len() + 1);
    padded.extend_from_slice(plaintext);
    // Delimiter of the last (here: only) record, with no padding after it.
    padded.push(2);
    let cipher = Aes128Gcm::new(&content_key.into());
    let ciphertext = cipher
        .encrypt(&nonce.into(), padded.as_slice())
        .map_err(|_| EncryptionError::Cipher)?;

    let mut body = Vec::with_capacity(HEADER_BYTES + ciphertext.len());
    body.extend_from_slice(salt);
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(65);
    body.extend_from_slice(server_public.as_bytes());
    body.extend_from_slice(&ciphertext);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    use super::*;

    fn bytes(base64url: &str) -> Vec<u8> {
        URL_SAFE_NO_PAD.decode(base64url).unwrap()
    }

    /// The example of RFC 8291 section 5 (intermediate values in its appendix A).
    #[test]
    fn matches_the_rfc_8291_example() {
        let client_key = PublicKey::from_sec1_bytes(&bytes(
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
        ))
        .unwrap();
        let server_key =
            SecretKey::from_slice(&bytes("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let auth_secret: [u8; 16] = bytes("BTBZMqHH6r4Tts7J_aSIgg").try_into().unwrap();
        let salt: [u8; 16] = bytes("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();

        let body = encrypt_with(
            b"When I grow up, I want to be a watermelon",
            &client_key,
            &auth_secret,
            &server_key,
            &salt,
        )
        .unwrap();

        assert_eq!(
            URL_SAFE_NO_PAD.encode(body),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml\
             mlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPT\
             pK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn refuses_a_message_longer_than_one_record() {
        let client_key = SecretKey::from_slice(&[7; 32]).unwrap().public_key();
        let too_long = vec![b'x'; MAX_PLAINTEXT_BYTES + 1];

        let result = encrypt(&too_long, &client_key, &[1; 16]);

        assert!(matches!(result, Err(EncryptionError::TooLong(_))));
    }
}
