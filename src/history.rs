use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use librespot::core::SpotifyUri;
use serde::{Deserialize, Serialize};

pub const HISTORY_FILE: &str = ".spotify-dl-history.json";

#[derive(Debug, Default, Deserialize, Serialize)]
struct StoredHistory {
    playlists: HashMap<String, BTreeSet<String>>,
}

pub struct PlaylistHistory {
    path: PathBuf,
    data: StoredHistory,
}

impl PlaylistHistory {
    pub fn load(destination: &Path) -> Self {
        let path = destination.join(HISTORY_FILE);
        let data = fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
        Self { path, data }
    }

    pub fn reset(destination: &Path) -> Result<bool> {
        let path = destination.join(HISTORY_FILE);
        match fs::remove_file(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub fn contains(&self, playlist: &SpotifyUri, track: &SpotifyUri) -> bool {
        uri(playlist)
            .zip(uri(track))
            .and_then(|(playlist, track)| {
                self.data
                    .playlists
                    .get(&playlist)
                    .map(|set| set.contains(&track))
            })
            .unwrap_or(false)
    }

    pub fn record(&mut self, playlist: &SpotifyUri, track: &SpotifyUri) -> Result<()> {
        if let Some((playlist, track)) = uri(playlist).zip(uri(track)) {
            self.data
                .playlists
                .entry(playlist)
                .or_default()
                .insert(track);
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&self.path, serde_json::to_vec_pretty(&self.data)?)?;
        }
        Ok(())
    }
}

fn uri(value: &SpotifyUri) -> Option<String> {
    value.to_uri().ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use librespot::core::SpotifyUri;

    use super::{HISTORY_FILE, PlaylistHistory};

    #[test]
    fn records_loads_and_resets_playlist_history() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let destination = std::env::temp_dir().join(format!("spotify-dl-history-{unique}"));
        let playlist = SpotifyUri::from_uri("spotify:playlist:5nR8SY0dIak6Vt9KHxI8T7").unwrap();
        let track = SpotifyUri::from_uri("spotify:track:6oUGAx0vkBcnGzYkvw0ZsA").unwrap();

        let mut history = PlaylistHistory::load(&destination);
        assert!(!history.contains(&playlist, &track));
        history.record(&playlist, &track).unwrap();
        assert!(destination.join(HISTORY_FILE).exists());
        assert!(PlaylistHistory::load(&destination).contains(&playlist, &track));
        assert!(PlaylistHistory::reset(&destination).unwrap());
        assert!(!PlaylistHistory::reset(&destination).unwrap());

        fs::remove_dir_all(destination).unwrap();
    }
}
