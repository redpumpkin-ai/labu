//! Replay. Modul ini sengaja tidak mengimpor `reqwest` atau kredensial AWS:
//! ia tidak punya cara untuk memanggil jaringan (invariant "client-less replay").

use crate::cassette::{self, Entry};
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::Response;
use base64::Engine;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub struct ReplayState {
    entries: Mutex<HashMap<String, VecDeque<Entry>>>,
}

impl ReplayState {
    pub fn load(path: &str) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut map: HashMap<String, VecDeque<Entry>> = HashMap::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let e: Entry = serde_json::from_str(line)?;
            map.entry(e.key.clone()).or_default().push_back(e);
        }
        Ok(Self {
            entries: Mutex::new(map),
        })
    }
}

pub async fn handle(
    State(st): State<Arc<ReplayState>>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let key = cassette::key(method.as_str(), path, &body);

    // Request yang sama berulang: pakai urutan rekaman; entri terakhir dipakai ulang.
    let entry = {
        let mut map = st.entries.lock().unwrap();
        match map.get_mut(&key) {
            Some(q) if q.len() > 1 => q.pop_front(),
            Some(q) => q.front().cloned(),
            None => None,
        }
    };

    let Some(entry) = entry else {
        eprintln!("MISS {method} {path} key={key}");
        return Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"labu":"replay miss","key":"{key}"}}"#
            )))
            .unwrap();
    };

    eprintln!("HIT  {method} {path}");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&entry.body_b64)
        .expect("cassette body_b64 rusak");
    let mut resp = Response::builder().status(entry.status);
    for (k, v) in &entry.headers {
        resp = resp.header(k, v);
    }
    resp.body(Body::from(bytes)).unwrap()
}
