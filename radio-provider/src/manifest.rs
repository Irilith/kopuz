use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_icon() -> String {
    "fa-solid fa-radio".to_string()
}

fn default_poll_secs() -> u64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationManifest {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub streams: Vec<StreamDef>,
    /// How to fetch now-playing metadata for this station
    pub metadata: Option<MetadataSourceDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDef {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub bitrate: Option<u32>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MetadataSourceDef {
    #[serde(rename = "websocket")]
    WebSocket(WebSocketSourceDef),
    #[serde(rename = "rest")]
    Rest(RestSourceDef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSourceDef {
    /// URL template, e.g. "wss://listen.moe/{stream_key}/gateway_v2"
    pub url: String,
    #[serde(default)]
    pub stream_url_map: HashMap<String, String>,
    #[serde(default)]
    pub message_filter: Option<WsMessageFilter>,
    #[serde(default)]
    pub heartbeat: Option<WsHeartbeat>,
    pub mapping: FieldMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessageFilter {
    pub op_field: String,
    pub op_value: u64,
    pub type_field: Option<String>,
    pub type_value: Option<String>,
    pub data_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsHeartbeat {
    pub message: String,
    pub interval_field: String,
    pub default_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestSourceDef {
    /// URL template: "https://api.example.com/{station_key}/now"
    pub url: String,
    #[serde(default)]
    pub station_key_map: HashMap<String, String>,
    #[serde(default = "default_poll_secs")]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub mapping: FieldMapping,
}

/// User-defined paths to extract metadata from JSON responses
/// Uses dot-notation: "song.title", "song.artists.0.name"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub artwork_url: Option<String>,
    #[serde(default)]
    pub artwork_url_template: Option<String>,
    #[serde(default)]
    pub artist_separator: Option<String>,
    #[serde(default)]
    pub artist_array_field: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("Manifest ID must contain only alphanumeric characters, underscores, and dashes")]
    InvalidId,
    #[error("Stream URLs must use https:// or wss:// scheme. Invalid URL: {0}")]
    InsecureUrl(String),
    #[error("Manifest must contain at least one stream")]
    NoStreams,
}

impl StationManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        // ID check
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ManifestError::InvalidId);
        }

        if self.streams.is_empty() {
            return Err(ManifestError::NoStreams);
        }

        for stream in &self.streams {
            if !stream.url.starts_with("https://") && !stream.url.starts_with("wss://") {
                return Err(ManifestError::InsecureUrl(stream.url.clone()));
            }
        }

        if let Some(meta) = &self.metadata {
            match meta {
                MetadataSourceDef::WebSocket(ws) => {
                    if !ws.url.starts_with("wss://") {
                        return Err(ManifestError::InsecureUrl(ws.url.clone()));
                    }
                }
                MetadataSourceDef::Rest(rest) => {
                    if !rest.url.starts_with("https://") {
                        return Err(ManifestError::InsecureUrl(rest.url.clone()));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest() {
        let json = r#"{
            "schema_version": "1.0",
            "id": "test_station",
            "name": "Test",
            "description": "Test description",
            "streams": [
                {
                    "id": "main",
                    "name": "Main",
                    "url": "https://example.com/stream"
                }
            ],
            "metadata": {
                "type": "rest",
                "url": "https://api.example.com",
                "mapping": {
                    "title": "title",
                    "artist": "artist"
                }
            }
        }"#;

        let manifest: StationManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_invalid_id() {
        let manifest = StationManifest {
            schema_version: "1.0".into(),
            id: "test station !".into(),
            name: "Test".into(),
            description: "Test".into(),
            icon: default_icon(),
            tags: vec![],
            streams: vec![StreamDef {
                id: "main".into(),
                name: "Main".into(),
                url: "https://example.com".into(),
                codec: None,
                bitrate: None,
                icon: None,
            }],
            metadata: None,
        };

        assert!(matches!(manifest.validate(), Err(ManifestError::InvalidId)));
    }

    #[test]
    fn test_insecure_url() {
        let json = r#"{
            "schema_version": "1.0",
            "id": "test",
            "name": "Test",
            "description": "Test",
            "streams": [
                {
                    "id": "main",
                    "name": "Main",
                    "url": "http://example.com/stream"
                }
            ]
        }"#;

        let manifest: StationManifest = serde_json::from_str(json).unwrap();
        assert!(matches!(
            manifest.validate(),
            Err(ManifestError::InsecureUrl(_))
        ));
    }
}
