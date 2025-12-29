include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

pub fn asset(path: &str) -> Option<&'static EmbeddedAsset> {
    let normalized = path.trim_start_matches('/');
    if normalized.is_empty() {
        return None;
    }
    EMBEDDED_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
}

pub fn index_asset() -> Option<&'static EmbeddedAsset> {
    EMBEDDED_ASSETS
        .iter()
        .find(|asset| asset.path == "index.html")
}
