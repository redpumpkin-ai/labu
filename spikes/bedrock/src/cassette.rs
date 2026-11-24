//! Format cassette spike: JSONL, satu respons per baris.
//! Body respons disimpan sebagai byte mentah (base64), tanpa di-parse.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Allowlist: hanya header respons ini yang boleh sampai ke disk.
pub const SAFE_RESPONSE_HEADERS: &[&str] = &["content-type", "x-amzn-errortype"];

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub v: u32,
    pub key: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body_b64: String,
}

/// Kunci matching: hash dari method + path + body request.
/// Header (termasuk tanda tangan SigV4) sengaja tidak ikut.
pub fn key(method: &str, path: &str, body: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(method.as_bytes());
    h.update(b"\n");
    h.update(path.as_bytes());
    h.update(b"\n");
    h.update(body);
    hex::encode(h.finalize())
}
