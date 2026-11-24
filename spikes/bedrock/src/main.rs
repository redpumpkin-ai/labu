//! Spike Bedrock (HANDOFF langkah 3). Kode buangan, bukan fondasi v0.1.
//!
//!   bedrock-spike record   rekam ke cassette (butuh kredensial AWS)
//!   bedrock-spike replay   putar ulang dari cassette (tanpa kredensial, tanpa jaringan)
//!
//! Env: LABU_PORT (8787), LABU_CASSETTE (cassette.jsonl),
//!      LABU_UPSTREAM (default: endpoint Bedrock Runtime di region aktif).

mod cassette;
mod record;
mod replay;

use aws_credential_types::provider::ProvideCredentials;
use axum::Router;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let port = std::env::var("LABU_PORT").unwrap_or_else(|_| "8787".into());
    let cassette_path = std::env::var("LABU_CASSETTE").unwrap_or_else(|_| "cassette.jsonl".into());

    let app: Router = match mode.as_str() {
        "record" => {
            let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let region = cfg
                .region()
                .ok_or("region AWS tidak ditemukan (set AWS_REGION)")?
                .to_string();
            let creds = cfg
                .credentials_provider()
                .ok_or("kredensial AWS tidak ditemukan")?
                .provide_credentials()
                .await?;
            let upstream = std::env::var("LABU_UPSTREAM")
                .unwrap_or_else(|_| format!("https://bedrock-runtime.{region}.amazonaws.com"));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&cassette_path)?;
            eprintln!("record: upstream={upstream} region={region} cassette={cassette_path}");
            let state = Arc::new(record::RecordState {
                client: reqwest::Client::new(),
                upstream,
                region,
                identity: creds.into(),
                cassette: Mutex::new(file),
            });
            Router::new().fallback(record::handle).with_state(state)
        }
        "replay" => {
            let state = Arc::new(replay::ReplayState::load(&cassette_path)?);
            eprintln!("replay: cassette={cassette_path}");
            Router::new().fallback(replay::handle).with_state(state)
        }
        _ => return Err("pakai: bedrock-spike <record|replay>".into()),
    };

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    eprintln!("listening on http://127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
