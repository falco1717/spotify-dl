# Spotify DL GUI

A native Windows frontend for this fork of `spotify-dl`. It provides browser-based Spotify login, URL and destination fields, format selection, cancellation, and per-item download progress without requiring command-line use.

If a track load fails, the downloader pauses in place for five minutes before each retry and reports the wait in the activity feed.

The status line counts each retry wait down once per second. While a download is active, the Download button becomes a red Cancel button and returns to Download when the process exits.

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
