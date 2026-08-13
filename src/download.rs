use std::fmt::Write;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use futures::TryStreamExt;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use librespot::core::session::Session;
use tokio::sync::Mutex;

use crate::encoder;
use crate::encoder::Format;
use crate::encoder::Samples;
use crate::history::PlaylistHistory;
use crate::stream::Stream;
use crate::stream::StreamError;
use crate::stream::StreamEvent;
use crate::stream::StreamEventChannel;
use crate::track::Track;
use crate::track::TrackMetadata;

const MAX_TRACK_RETRIES: usize = 3;
const RETRY_PAUSE: Duration = Duration::from_secs(5 * 60);

pub struct Downloader {
    session: Session,
    progress_bar: MultiProgress,
    history: Option<Arc<Mutex<PlaylistHistory>>>,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub destination: PathBuf,
    pub parallel: usize,
    pub format: Format,
    pub force: bool,
    pub machine_readable: bool,
    pub playlist_track_numbers: bool,
    pub playlist_sync: bool,
    pub realistic_delay: bool,
}

impl DownloadOptions {
    pub fn new(
        destination: Option<String>,
        parallel: usize,
        format: Format,
        force: bool,
        machine_readable: bool,
        playlist_track_numbers: bool,
        playlist_sync: bool,
        realistic_delay: bool,
    ) -> Self {
        let destination =
            destination.map_or_else(|| std::env::current_dir().unwrap(), PathBuf::from);
        DownloadOptions {
            destination,
            parallel,
            format,
            force,
            machine_readable,
            playlist_track_numbers,
            playlist_sync,
            realistic_delay,
        }
    }
}

impl Downloader {
    pub fn new(session: Session, history: Option<Arc<Mutex<PlaylistHistory>>>) -> Self {
        Downloader {
            session,
            progress_bar: MultiProgress::new(),
            history,
        }
    }

    pub async fn download_tracks(
        self,
        tracks: Vec<Track>,
        options: &DownloadOptions,
    ) -> Result<()> {
        emit_machine_event(
            options.machine_readable,
            "QUEUE",
            &[tracks.len().to_string()],
        );
        futures::stream::iter(tracks)
            .map(|track| self.download_track(track, options))
            .buffer_unordered(options.parallel)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    #[tracing::instrument(name = "download_track", skip(self))]
    async fn download_track(&self, track: Track, options: &DownloadOptions) -> Result<()> {
        if options.playlist_sync && self.was_downloaded(&track).await {
            emit_machine_event(
                options.machine_readable,
                "ITEM_SYNC_SKIP",
                &[track
                    .uri
                    .to_uri()
                    .unwrap_or_else(|_| format!("{:?}", track.uri))],
            );
            return Ok(());
        }

        let metadata = match track.metadata(&self.session).await {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::warn!(%error, "Skipping a track whose metadata is unavailable");
                emit_machine_event(
                    options.machine_readable,
                    "ITEM_UNAVAILABLE",
                    &[track
                        .uri
                        .to_uri()
                        .unwrap_or_else(|_| format!("{:?}", track.uri))],
                );
                return Ok(());
            }
        };
        tracing::info!("Downloading track: {:?}", metadata.track_name);
        let display_name = metadata.to_string();

        let path = output_path(
            &options.destination,
            &metadata.to_string(),
            options.format.extension(),
        )
        .to_str()
        .ok_or(anyhow::anyhow!("Could not set the output path"))?
        .to_string();

        if !options.force && PathBuf::from(&path).exists() {
            tracing::info!(
                "Skipping {}, file already exists. Use --force to force re-downloading the track",
                &metadata.track_name
            );
            emit_machine_event(options.machine_readable, "ITEM_SKIP", &[display_name]);
            self.mark_downloaded(&track, options.playlist_sync).await;
            return Ok(());
        }

        emit_machine_event(
            options.machine_readable,
            "ITEM_START",
            &[display_name.clone(), metadata.approx_size().to_string()],
        );

        let pb = self.add_progress_bar(&metadata);

        let mut retry = 0;
        let samples = loop {
            let session = if retry == 0 {
                Ok(self.session.clone())
            } else {
                crate::session::create_session().await
            };
            let result = match session {
                Ok(session) => match Stream::new(session).stream(Arc::new(track.clone())).await {
                    Ok(channel) => {
                        self.buffer_track(channel, &pb, &metadata, options.machine_readable)
                            .await
                    }
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            };

            match result {
                Ok(samples) => break samples,
                Err(error)
                    if error
                        .downcast_ref::<StreamError>()
                        .is_some_and(|error| matches!(error, StreamError::Unavailable)) =>
                {
                    pb.finish_with_message(format!("Unavailable {}", display_name));
                    emit_machine_event(
                        options.machine_readable,
                        "ITEM_UNAVAILABLE",
                        &[display_name],
                    );
                    return Ok(());
                }
                Err(error) if retry < MAX_TRACK_RETRIES => {
                    retry += 1;
                    tracing::warn!(
                        "Track attempt failed; waiting {} seconds before retry {} of {}: {} ({})",
                        RETRY_PAUSE.as_secs(),
                        retry,
                        MAX_TRACK_RETRIES,
                        display_name,
                        error
                    );
                    pb.set_message(format!(
                        "Waiting 5 minutes before retry ({}/{}) {}",
                        retry, MAX_TRACK_RETRIES, display_name
                    ));
                    emit_machine_event(
                        options.machine_readable,
                        "ITEM_RETRY",
                        &[
                            display_name.clone(),
                            retry.to_string(),
                            MAX_TRACK_RETRIES.to_string(),
                            RETRY_PAUSE.as_secs().to_string(),
                        ],
                    );
                    tokio::time::sleep(RETRY_PAUSE).await;
                    emit_machine_event(
                        options.machine_readable,
                        "ITEM_RETRY_RESUME",
                        &[display_name.clone(), retry.to_string()],
                    );
                }
                Err(error) => {
                    self.fail_with_error(
                        &pb,
                        &display_name,
                        error.to_string(),
                        options.machine_readable,
                    );
                    return Ok(());
                }
            }
        };

        tracing::info!("Encoding track: {}", metadata.to_string());
        pb.set_message(format!("Encoding {}", metadata.to_string()));
        emit_machine_event(
            options.machine_readable,
            "ITEM_STAGE",
            &[display_name.clone(), "encoding".to_string()],
        );

        let encoder = crate::encoder::get_encoder(options.format);
        let stream = encoder.encode(samples).await?;

        pb.set_message(format!("Writing {}", metadata.to_string()));
        emit_machine_event(
            options.machine_readable,
            "ITEM_STAGE",
            &[display_name.clone(), "writing".to_string()],
        );
        tracing::info!(
            "Writing track: {:?} to file: {}",
            metadata.to_string(),
            &path
        );
        stream.write_to_file(&path).await?;

        let mut tags = metadata.tags().await?;
        apply_playlist_position(&mut tags, &track, options.playlist_track_numbers);
        encoder::tags::store_tags(path, &tags, options.format).await?;

        pb.finish_with_message(format!("Downloaded {}", metadata.to_string()));
        emit_machine_event(
            options.machine_readable,
            "ITEM_DONE",
            &[display_name.clone()],
        );
        self.mark_downloaded(&track, options.playlist_sync).await;
        if options.realistic_delay && options.parallel == 1 {
            let delay = Duration::from_millis((metadata.duration.max(0) as u64) / 5);
            emit_machine_event(
                options.machine_readable,
                "PACING_WAIT",
                &[display_name, delay.as_secs().to_string()],
            );
            tokio::time::sleep(delay).await;
        }
        Ok(())
    }

    fn add_progress_bar(&self, track: &TrackMetadata) -> ProgressBar {
        let pb = self
            .progress_bar
            .add(ProgressBar::new(track.approx_size() as u64));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            // Infallible
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));
        pb.set_message(track.to_string());
        pb
    }

    async fn buffer_track(
        &self,
        mut rx: StreamEventChannel,
        pb: &ProgressBar,
        metadata: &TrackMetadata,
        machine_readable: bool,
    ) -> Result<Samples> {
        let mut samples = Vec::<i32>::new();
        let mut last_reported_percent: Option<usize> = None;
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Write {
                    bytes,
                    total,
                    mut content,
                } => {
                    tracing::trace!("Written {} bytes out of {}", bytes, total);
                    pb.set_position(bytes as u64);
                    let percent = if total == 0 {
                        0
                    } else {
                        bytes.saturating_mul(100) / total
                    };
                    if last_reported_percent != Some(percent) {
                        emit_machine_event(
                            machine_readable,
                            "ITEM_PROGRESS",
                            &[
                                metadata.to_string(),
                                percent.to_string(),
                                bytes.to_string(),
                                total.to_string(),
                            ],
                        );
                        last_reported_percent = Some(percent);
                    }
                    samples.append(&mut content);
                }
                StreamEvent::Finished => {
                    tracing::info!("Finished downloading track");
                    break;
                }
                StreamEvent::Error(stream_error) => {
                    tracing::error!("Error while streaming track: {:?}", stream_error);
                    return Err(stream_error.into());
                }
            }
        }
        Ok(Samples {
            samples,
            ..Default::default()
        })
    }

    fn fail_with_error<S>(&self, pb: &ProgressBar, name: &str, e: S, machine_readable: bool)
    where
        S: Into<String>,
    {
        let error = e.into();
        tracing::error!("Failed to download {}: {}", name, error);
        emit_machine_event(machine_readable, "ITEM_ERROR", &[name.to_string(), error]);
        pb.finish_with_message(
            console::style(format!("Failed! {}", name))
                .red()
                .to_string(),
        );
    }

    async fn was_downloaded(&self, track: &Track) -> bool {
        let Some(history) = &self.history else {
            return false;
        };
        let Some(playlist) = &track.playlist_uri else {
            return false;
        };
        history.lock().await.contains(playlist, &track.uri)
    }

    async fn mark_downloaded(&self, track: &Track, enabled: bool) {
        if !enabled {
            return;
        }
        let Some(history) = &self.history else { return };
        let Some(playlist) = &track.playlist_uri else {
            return;
        };
        if let Err(error) = history.lock().await.record(playlist, &track.uri) {
            tracing::warn!(%error, "Failed to update playlist sync history");
        }
    }
}

fn emit_machine_event(enabled: bool, kind: &str, fields: &[String]) {
    if !enabled {
        return;
    }
    println!("{}", format_machine_event(kind, fields));
    let _ = std::io::stdout().flush();
}

fn format_machine_event(kind: &str, fields: &[String]) -> String {
    let mut event = sanitize_machine_field(kind);
    for field in fields {
        event.push('\t');
        event.push_str(&sanitize_machine_field(field));
    }
    event
}

fn sanitize_machine_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn output_path(destination: &std::path::Path, name: &str, extension: &str) -> PathBuf {
    destination.join(format!("{name}.{extension}"))
}

fn apply_playlist_position(tags: &mut crate::encoder::tags::Tags, track: &Track, enabled: bool) {
    if enabled {
        tags.track = track.playlist_position;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_TRACK_RETRIES, RETRY_PAUSE, apply_playlist_position, format_machine_event, output_path,
    };
    use crate::encoder::tags::Tags;
    use crate::track::Track;
    use librespot::core::SpotifyUri;

    #[test]
    fn machine_events_are_single_line_and_tab_delimited() {
        let event = format_machine_event(
            "ITEM_PROGRESS",
            &["Artist\tTitle\nLive".to_string(), "42".to_string()],
        );
        assert_eq!(event, "ITEM_PROGRESS\tArtist Title Live\t42");
    }

    #[test]
    fn dotted_names_keep_the_full_title_before_the_extension() {
        let destination = std::path::Path::new("music");
        assert_eq!(
            output_path(destination, "V.I.C. - Get Silly", "mp3"),
            destination.join("V.I.C. - Get Silly.mp3")
        );
        assert_eq!(
            output_path(destination, "V.I.C. - Wobble", "mp3"),
            destination.join("V.I.C. - Wobble.mp3")
        );
    }

    #[test]
    fn retry_policy_uses_fresh_attempts_after_five_minutes() {
        assert_eq!(MAX_TRACK_RETRIES, 3);
        assert_eq!(RETRY_PAUSE.as_secs(), 300);
    }

    #[test]
    fn playlist_position_is_applied_only_when_enabled() {
        let mut tags = Tags {
            title: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_title: "Album".to_string(),
            album_cover: None,
            track: None,
        };
        let track = Track {
            uri: SpotifyUri::from_uri("spotify:track:6oUGAx0vkBcnGzYkvw0ZsA").unwrap(),
            playlist_position: Some((1, 91)),
            playlist_uri: None,
        };
        apply_playlist_position(&mut tags, &track, false);
        assert_eq!(tags.track, None);
        apply_playlist_position(&mut tags, &track, true);
        assert_eq!(tags.track, Some((1, 91)));
    }
}
