use aidoku::alloc::{string::String, vec::Vec};
use serde::Deserialize;

const SECRET: &[u8] = b"DEV_SCAN_SECRET_2026_change_me";

#[derive(Deserialize)]
pub struct ScanItemDecrypted {
    pub url: String,
}

/// Decrypt an encrypted scans string from happymh.com.
///
/// Algorithm (fully reverse-engineered from scandec.wasm):
///   1. sha256(SECRET + encStr[0:8] + domain) → derive key offsets
///   2. hex-decode key1 (32 B) and key2 (16 B) from the hex prefix
///   3. base64-decode the rest → ciphertext
///   4. SHA-256-CTR: ks_n = sha256(key1 || key2 || BE32(n)), XOR with ciphertext
///   5. Output must start with "SC01"; strip it and inflate (raw-deflate)
///   6. Parse JSON array of {url, ...}
pub fn decrypt_scans(enc_str: &str, domain: &str) -> Option<Vec<ScanItemDecrypted>> {
    if enc_str.len() < 128 {
        return None;
    }
    let enc = enc_str.as_bytes();

    // Step 1: derive offsets
    let mut h_input = Vec::with_capacity(SECRET.len() + 8 + domain.len());
    h_input.extend_from_slice(SECRET);
    h_input.extend_from_slice(&enc[..8]);
    h_input.extend_from_slice(domain.as_bytes());
    let sha = sha256(&h_input);

    let off0       = (sha[0] as usize % 24) + 16;
    let gap1       = (sha[1] as usize % 24) + 8;
    let gap2       = (sha[2] as usize % 24) + 8;
    let key2_start = off0 + 64 + gap1;
    let b64_start  = key2_start + 32 + gap2;

    if enc_str.len() <= b64_start {
        return None;
    }

    // Step 2: extract key1 (32 B) and key2 (16 B)
    let key1 = hex_decode(&enc_str[off0..off0 + 64])?;
    let key2 = hex_decode(&enc_str[key2_start..key2_start + 32])?;

    // Step 3: base64-decode ciphertext
    let ciphertext = base64_decode(&enc_str[b64_start..]);
    if ciphertext.len() < 4 {
        return None;
    }

    // Step 4: SHA-256-CTR decrypt
    let mut buf = [0u8; 52];
    buf[..32].copy_from_slice(&key1);
    buf[32..48].copy_from_slice(&key2);

    let mut output = alloc::vec![0u8; ciphertext.len()];
    let num_chunks = (ciphertext.len() + 31) / 32;
    for ctr in 0..num_chunks {
        buf[48..52].copy_from_slice(&(ctr as u32).to_be_bytes());
        let ks = sha256(&buf);
        let start = ctr * 32;
        let end = (start + 32).min(ciphertext.len());
        for j in 0..(end - start) {
            output[start + j] = ciphertext[start + j] ^ ks[j];
        }
    }

    // Step 5: verify "SC01" magic
    if &output[..4] != b"SC01" {
        return None;
    }

    // Step 6: raw-deflate inflate
    let inflated = miniz_oxide::inflate::decompress_to_vec(&output[4..]).ok()?;

    // Step 7: parse JSON
    serde_json::from_slice::<Vec<ScanItemDecrypted>>(&inflated).ok()
}

// ── Inline SHA-256 (no_std, pure Rust) ──────────────────────────────────────

fn sha256(data: &[u8]) -> [u8; 32] {
    #[rustfmt::skip]
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pad message: append 0x80, zeros, then 64-bit big-endian bit length
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let pad_start = data.len() + 1;
    let total_len = ((pad_start + 8 + 63) / 64) * 64;
    let mut msg = alloc::vec![0u8; total_len];
    msg[..data.len()].copy_from_slice(data);
    msg[data.len()] = 0x80;
    msg[total_len - 8..].copy_from_slice(&bit_len.to_be_bytes());

    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e;
            e = d.wrapping_add(t1);
            d = c; c = b; b = a;
            a = t1.wrapping_add(t2);
        }

        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ── Inline base64 decode (standard alphabet, no padding requirement) ─────────

fn base64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &b in s.as_bytes() {
        let v: u32 = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => continue,
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

// ── Inline hex decode ────────────────────────────────────────────────────────

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

#[inline]
fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
