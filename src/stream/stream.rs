use std::sync::Arc;

use anyhow::Result;
use librespot::core::Session;
use librespot::playback::config::{Bitrate, PlayerConfig};
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::player::{Player, PlayerEvent};
use tokio::sync::mpsc::UnboundedSender;

use crate::stream::channel_sink::{ChannelSink, SinkEvent};
use crate::stream::{StreamError, StreamEvent, StreamEventChannel};
use crate::track::Track;

pub struct Stream {
    player_config: PlayerConfig,
    session: Session,
}

impl Stream {
    pub fn new(session: Session) -> Self {
        let config = PlayerConfig {
            bitrate: Bitrate::Bitrate320,
            ..Default::default()
        };
        Stream {
            player_config: config,
            session,
        }
    }

    pub async fn stream(&self, track: Arc<Track>) -> Result<StreamEventChannel> {
        let metadata = track.metadata(&self.session).await?;
        let (sink, mut channel) = ChannelSink::new(metadata);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let player = Player::new(
            self.player_config.clone(),
            self.session.clone(),
            Box::new(NoOpVolume),
            move || Box::new(sink),
        );

        tokio::spawn(async move {
            match Self::load(player.clone(), &track).await {
                Ok(_) => tracing::info!("Track loaded successfully: {:?}", track.uri),
                Err(e) => {
                    tracing::error!("Failed to load track: {:?}, error: {:?}", track.uri, e);
                    Self::send_event(&tx, StreamEvent::Error(e)).await;
                    return;
                }
            }

            tracing::info!("Streaming track: {:?}", track.uri);

            while let Some(event) = channel.recv().await {
                match event {
                    SinkEvent::Write {
                        bytes,
                        total,
                        content,
                    } => {
                        Self::send_event(
                            &tx,
                            StreamEvent::Write {
                                bytes,
                                total,
                                content,
                            },
                        )
                        .await
                    }
                    SinkEvent::Finished => {
                        Self::send_event(&tx, StreamEvent::Finished).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn load(player: Arc<Player>, track: &Track) -> Result<(), StreamError> {
        player.load(track.uri.clone(), true, 0);

        tracing::info!("Loading track: {:?}", track.uri);
        loop {
            match player.get_player_event_channel().recv().await {
                Some(PlayerEvent::Playing { .. })
                | Some(PlayerEvent::TrackChanged { .. })
                | Some(PlayerEvent::EndOfTrack { .. }) => {
                    tracing::info!("Player started playing track: {:?}", track.uri);
                    break;
                }
                Some(PlayerEvent::Unavailable { .. }) => {
                    tracing::info!("Track is unavailable: {:?}", track.uri);
                    return Err(StreamError::Unavailable);
                }
                _ => {
                    // Ignore other events
                }
            }
        }

        tokio::spawn(async move {
            player.await_end_of_track().await;
            player.stop();
        });

        Ok(())
    }

    async fn send_event(tx: &UnboundedSender<StreamEvent>, event: StreamEvent) {
        tx.send(event).unwrap_or_else(|e| {
            tracing::error!("Failed to send event: {:?}", e);
        });
    }
}
