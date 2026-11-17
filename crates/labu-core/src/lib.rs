//! Inti `labu`. Untuk saat ini hanya kerangka; logika record/replay
//! ditulis test-first di langkah 5 (lihat HANDOFF.md).

/// Versi `labu-core`, dibaca dari Cargo.toml.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }
}
