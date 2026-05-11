use super::models::{Album, Library, Track};
use super::utils::{find_folder_cover, save_cover};
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::ItemKey;
use lofty::{file::TaggedFile, probe::Probe, properties::FileProperties, tag::Tag};
use std::path::Path;

fn slugify_album_key(value: &str) -> String {
    value
        .to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
}

pub fn make_album_id(album: &str, grouping_key: &str) -> String {
    let normalized_album = album.trim();

    if !normalized_album.is_empty() {
        return format!("alb_{}", slugify_album_key(normalized_album));
    }

    let fallback = slugify_album_key(grouping_key);
    if fallback.is_empty() {
        "alb_unknown".to_string()
    } else {
        format!("alb_unknown_{fallback}")
    }
}

pub fn extract_embedded_cover(tagged_file: &TaggedFile, tag: Option<&Tag>) -> Option<Vec<u8>> {
    tag.and_then(|tag| tag.pictures().first())
        .or_else(|| {
            tagged_file
                .tags()
                .iter()
                .find_map(|tag| tag.pictures().first())
        })
        .map(|pic| pic.data().to_vec())
}

pub fn extract_metadata(
    tag: Option<&Tag>,
    properties: &FileProperties,
    track_path: &Path,
) -> Track {
    let artist = tag
        .and_then(|t| t.artist().map(|a| a.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let artists: Vec<String> = tag
        .map(|t| {
            let from_tag: Vec<String> = t
                .get_strings(&ItemKey::TrackArtists)
                .flat_map(|s| s.split(';').map(|a| a.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
            if !from_tag.is_empty() {
                from_tag
            } else if artist.contains(';') {
                artist
                    .split(';')
                    .map(|a| a.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![artist.clone()]
            }
        })
        .unwrap_or_else(|| vec![artist.clone()]);

    let album_title = tag.and_then(|t| t.album().map(|a| a.to_string()));

    let album_artist = tag
        .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
        .map(|s| s.to_string());

    let parent_path = track_path.parent().map(|p| p.to_string_lossy());
    let grouping_key = album_artist
        .as_deref()
        .or(parent_path.as_deref())
        .unwrap_or(&artist);

    let title = tag
        .and_then(|t| t.title().map(|t| t.to_string()))
        .or_else(|| {
            track_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Unknown Title".to_string());

    let musicbrainz_release_id = tag
        .and_then(|t| t.get_string(&ItemKey::MusicBrainzReleaseId))
        .map(|s| s.to_string());

    Track {
        path: track_path.to_path_buf(),
        album_id: make_album_id(album_title.as_deref().unwrap_or(""), grouping_key),
        title,
        artist,
        artists,
        album: album_title.unwrap_or_else(|| "Unknown Album".to_string()),
        khz: properties.sample_rate().unwrap_or(0),
        bitrate: properties.bit_depth().unwrap_or(0),
        duration: properties.duration().as_secs()
            + u64::from(properties.duration().subsec_nanos() > 0),
        track_number: tag.and_then(|t| t.track()),
        disc_number: tag.and_then(|t| t.disk()),
        musicbrainz_release_id,
        playlist_item_id: None,
    }
}

pub fn read(track_path: &Path, cover_cache: &Path, library: &mut Library) -> Option<Track> {
    let tagged_file = Probe::open(track_path).ok()?.read().ok()?;
    let properties = tagged_file.properties();
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let track = extract_metadata(tag, properties, track_path);
    let album_id = track.album_id.clone();

    let album_artist = tag
        .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
        .map(|s| s.to_string())
        .unwrap_or_else(|| track.artist.clone());

    let album_exists = library.albums.iter().any(|a| a.id == album_id);

    if !album_exists {
        let mut cover = None;

        if let Some(bytes) = extract_embedded_cover(&tagged_file, tag) {
            cover = save_cover(&album_id, &bytes, cover_cache).ok();
        } else if let Some(folder_cover) = find_folder_cover(track_path.parent()?) {
            cover = Some(folder_cover);
        }

        let genre = tag
            .and_then(|t| t.genre().map(|g| g.to_string()))
            .unwrap_or_else(|| "Unknown".to_string());

        let year = tag.and_then(|t| t.year()).unwrap_or(0) as u16;

        library.add_album(Album {
            id: album_id.clone(),
            title: track.album.clone(),
            artist: album_artist,
            genre,
            year,
            cover_path: cover,
        });
    }

    library.add_track(track.clone());
    Some(track)
}
