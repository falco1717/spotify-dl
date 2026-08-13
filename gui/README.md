# Spotify DL GUI

A native Windows frontend for this fork of `spotify-dl`. It provides browser-based Spotify login, URL and destination fields, format selection, cancellation, and per-item download progress without requiring command-line use.

Large playlists and albums download in groups of 28 tracks with a three-minute cooldown between groups. The activity feed reports each pause and resume.

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
