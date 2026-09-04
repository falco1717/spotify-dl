# Spotify DL GUI

A native Windows frontend for this fork of `spotify-dl`. It provides browser-based Spotify login, URL and destination fields, format selection, cancellation, and per-item download progress without requiring command-line use.

The **Log in** button opens the generated Spotify OAuth authorization page in the default browser. Keep Spotify DL open until the browser confirms that it is connected; credentials are then cached by the CLI for later sessions.

If a track load fails, the downloader pauses in place for five minutes before each retry and reports the wait in the activity feed.

The status line counts each retry wait down once per second. While a download is active, the Download button becomes a red Cancel button and returns to Download when the process exits.

Enable **Use playlist order as track #** to tag playlist items as `position/playlist length` in MP3 or FLAC metadata. The option is remembered between launches and does not alter standalone track or album numbering.

Enable **Playlist sync** to keep a `.spotify-dl-history.json` file in the selected destination. Later runs skip completed playlist items before fetching their metadata. **Reset history** clears only that file and does not delete music. The last Spotify URL is remembered locally in the GUI settings.

The speed selector offers **Normal** (one track at a time with a short listening-style pause), **Turbo (5)**, and a custom parallel-download count. Unavailable Spotify items appear as skipped and do not stop the playlist.

The footer checks GitHub Releases for updates at most once per day and also provides a manual **Check updates** button. Available releases show their notes, download progress, and a skip option. Before launching an upgrade, the downloaded installer is verified against the release's `Spotify-DL-Setup.exe.sha256` asset. Settings, playlist history, downloads, and cached Spotify credentials remain in place during an upgrade.

## Run from source

```powershell
dotnet run --project gui/SpotifyDlGui.csproj
```

The GUI first looks for `spotify-dl.exe` beside itself, then in `%USERPROFILE%\.cargo\bin`, and finally on `PATH`.

## Build the installer

From the repository root on Windows:

```powershell
powershell -ExecutionPolicy Bypass -File installer/build-installer.ps1
```

The script builds the Rust CLI, publishes the GUI as a self-contained Windows x64 application, and creates `artifacts/installer/Spotify-DL-Setup.exe`. The installer deploys both programs under Program Files, creates a Start menu shortcut, and registers an uninstaller in Windows Installed Apps / Control Panel.

Spotify OAuth credentials are stored in the user's normal `~/.spotify-dl` profile directory at runtime. They are never embedded in the source tree or installer.
