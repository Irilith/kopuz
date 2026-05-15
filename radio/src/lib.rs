use tokio::sync::mpsc;

pub mod listen_moe;

#[derive(Debug, Clone, PartialEq)]
pub struct RadioMetadata {
    pub title: String,
    pub artist: String,
    pub cover_url: Option<String>,
}

pub trait RadioMetadataProvider: Send + Sync {
    fn start(&self, stream_id: &str) -> mpsc::UnboundedReceiver<RadioMetadata>;
}
