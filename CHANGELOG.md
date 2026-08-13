# Changelog
All notable changes to this project will be documented in this file.

## [0.3.10-gui] - 2026-08-13

- Add automatic daily and manual GitHub Release update checks.
- Show release notes and installer download progress in the application.
- Verify downloaded installers with a published SHA-256 checksum before launch.
- Allow users to skip a release and upgrade in place without clearing settings or Spotify credentials.

## [0.3.9-gui] - 2026-08-13

- Add opt-in per-folder playlist sync history and a reset-history action.
- Remember the last Spotify URL in the GUI.
- Add Normal, Turbo (5), and custom parallel speed controls.
- Skip unavailable tracks without stopping the remaining playlist.

## [0.3.8-gui] - 2026-08-13

- Add optional `--playlist-track-numbers` tagging that writes each playlist item's 1-based position and playlist length to standard MP3/FLAC track fields.
- Add a remembered GUI checkbox for playlist-order track numbering.

## [0.3.7-gui] - 2026-08-13

- Reconnect the Spotify session and construct a fresh librespot player after every five-minute retry countdown instead of reusing a failed player instance.
- Report the actual retry resumption in the GUI activity feed.

## [0.3.6-gui] - 2026-08-13

- Preserve dotted artist/title names when adding the audio extension, preventing distinct tracks such as `V.I.C. - Get Silly` and `V.I.C. - Wobble` from overwriting one another.

## [0.3.5-gui] - 2026-08-13

- Increase the pause before every track-load retry from three minutes to five minutes.
- Derive the GUI activity message and live countdown from the CLI-provided retry duration.

## [0.3.4-gui] - 2026-08-13

- Remove scheduled 28-track playlist cooldowns so queues continue without arbitrary pauses.
- Retain the three-minute countdown only for actual track-load retries.

## [0.3.3-gui] - 2026-08-13

- Show live `M:SS` countdowns during retry and playlist cooldown pauses.
- Replace the separate Download and Cancel controls with one state-aware action button.

## [0.3.2-gui] - 2026-08-13

- Replace short exponential track-load retry delays with a fixed three-minute pause before every retry.
- Show the retry cooldown explicitly in the Windows GUI activity feed.

## [0.3.1-gui] - 2026-08-13

- Download large queues in batches of 28 tracks with a three-minute cooldown between batches.
- Report batch pauses and resumes in the Windows GUI activity feed.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.9.1] - 2025-07-11
### :sparkles: New Features
- [`1ac214f`](https://github.com/GuillemCastro/spotify-dl/commit/1ac214f896e9422745c165a53ade152719803978) - **encoding**: Use 32 bit audio *(PR [#32](https://github.com/GuillemCastro/spotify-dl/pull/32) by [@GuillemCastro](https://github.com/GuillemCastro))*

### :bug: Bug Fixes
- [`9f59dcc`](https://github.com/GuillemCastro/spotify-dl/commit/9f59dcc7951c75cda456c5d99957bb9841c1bc55) - **flac**: flacenc supports max 24-bit audio *(PR [#34](https://github.com/GuillemCastro/spotify-dl/pull/34) by [@GuillemCastro](https://github.com/GuillemCastro))*

### :wrench: Chores
- [`2947214`](https://github.com/GuillemCastro/spotify-dl/commit/29472147655f7f6780949711aaaef055481ec2e8) - **deps**: Optimize dependencies & upgrade to 2024 edition *(PR [#33](https://github.com/GuillemCastro/spotify-dl/pull/33) by [@GuillemCastro](https://github.com/GuillemCastro))*


## [v0.9.0] - 2025-07-07
### :sparkles: New Features
- [`607bf97`](https://github.com/GuillemCastro/spotify-dl/commit/607bf976283305354e2c8a99e957c9cb46b06ccf) - **downloader**: Skip existing files *(PR [#30](https://github.com/GuillemCastro/spotify-dl/pull/30) by [@GuillemCastro](https://github.com/GuillemCastro))*


## [v0.8.0] - 2025-07-06
### :sparkles: New Features
- [`14d3e1f`](https://github.com/GuillemCastro/spotify-dl/commit/14d3e1fda34f9cc587fc64d1442cb3e5deac440a) - **downloader**: Increase bitrate to 320kbps *(PR [#28](https://github.com/GuillemCastro/spotify-dl/pull/28) by [@GuillemCastro](https://github.com/GuillemCastro))*

### :wrench: Chores
- [`09fc850`](https://github.com/GuillemCastro/spotify-dl/commit/09fc850bcbbed6331c1c900c72cd629f0d34ea55) - **deps**: Upgrade dependencies *(PR [#29](https://github.com/GuillemCastro/spotify-dl/pull/29) by [@GuillemCastro](https://github.com/GuillemCastro))*


## [v0.7.0] - 2025-06-22
### :wrench: Chores
- [`dcedaa0`](https://github.com/GuillemCastro/spotify-dl/commit/dcedaa0d7e2eb487e29e1282ddbf05402fdb69da) - Update dependencies and fix auth *(PR [#27](https://github.com/GuillemCastro/spotify-dl/pull/27) by [@GuillemCastro](https://github.com/GuillemCastro))*


## [v0.5.0] - 2024-05-17
### :sparkles: New Features
- [`ad9288d`](https://github.com/GuillemCastro/spotify-dl/commit/ad9288d243c393ea6c5b283de9c8ccd53de8ee0c) - No native dependencies *(commit by [@GuillemCastro](https://github.com/GuillemCastro))*

[v0.5.0]: https://github.com/GuillemCastro/spotify-dl/compare/v0.2.1...v0.5.0
[v0.7.0]: https://github.com/GuillemCastro/spotify-dl/compare/v0.5.4...v0.7.0
[v0.8.0]: https://github.com/GuillemCastro/spotify-dl/compare/v0.7.1...v0.8.0
[v0.9.0]: https://github.com/GuillemCastro/spotify-dl/compare/v0.8.0...v0.9.0
[v0.9.1]: https://github.com/GuillemCastro/spotify-dl/compare/v0.9.0...v0.9.1
