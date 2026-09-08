//! Apple Remote Desktop authentication (security type 30).
//!
//! This is what macOS Screen Sharing uses by default, before the "VNC
//! viewers may control screen with password" option is enabled. It is
//! undocumented by Apple but well understood; the implementation here
//! follows the wire format used by libvncclient, gtk-vnc and others.
//!
//! Wire flow after the client selects type 30:
//!
//! ```text
//! server -> client:  generator (u16), key_len (u16),
//!                    prime  [key_len bytes, big-endian],
//!                    server_public_key [key_len bytes, big-endian]
//! client -> server:  ciphertext [128 bytes], client_public_key [key_len bytes]
//! server -> client:  SecurityResult (u32)
//! ```
//!
//! The client computes a Diffie-Hellman shared secret, hashes it with MD5
//! to get a 128-bit AES key, then AES-128-ECB encrypts a 128-byte
//! credential block: username (NUL terminated) at offset 0, password (NUL
//! terminated) at offset 64, each padded with random bytes.

use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes128;
use md5::{Digest, Md5};
use num_bigint::BigUint;
use rand::RngCore;

use crate::error::{Error, Result};

/// Parameters the server sends at the start of ARD auth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerParams {
    pub generator: u16,
    pub key_len: usize,
    pub prime: Vec<u8>,
    pub server_public: Vec<u8>,
}

/// What the client sends back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientResponse {
    pub ciphertext: [u8; 128],
    pub client_public: Vec<u8>,
}

impl ServerParams {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(Error::Protocol("ARD params too short".into()));
        }
        let generator = u16::from_be_bytes([bytes[0], bytes[1]]);
        let key_len = u16::from_be_bytes([bytes[2], bytes[3]]) as usize;
        if key_len == 0 || key_len > 1024 || bytes.len() != 4 + 2 * key_len {
            return Err(Error::Protocol(format!(
                "ARD params: bad key length {key_len} for {} bytes",
                bytes.len()
            )));
        }
        Ok(Self {
            generator,
            key_len,
            prime: bytes[4..4 + key_len].to_vec(),
            server_public: bytes[4 + key_len..].to_vec(),
        })
    }
}

/// Build the response using a freshly generated random private key.
pub fn respond(params: &ServerParams, username: &str, password: &str) -> Result<ClientResponse> {
    let mut private = vec![0u8; params.key_len];
    rand::thread_rng().fill_bytes(&mut private);
    respond_with_private(params, username, password, &private, |buf| {
        rand::thread_rng().fill_bytes(buf)
    })
}

/// Deterministic core, split out so tests can pin every byte.
pub fn respond_with_private(
    params: &ServerParams,
    username: &str,
    password: &str,
    private: &[u8],
    mut fill_padding: impl FnMut(&mut [u8]),
) -> Result<ClientResponse> {
    let prime = BigUint::from_bytes_be(&params.prime);
    let generator = BigUint::from(params.generator);
    let server_public = BigUint::from_bytes_be(&params.server_public);
    let private = BigUint::from_bytes_be(private);

    if prime <= BigUint::from(1u8) {
        return Err(Error::Protocol("ARD prime is not > 1".into()));
    }

    let client_public = generator.modpow(&private, &prime);
    let shared = server_public.modpow(&private, &prime);

    let aes_key = Md5::digest(pad_be(&shared, params.key_len));

    let mut creds = [0u8; 128];
    fill_padding(&mut creds);
    write_cstr(&mut creds[0..64], username);
    write_cstr(&mut creds[64..128], password);

    let cipher = Aes128::new(GenericArray::from_slice(&aes_key));
    for block in creds.chunks_exact_mut(16) {
        cipher.encrypt_block(GenericArray::from_mut_slice(block));
    }

    Ok(ClientResponse {
        ciphertext: creds,
        client_public: pad_be(&client_public, params.key_len),
    })
}

/// Big-endian bytes of `n`, left-padded with zeros to exactly `len` bytes.
fn pad_be(n: &BigUint, len: usize) -> Vec<u8> {
    let bytes = n.to_bytes_be();
    if bytes.len() >= len {
        bytes[bytes.len() - len..].to_vec()
    } else {
        let mut out = vec![0u8; len - bytes.len()];
        out.extend_from_slice(&bytes);
        out
    }
}

/// Copy up to `dst.len() - 1` bytes of `s` followed by a NUL terminator.
fn write_cstr(dst: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let n = bytes.len().min(dst.len() - 1);
    dst[..n].copy_from_slice(&bytes[..n]);
    dst[n] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockDecrypt;

    fn small_params() -> ServerParams {
        // Toy group: p = 23, g = 5, server private = 15 -> public = 5^15 mod 23 = 19.
        ServerParams {
            generator: 5,
            key_len: 1,
            prime: vec![23],
            server_public: vec![19],
        }
    }

    #[test]
    fn parse_layout() {
        let mut bytes = vec![0x00, 0x02, 0x00, 0x03];
        bytes.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
        bytes.extend_from_slice(&[0x11, 0x22, 0x33]);
        let p = ServerParams::parse(&bytes).unwrap();
        assert_eq!(p.generator, 2);
        assert_eq!(p.key_len, 3);
        assert_eq!(p.prime, vec![0xaa, 0xbb, 0xcc]);
        assert_eq!(p.server_public, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn parse_rejects_wrong_length() {
        assert!(ServerParams::parse(&[0, 2, 0, 3, 1, 2, 3]).is_err());
        assert!(ServerParams::parse(&[0, 2, 0, 0]).is_err());
    }

    #[test]
    fn dh_math_matches_textbook() {
        // Client private = 6 -> public = 5^6 mod 23 = 8; shared = 19^6 mod 23 = 2.
        let resp = respond_with_private(&small_params(), "u", "p", &[6], |_| {}).unwrap();
        assert_eq!(resp.client_public, vec![8]);

        // Decrypt the credential block with the key the server would derive.
        let key = Md5::digest([2u8]);
        let cipher = Aes128::new(GenericArray::from_slice(&key));
        let mut plain = resp.ciphertext;
        for block in plain.chunks_exact_mut(16) {
            cipher.decrypt_block(GenericArray::from_mut_slice(block));
        }
        assert_eq!(&plain[0..2], b"u\0");
        assert_eq!(&plain[64..66], b"p\0");
    }

    #[test]
    fn credentials_are_nul_terminated_and_truncated() {
        let long = "x".repeat(100);
        let resp =
            respond_with_private(&small_params(), &long, &long, &[6], |b| b.fill(0xee)).unwrap();
        let key = Md5::digest([2u8]);
        let cipher = Aes128::new(GenericArray::from_slice(&key));
        let mut plain = resp.ciphertext;
        for block in plain.chunks_exact_mut(16) {
            cipher.decrypt_block(GenericArray::from_mut_slice(block));
        }
        assert!(plain[0..63].iter().all(|&b| b == b'x'));
        assert_eq!(plain[63], 0);
        assert!(plain[64..127].iter().all(|&b| b == b'x'));
        assert_eq!(plain[127], 0);
    }

    #[test]
    fn padding_is_random_not_zero() {
        let resp = respond_with_private(&small_params(), "u", "p", &[6], |b| b.fill(0x5a)).unwrap();
        let key = Md5::digest([2u8]);
        let cipher = Aes128::new(GenericArray::from_slice(&key));
        let mut plain = resp.ciphertext;
        for block in plain.chunks_exact_mut(16) {
            cipher.decrypt_block(GenericArray::from_mut_slice(block));
        }
        assert_eq!(plain[2], 0x5a);
        assert_eq!(plain[66], 0x5a);
    }

    #[test]
    fn pad_be_pads_and_truncates() {
        assert_eq!(pad_be(&BigUint::from(0x0102u32), 4), vec![0, 0, 1, 2]);
        assert_eq!(pad_be(&BigUint::from(0x01020304u32), 2), vec![3, 4]);
    }

    #[test]
    fn random_path_produces_well_formed_output() {
        let resp = respond(&small_params(), "user", "pass").unwrap();
        assert_eq!(resp.client_public.len(), 1);
        assert_eq!(resp.ciphertext.len(), 128);
    }
}
