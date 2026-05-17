use crate::manifest::FieldMapping;
use serde_json::Value;

/// Traverse a JSON value using a dot-notation path.
/// Handles both object keys ("foo.bar") and array indices ("foo.0.bar").
pub fn extract_value<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(json);
    }

    let mut current = json;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            Value::Array(arr) => {
                if let Ok(idx) = segment.parse::<usize>() {
                    current = arr.get(idx)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(current)
}

pub fn extract_str(json: &Value, path: &str) -> Option<String> {
    let val = extract_value(json, path)?;
    match val {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Extract artist information, handling potential arrays.
pub fn extract_artist(json: &Value, mapping: &FieldMapping) -> String {
    let val = extract_value(json, &mapping.artist);

    let separator = mapping.artist_separator.as_deref().unwrap_or(", ");

    match val {
        Some(Value::Array(arr)) => {
            let mut artists = Vec::new();
            for item in arr {
                match item {
                    Value::String(s) => artists.push(s.clone()),
                    Value::Object(_) => {
                        if let Some(field) = &mapping.artist_array_field {
                            if let Some(s) = extract_str(item, field) {
                                artists.push(s);
                            }
                        }
                    }
                    _ => {}
                }
            }
            if artists.is_empty() {
                "Unknown Artist".to_string()
            } else {
                artists.join(separator)
            }
        }
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                "Unknown Artist".to_string()
            } else {
                s.clone()
            }
        }
        _ => "Unknown Artist".to_string(),
    }
}

pub fn extract_title(json: &Value, mapping: &FieldMapping) -> String {
    extract_str(json, &mapping.title).unwrap_or_else(|| "Unknown".to_string())
}

/// Extract artwork URL, applying template if specified.
pub fn extract_artwork(json: &Value, mapping: &FieldMapping) -> Option<String> {
    let path = mapping.artwork_url.as_ref()?;
    let extracted = extract_str(json, path)?;

    if extracted.is_empty() {
        return None;
    }

    if let Some(template) = &mapping.artwork_url_template {
        Some(template.replace("{value}", &extracted))
    } else {
        Some(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_value_basic() {
        let json = serde_json::json!({
            "song": {
                "title": "Hello",
                "track": 5
            }
        });

        assert_eq!(extract_str(&json, "song.title"), Some("Hello".to_string()));
        assert_eq!(extract_str(&json, "song.track"), Some("5".to_string()));
        assert_eq!(extract_str(&json, "song.missing"), None);
    }

    #[test]
    fn test_extract_value_array() {
        let json = serde_json::json!({
            "albums": [
                { "image": "cover.jpg" },
                { "image": "back.jpg" }
            ],
            "tags": ["pop", "rock"]
        });

        assert_eq!(extract_str(&json, "albums.0.image"), Some("cover.jpg".to_string()));
        assert_eq!(extract_str(&json, "albums.1.image"), Some("back.jpg".to_string()));
        assert_eq!(extract_str(&json, "tags.0"), Some("pop".to_string()));
        assert_eq!(extract_str(&json, "tags.2"), None);
    }

    #[test]
    fn test_extract_artist_string() {
        let json = serde_json::json!({
            "artist": "John Doe"
        });
        let mapping = FieldMapping {
            title: "title".into(),
            artist: "artist".into(),
            artwork_url: None,
            artwork_url_template: None,
            artist_separator: None,
            artist_array_field: None,
        };
        assert_eq!(extract_artist(&json, &mapping), "John Doe");
    }

    #[test]
    fn test_extract_artist_array_of_strings() {
        let json = serde_json::json!({
            "artists": ["John", "Jane"]
        });
        let mapping = FieldMapping {
            title: "title".into(),
            artist: "artists".into(),
            artwork_url: None,
            artwork_url_template: None,
            artist_separator: Some(" & ".into()),
            artist_array_field: None,
        };
        assert_eq!(extract_artist(&json, &mapping), "John & Jane");
    }

    #[test]
    fn test_extract_artist_array_of_objects() {
        // LISTEN.moe style
        let json = serde_json::json!({
            "song": {
                "artists": [
                    { "name": "Artist1" },
                    { "name": "Artist2" }
                ]
            }
        });
        let mapping = FieldMapping {
            title: "title".into(),
            artist: "song.artists".into(),
            artwork_url: None,
            artwork_url_template: None,
            artist_separator: None, // defaults to ", "
            artist_array_field: Some("name".into()),
        };
        assert_eq!(extract_artist(&json, &mapping), "Artist1, Artist2");
    }

    #[test]
    fn test_extract_artwork_template() {
        let json = serde_json::json!({
            "cover": "image123.jpg"
        });
        let mapping = FieldMapping {
            title: "title".into(),
            artist: "artist".into(),
            artwork_url: Some("cover".into()),
            artwork_url_template: Some("https://cdn.example.com/{value}".into()),
            artist_separator: None,
            artist_array_field: None,
        };
        assert_eq!(
            extract_artwork(&json, &mapping),
            Some("https://cdn.example.com/image123.jpg".to_string())
        );
    }
}
