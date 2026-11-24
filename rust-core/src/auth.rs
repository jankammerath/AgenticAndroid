use base64::Engine;
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use qrcode::render::svg;
use qrcode::QrCode;
use rand::RngCore;
use sha1::Sha1;
use sha2::Sha256;

const PBKDF2_ITERATIONS: u32 = 100_000;
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn hash_password(password: &str) -> (String, String) {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut hash = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut hash);
    let b64 = base64::engine::general_purpose::STANDARD;
    (b64.encode(salt), b64.encode(hash))
}

pub fn verify_password(password: &str, salt_b64: &str, expected_hash_b64: &str) -> bool {
    let b64 = base64::engine::general_purpose::STANDARD;
    let Ok(salt) = b64.decode(salt_b64) else {
        return false;
    };
    let Ok(expected) = b64.decode(expected_hash_b64) else {
        return false;
    };
    let mut computed = vec![0u8; expected.len()];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut computed);
    constant_time_eq(&computed, &expected)
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_totp_secret() -> String {
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

pub fn build_otpauth_uri(username: &str, secret_base32: &str) -> String {
    let user = url_encode(if username.is_empty() { "admin" } else { username });
    format!(
        "otpauth://totp/AgenticAndroid:{user}?secret={secret_base32}&issuer=AgenticAndroid&algorithm=SHA1&digits=6&period=30"
    )
}

pub fn render_qr_svg(data: &str) -> Result<String, String> {
    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("QR error: {e}"))?;
    Ok(code
        .render::<svg::Color>()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#0b0d12"))
        .light_color(svg::Color("#ffffff"))
        .quiet_zone(true)
        .build())
}

pub fn verify_totp(secret_base32: &str, code: &str) -> bool {
    let clean: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() != 6 {
        return false;
    }
    let Ok(target_code) = clean.parse::<u32>() else {
        return false;
    };
    let Some(secret_bytes) = base32_decode(secret_base32) else {
        return false;
    };

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let step = now_secs / 30;

    // Allow +/- 1 time step (30s) for clock skew
    for delta in [-1i64, 0, 1] {
        let candidate_step = (step as i64 + delta).max(0) as u64;
        if compute_totp_at(&secret_bytes, candidate_step) == target_code {
            return true;
        }
    }
    false
}

fn compute_totp_at(secret: &[u8], counter: u64) -> u32 {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    binary % 1_000_000
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn base32_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits_left: u8 = 0;
    for &byte in data {
        buffer = (buffer << 8) | (byte as u32);
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            let idx = ((buffer >> bits_left) & 0x1f) as usize;
            out.push(BASE32_ALPHABET[idx] as char);
        }
    }
    if bits_left > 0 {
        let idx = ((buffer << (5 - bits_left)) & 0x1f) as usize;
        out.push(BASE32_ALPHABET[idx] as char);
    }
    out
}

fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_left: u8 = 0;
    for ch in input.chars() {
        if ch == '=' || ch.is_whitespace() || ch == '-' {
            continue;
        }
        let val = match ch.to_ascii_uppercase() {
            'A'..='Z' => (ch.to_ascii_uppercase() as u8 - b'A') as u32,
            '2'..='7' => (ch as u8 - b'2' + 26) as u32,
            _ => return None,
        };
        buffer = (buffer << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            out.push(((buffer >> bits_left) & 0xff) as u8);
        }
    }
    Some(out)
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
