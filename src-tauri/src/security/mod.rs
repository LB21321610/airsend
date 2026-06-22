#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    pub code: String,
}

pub fn pairing_code_for_fingerprints(local: &str, remote: &str) -> PairingCode {
    let hash = blake3::hash(format!("{local}:{remote}").as_bytes());
    let numeric = u32::from_le_bytes(hash.as_bytes()[0..4].try_into().expect("slice size"));
    PairingCode {
        code: format!("{:06}", numeric % 1_000_000),
    }
}
