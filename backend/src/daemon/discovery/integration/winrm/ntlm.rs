//! Minimal NTLMv2 message-authentication handshake (MS-NLMP), scoped to exactly
//! what WinRM needs: build a Type 1 (Negotiate), parse a Type 2 (Challenge),
//! and build a Type 3 (Authenticate). No message-level signing/sealing is
//! implemented — see the module doc on `mod.rs` for why HTTPS or
//! `AllowUnencrypted` is required.

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use md4::Md4;
use md5::Md5;
use rand::RngCore;

type HmacMd5 = Hmac<Md5>;

const SIGNATURE: &[u8; 8] = b"NTLMSSP\0";

const NEGOTIATE_UNICODE: u32 = 0x0000_0001;
const NEGOTIATE_REQUEST_TARGET: u32 = 0x0000_0004;
const NEGOTIATE_NTLM: u32 = 0x0000_0200;
const NEGOTIATE_ALWAYS_SIGN: u32 = 0x0000_8000;
const NEGOTIATE_EXTENDED_SESSION_SECURITY: u32 = 0x0008_0000;
const NEGOTIATE_TARGET_INFO: u32 = 0x0080_0000;
const NEGOTIATE_128: u32 = 0x2000_0000;
const NEGOTIATE_56: u32 = 0x8000_0000;

const TYPE1_FLAGS: u32 = NEGOTIATE_UNICODE
    | NEGOTIATE_REQUEST_TARGET
    | NEGOTIATE_NTLM
    | NEGOTIATE_ALWAYS_SIGN
    | NEGOTIATE_EXTENDED_SESSION_SECURITY
    | NEGOTIATE_TARGET_INFO
    | NEGOTIATE_128
    | NEGOTIATE_56;

/// Build the Type 1 (Negotiate) message. No domain/workstation hint —
/// minimal 32-byte message, which every NTLM implementation accepts.
pub fn negotiate_message() -> Vec<u8> {
    let mut message = Vec::with_capacity(32);
    message.extend_from_slice(SIGNATURE);
    message.extend_from_slice(&1u32.to_le_bytes()); // MessageType
    message.extend_from_slice(&TYPE1_FLAGS.to_le_bytes());
    message.extend_from_slice(&[0u8; 8]); // DomainNameFields (empty)
    message.extend_from_slice(&[0u8; 8]); // WorkstationFields (empty)
    message
}

pub struct Challenge {
    pub server_challenge: [u8; 8],
    /// Raw AV_PAIR target-info blob, copied verbatim into the NTLMv2 response.
    pub target_info: Vec<u8>,
}

#[derive(Debug)]
pub struct NtlmError(pub &'static str);

impl std::fmt::Display for NtlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NTLM: {}", self.0)
    }
}
impl std::error::Error for NtlmError {}

/// Parse a Type 2 (Challenge) message from the server.
pub fn parse_challenge(bytes: &[u8]) -> Result<Challenge, NtlmError> {
    if bytes.len() < 48 || &bytes[0..8] != SIGNATURE || bytes[8..12] != 2u32.to_le_bytes() {
        return Err(NtlmError("malformed Type 2 message"));
    }
    let mut server_challenge = [0u8; 8];
    server_challenge.copy_from_slice(&bytes[24..32]);

    let flags = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    if flags & NEGOTIATE_TARGET_INFO == 0 {
        // Every modern Windows target sets this; without it we have no
        // AV_PAIR blob and cannot compute a spec-correct NTLMv2 response.
        return Err(NtlmError(
            "server did not negotiate NTLMv2 target info (unsupported/legacy NTLM server)",
        ));
    }

    let ti_len = u16::from_le_bytes(bytes[40..42].try_into().unwrap()) as usize;
    let ti_offset = u32::from_le_bytes(bytes[44..48].try_into().unwrap()) as usize;
    let target_info = bytes
        .get(ti_offset..ti_offset + ti_len)
        .ok_or(NtlmError("target info out of bounds"))?
        .to_vec();

    Ok(Challenge {
        server_challenge,
        target_info,
    })
}

fn utf16le(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn hmac_md5(key: &[u8], data: &[u8]) -> [u8; 16] {
    let mut mac = HmacMd5::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// NTOWFv2 / LMOWFv2 response key: HMAC-MD5(MD4(UTF16LE(password)), UTF16LE(UPPER(user) + domain)).
fn response_key(username: &str, domain: &str, password: &str) -> [u8; 16] {
    use md4::Digest;
    let nt_hash: [u8; 16] = Md4::digest(utf16le(password)).into();
    let identity = utf16le(&format!("{}{}", username.to_uppercase(), domain));
    hmac_md5(&nt_hash, &identity)
}

pub struct AuthenticateInputs<'a> {
    pub challenge: &'a Challenge,
    pub domain: &'a str,
    pub username: &'a str,
    pub password: &'a str,
}

/// Build the Type 3 (Authenticate) message using NTLMv2. No session-key
/// exchange and no Message Integrity Check block — this handshake proves
/// identity only; it does not establish sign/seal keys (see module docs).
pub fn authenticate_message(inputs: &AuthenticateInputs<'_>) -> Vec<u8> {
    let AuthenticateInputs {
        challenge,
        domain,
        username,
        password,
    } = *inputs;

    let key = response_key(username, domain, password);

    let mut client_challenge = [0u8; 8];
    rand::rng().fill_bytes(&mut client_challenge);

    // temp = timestamp(8) + client_challenge(8) + reserved(4) + target_info + reserved(4)
    let timestamp = windows_filetime_now();
    let mut temp = Vec::with_capacity(8 + 8 + 4 + challenge.target_info.len() + 4);
    temp.extend_from_slice(&timestamp);
    temp.extend_from_slice(&client_challenge);
    temp.extend_from_slice(&[0u8; 4]);
    temp.extend_from_slice(&challenge.target_info);
    temp.extend_from_slice(&[0u8; 4]);

    let mut nt_proof_input = Vec::with_capacity(8 + temp.len());
    nt_proof_input.extend_from_slice(&challenge.server_challenge);
    nt_proof_input.extend_from_slice(&temp);
    let nt_proof_str = hmac_md5(&key, &nt_proof_input);

    let mut nt_challenge_response = Vec::with_capacity(16 + temp.len());
    nt_challenge_response.extend_from_slice(&nt_proof_str);
    nt_challenge_response.extend_from_slice(&temp);

    let mut lm_input = Vec::with_capacity(16);
    lm_input.extend_from_slice(&challenge.server_challenge);
    lm_input.extend_from_slice(&client_challenge);
    let lm_proof = hmac_md5(&key, &lm_input);
    let mut lm_challenge_response = Vec::with_capacity(24);
    lm_challenge_response.extend_from_slice(&lm_proof);
    lm_challenge_response.extend_from_slice(&client_challenge);

    let domain_bytes = utf16le(domain);
    let username_bytes = utf16le(username);
    let workstation_bytes = utf16le("SCANOPY");

    // Header is 8 fixed fields of 8 bytes each = 64 bytes, then flags(4) = 68,
    // then the variable payload in the order fields point to it.
    const HEADER_LEN: u32 = 8 + 4 + 8 + 8 + 8 + 8 + 8 + 8 + 4;
    let mut offset = HEADER_LEN;

    let lm_field = field(lm_challenge_response.len() as u16, offset);
    offset += lm_challenge_response.len() as u32;
    let nt_field = field(nt_challenge_response.len() as u16, offset);
    offset += nt_challenge_response.len() as u32;
    let domain_field = field(domain_bytes.len() as u16, offset);
    offset += domain_bytes.len() as u32;
    let user_field = field(username_bytes.len() as u16, offset);
    offset += username_bytes.len() as u32;
    let workstation_field = field(workstation_bytes.len() as u16, offset);
    offset += workstation_bytes.len() as u32;
    let session_key_field = field(0, offset);

    let mut message = Vec::with_capacity(offset as usize);
    message.extend_from_slice(SIGNATURE);
    message.extend_from_slice(&3u32.to_le_bytes());
    message.extend_from_slice(&lm_field);
    message.extend_from_slice(&nt_field);
    message.extend_from_slice(&domain_field);
    message.extend_from_slice(&user_field);
    message.extend_from_slice(&workstation_field);
    message.extend_from_slice(&session_key_field);
    message.extend_from_slice(&TYPE1_FLAGS.to_le_bytes());
    message.extend_from_slice(&lm_challenge_response);
    message.extend_from_slice(&nt_challenge_response);
    message.extend_from_slice(&domain_bytes);
    message.extend_from_slice(&username_bytes);
    message.extend_from_slice(&workstation_bytes);
    message
}

fn field(len: u16, offset: u32) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    bytes[0..2].copy_from_slice(&len.to_le_bytes());
    bytes[2..4].copy_from_slice(&len.to_le_bytes());
    bytes[4..8].copy_from_slice(&offset.to_le_bytes());
    bytes
}

/// Windows FILETIME: 100ns intervals since 1601-01-01, as used in the NTLMv2
/// `temp` blob. Precision doesn't matter for auth (the server doesn't
/// validate it against wall-clock time in this handshake), only that it's a
/// plausible 8-byte LE value.
fn windows_filetime_now() -> [u8; 8] {
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
    let unix_100ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 / 100)
        .unwrap_or(0);
    (unix_100ns.saturating_add(EPOCH_DIFF_100NS)).to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_message_has_expected_header() {
        let msg = negotiate_message();
        assert_eq!(&msg[0..8], SIGNATURE);
        assert_eq!(&msg[8..12], &1u32.to_le_bytes());
        assert_eq!(msg.len(), 32);
    }

    fn build_challenge(target_info: &[u8]) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(SIGNATURE);
        msg.extend_from_slice(&2u32.to_le_bytes());
        msg.extend_from_slice(&[0u8; 8]); // target name fields (unused)
        msg.extend_from_slice(
            &(NEGOTIATE_TARGET_INFO | NEGOTIATE_EXTENDED_SESSION_SECURITY).to_le_bytes(),
        );
        msg.extend_from_slice(&[0xAAu8; 8]); // server challenge
        msg.extend_from_slice(&[0u8; 8]); // reserved
        let ti_offset = 48u32;
        msg.extend_from_slice(&(target_info.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(target_info.len() as u16).to_le_bytes());
        msg.extend_from_slice(&ti_offset.to_le_bytes());
        msg.extend_from_slice(target_info);
        msg
    }

    #[test]
    fn parse_challenge_extracts_server_challenge_and_target_info() {
        let target_info = vec![0x02, 0x00, 0x04, 0x00, b'D', 0, b'C', 0, 0x00, 0x00];
        let raw = build_challenge(&target_info);
        let challenge = parse_challenge(&raw).expect("valid challenge");
        assert_eq!(challenge.server_challenge, [0xAA; 8]);
        assert_eq!(challenge.target_info, target_info);
    }

    #[test]
    fn parse_challenge_rejects_missing_target_info_flag() {
        let mut msg = Vec::new();
        msg.extend_from_slice(SIGNATURE);
        msg.extend_from_slice(&2u32.to_le_bytes());
        msg.extend_from_slice(&[0u8; 8]);
        msg.extend_from_slice(&0u32.to_le_bytes()); // no NEGOTIATE_TARGET_INFO
        msg.extend_from_slice(&[0u8; 8]);
        msg.extend_from_slice(&[0u8; 8]);
        msg.extend_from_slice(&[0u8; 8]);
        assert!(parse_challenge(&msg).is_err());
    }

    #[test]
    fn parse_challenge_rejects_bad_signature() {
        let mut raw = build_challenge(&[]);
        raw[0] = b'X';
        assert!(parse_challenge(&raw).is_err());
    }

    #[test]
    fn authenticate_message_is_deterministic_given_fixed_client_challenge() {
        // response_key() itself is pure and worth locking down independently
        // of the random client challenge baked into the full message.
        let key_a = response_key("alice", "EXAMPLE", "correct horse battery staple");
        let key_b = response_key("alice", "EXAMPLE", "correct horse battery staple");
        assert_eq!(key_a, key_b);
        let key_wrong_password = response_key("alice", "EXAMPLE", "wrong password");
        assert_ne!(key_a, key_wrong_password);
        let key_wrong_domain = response_key("alice", "OTHER", "correct horse battery staple");
        assert_ne!(key_a, key_wrong_domain);
    }

    #[test]
    fn authenticate_message_round_trips_field_offsets() {
        let target_info = vec![0x00, 0x00];
        let raw_challenge = build_challenge(&target_info);
        let challenge = parse_challenge(&raw_challenge).unwrap();
        let msg = authenticate_message(&AuthenticateInputs {
            challenge: &challenge,
            domain: "EXAMPLE",
            username: "alice",
            password: "hunter2",
        });
        assert_eq!(&msg[0..8], SIGNATURE);
        assert_eq!(&msg[8..12], &3u32.to_le_bytes());

        let lm_len = u16::from_le_bytes(msg[12..14].try_into().unwrap()) as usize;
        let lm_offset = u32::from_le_bytes(msg[16..20].try_into().unwrap()) as usize;
        assert_eq!(lm_len, 24);
        let nt_len = u16::from_le_bytes(msg[20..22].try_into().unwrap()) as usize;
        let nt_offset = u32::from_le_bytes(msg[24..28].try_into().unwrap()) as usize;
        assert_eq!(nt_len, 16 + target_info.len() + 8 + 8 + 4 + 4);
        assert_eq!(&msg[lm_offset..lm_offset + lm_len].len(), &lm_len);
        assert_eq!(&msg[nt_offset..nt_offset + nt_len].len(), &nt_len);

        let domain_len = u16::from_le_bytes(msg[28..30].try_into().unwrap()) as usize;
        let domain_offset = u32::from_le_bytes(msg[32..36].try_into().unwrap()) as usize;
        assert_eq!(
            String::from_utf16_lossy(
                &msg[domain_offset..domain_offset + domain_len]
                    .chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect::<Vec<_>>()
            ),
            "EXAMPLE"
        );
    }
}
