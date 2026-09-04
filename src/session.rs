use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use librespot::core::cache::Cache;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::discovery::Credentials;
use librespot::oauth::OAuthClientBuilder;

const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const SPOTIFY_REDIRECT_URI: &str = "http://127.0.0.1:8898/login";
const CREDENTIALS_FILE: &str = "credentials.json";

fn credentials_store() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|path| path.join(".spotify-dl"))
        .context("Unable to locate the user home directory")
}

fn credentials_file() -> Result<PathBuf> {
    Ok(credentials_store()?.join(CREDENTIALS_FILE))
}

fn credentials_cache() -> Result<Cache> {
    Ok(Cache::new(Some(credentials_store()?), None, None, None)?)
}

async fn connect(credentials: Credentials, cache: Cache) -> Result<Session> {
    cache.save_credentials(&credentials);
    let session = Session::new(SessionConfig::default(), Some(cache));
    session.connect(credentials, true).await?;
    Ok(session)
}

pub async fn create_session() -> Result<Session> {
    let cache = credentials_cache()?;
    let credentials = match cache.credentials() {
        Some(creds) => creds,
        None => load_credentials()?,
    };
    connect(credentials, cache).await
}

/// Always starts Spotify's browser-based OAuth flow and replaces cached credentials.
pub async fn login() -> Result<Session> {
    let credentials = load_credentials()?;
    connect(credentials, credentials_cache()?).await
}

/// Reports whether a parseable reusable credential is present locally.
pub fn is_logged_in() -> Result<bool> {
    Ok(credentials_cache()?.credentials().is_some())
}

/// Removes only spotify-dl's cached credential file.
pub fn logout() -> Result<bool> {
    remove_credentials_file(&credentials_file()?)
}

fn remove_credentials_file(path: &Path) -> Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("Unable to remove cached Spotify credentials"),
    }
}

pub fn load_credentials() -> Result<Credentials> {
    let token = OAuthClientBuilder::new(SPOTIFY_CLIENT_ID, SPOTIFY_REDIRECT_URI, vec!["streaming"])
        .open_in_browser()
        .with_custom_message(
            "<html><body style=\"font-family:Segoe UI,sans-serif;background:#101316;color:#f2f6f3;padding:48px\"><h1>Spotify DL is connected</h1><p>You can close this tab and return to Spotify DL.</p></body></html>",
        )
        .build()?
        .get_access_token()?;
    Ok(Credentials::with_access_token(token.access_token))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{CREDENTIALS_FILE, remove_credentials_file};

    #[test]
    fn credential_filename_is_stable() {
        assert_eq!(CREDENTIALS_FILE, "credentials.json");
    }

    #[test]
    fn logout_removes_only_the_requested_credential_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("spotify-dl-auth-test-{unique}"));
        let credentials = directory.join(CREDENTIALS_FILE);
        let unrelated = directory.join("keep.txt");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&credentials, "test credential").unwrap();
        fs::write(&unrelated, "keep").unwrap();

        assert!(remove_credentials_file(&credentials).unwrap());
        assert!(!credentials.exists());
        assert!(unrelated.exists());
        assert!(!remove_credentials_file(&credentials).unwrap());

        fs::remove_dir_all(directory).unwrap();
    }
}
