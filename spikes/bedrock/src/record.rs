//! Record: tanda tangani ulang request (SigV4) untuk host Bedrock asli,
//! teruskan, lalu simpan byte respons apa adanya sambil di-stream ke klien.

use crate::cassette::{self, Entry, SAFE_RESPONSE_HEADERS};
use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::identity::Identity;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use base64::Engine;
use futures_util::StreamExt;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio_stream::wrappers::ReceiverStream;

pub struct RecordState {
    pub client: reqwest::Client,
    /// Contoh: https://bedrock-runtime.us-east-1.amazonaws.com
    pub upstream: String,
    pub region: String,
    pub identity: Identity,
    pub cassette: Mutex<std::fs::File>,
}

fn bad_gateway(msg: String) -> Response {
    eprintln!("ERROR {msg}");
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .body(Body::from(msg))
        .unwrap()
}

pub async fn handle(
    State(st): State<Arc<RecordState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Path mentah dipertahankan (mis. `%3A` di modelId), karena ikut ditandatangani.
    let path = uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let url = format!("{}{}", st.upstream, path);
    let key = cassette::key(method.as_str(), &path, &body);

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    // Tanda tangan dari klien dibuang: itu dibuat untuk host proxy, bukan host AWS.
    let signed_headers = match sign_request(&st, &method, &url, &content_type, &body) {
        Ok(h) => h,
        Err(e) => return bad_gateway(format!("gagal menandatangani: {e}")),
    };

    let mut rb = st.client.request(method.clone(), &url).body(body);
    for (k, v) in &signed_headers {
        rb = rb.header(k, v);
    }
    let upstream = match rb.send().await {
        Ok(r) => r,
        Err(e) => return bad_gateway(format!("gagal menghubungi upstream: {e}")),
    };

    let status = upstream.status();
    let mut kept = BTreeMap::new();
    for name in SAFE_RESPONSE_HEADERS {
        if let Some(v) = upstream.headers().get(*name).and_then(|v| v.to_str().ok()) {
            kept.insert(name.to_string(), v.to_string());
        }
    }

    // Stream ke klien lewat channel, sambil mengumpulkan byte untuk cassette.
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(16);
    let entry_headers = kept.clone();
    let st2 = st.clone();
    let method_s = method.to_string();
    tokio::spawn(async move {
        let mut all: Vec<u8> = Vec::new();
        let mut stream = upstream.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(b) => {
                    all.extend_from_slice(&b);
                    let _ = tx.send(Ok(b)).await;
                }
                Err(e) => {
                    // Stream putus: jangan simpan rekaman setengah jadi.
                    eprintln!("ERROR stream upstream putus: {e}");
                    let _ = tx.send(Err(std::io::Error::other(e))).await;
                    return;
                }
            }
        }
        let entry = Entry {
            v: 1,
            key,
            method: method_s,
            path,
            status: status.as_u16(),
            headers: entry_headers,
            body_b64: base64::engine::general_purpose::STANDARD.encode(&all),
        };
        let line = serde_json::to_string(&entry).unwrap();
        let mut f = st2.cassette.lock().unwrap();
        writeln!(f, "{line}").expect("gagal menulis cassette");
        eprintln!(
            "REC  {} {} status={} bytes={}",
            entry.method,
            entry.path,
            entry.status,
            all.len()
        );
    });

    let mut resp = Response::builder().status(status);
    for (k, v) in &kept {
        resp = resp.header(k, v);
    }
    resp.body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap()
}

/// Mengembalikan header yang harus dikirim ke upstream (termasuk Authorization baru).
fn sign_request(
    st: &RecordState,
    method: &Method,
    url: &str,
    content_type: &str,
    body: &[u8],
) -> Result<http::HeaderMap, Box<dyn std::error::Error>> {
    let params = v4::SigningParams::builder()
        .identity(&st.identity)
        .region(&st.region)
        .name("bedrock")
        .time(SystemTime::now())
        .settings(SigningSettings::default())
        .build()?
        .into();
    let signable = SignableRequest::new(
        method.as_str(),
        url,
        std::iter::once(("content-type", content_type)),
        SignableBody::Bytes(body),
    )?;
    let (instructions, _signature) = sign(signable, &params)?.into_parts();

    let mut req = http::Request::builder()
        .method(method.as_str())
        .uri(url)
        .header("content-type", content_type)
        .body(())?;
    instructions.apply_to_request_http1x(&mut req);
    Ok(req.headers().clone())
}
