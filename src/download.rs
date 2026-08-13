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

use crate::encoder;
use crate::encoder::Format;
use crate::encoder::Samples;
use crate::stream::Stream;
use crate::stream::StreamEvent;
use crate::stream::StreamEventChannel;
use crate::track::Track;
use crate::track::TrackMetadata;

pub struct Downloader {
    session: Session,
    progress_bar: MultiProgress,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub destination: PathBuf,
    pub parallel: usize,
    pub format: Format,
    pub force: bool,
    pub machine_readable: bool,
}

impl DownloadOptions {
    pub fn new(
        destination: Option<String>,
        parallel: usize,
        format: Format,
        force: bool,
        machine_readable: bool,
    ) -> Self {
        let destination =
            destination.map_or_else(|| std::env::current_dir().unwrap(), PathBuf::from);
        DownloadOptions {
            destination,
            parallel,
            format,
            force,
            machine_readable,
        }
    }
}

impl Downloader {
    pub fn new(session: Session) -> Self {
        Downloader {
            session,
            progress_bar: MultiProgress::new(),
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
        let metadata = track.metadata(&self.session).await?;
        tracing::info!("Downloading track: {:?}", metadata.track_name);
        let display_name = metadata.to_string();

        let path = options
            .destination
            .join(metadata.to_string())
            .with_extension(options.format.extension())
            .to_str()
            .ok_or(anyhow::anyhow!("Could not set the output path"))?
            .to_string();

        if !options.force && PathBuf::from(&path).exists() {
            tracing::info!(
                "Skipping {}, file already exists. Use --force to force re-downloading the track",
                &metadata.track_name
            );
            emit_machine_event(options.machine_readable, "ITEM_SKIP", &[display_name]);
            return Ok(());
        }

        emit_machine_event(
            options.machine_readable,
            "ITEM_START",
            &[display_name.clone(), metadata.approx_size().to_string()],
        );

        let pb = self.add_progress_bar(&metadata);

        let stream = Stream::new(self.session.clone());
        let channel = match stream.stream(Arc::new(track)).await {
            Ok(channel) => channel,
            Err(e) => {
                self.fail_with_error(&pb, &display_name, e.to_string(), options.machine_readable);
                return Ok(());
            }
        };

        let samples = match self
            .buffer_track(channel, &pb, &metadata, options.machine_readable)
            .await
        {
            Ok(samples) => samples,
            Err(e) => {
                self.fail_with_error(&pb, &display_name, e.to_string(), options.machine_readable);
                return Ok(());
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

        let tags = metadata.tags().await?;
        encoder::tags::store_tags(path, &tags, options.format).await?;

        pb.finish_with_message(format!("Downloaded {}", metadata.to_string()));
        emit_machine_event(options.machine_readable, "ITEM_DONE", &[display_name]);
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
                    return Err(anyhow::anyhow!("Streaming error: {:?}", stream_error));
                }
                StreamEvent::Retry {
                    attempt,
                    max_attempts,
                    delay_seconds,
                } => {
                    tracing::warn!(
                        "Retrying download, attempt {} of {}: {}",
                        attempt,
                        max_attempts,
                        metadata.to_string()
                    );
                    pb.set_message(format!(
                        "Waiting 3 minutes before retry ({}/{}) {}",
                        attempt,
                        max_attempts,
                        metadata.to_string()
                    ));
                    emit_machine_event(
                        machine_readable,
                        "ITEM_RETRY",
                        &[
                            metadata.to_string(),
                            attempt.to_string(),
                            max_attempts.to_string(),
                            delay_seconds.to_string(),
                        ],
                    );
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

#[cfg(test)]
mod tests {
    use super::format_machine_event;

    #[test]
    fn machine_events_are_single_line_and_tab_delimited() {
        let event = format_machine_event(
            "ITEM_PROGRESS",
            &["Artist\tTitle\nLive".to_string(), "42".to_string()],
        );
        assert_eq!(event, "ITEM_PROGRESS\tArtist Title Live\t42");
    }
}
