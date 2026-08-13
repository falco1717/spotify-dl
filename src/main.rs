use spotify_dl::download::{DownloadOptions, Downloader};
use spotify_dl::encoder::Format;
use spotify_dl::log;
use spotify_dl::session::{create_session, is_logged_in, login, logout};
use spotify_dl::track::get_tracks;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "spotify-dl",
    about = "A commandline utility to download music directly from Spotify"
)]
struct Opt {
    #[structopt(
        help = "A list of Spotify URIs or URLs (songs, podcasts, playlists or albums)",
        required = false
    )]
    tracks: Vec<String>,
    #[structopt(
        long = "login",
        help = "Sign in through Spotify's secure browser OAuth flow"
    )]
    login: bool,
    #[structopt(
        long = "auth-status",
        help = "Print whether cached Spotify credentials are available"
    )]
    auth_status: bool,
    #[structopt(long = "logout", help = "Remove cached Spotify credentials")]
    logout: bool,
    #[structopt(
        short = "d",
        long = "destination",
        help = "The directory where the songs will be downloaded"
    )]
    destination: Option<String>,
    #[structopt(
        short = "t",
        long = "parallel",
        help = "Number of parallel downloads. Default is 5.",
        default_value = "5"
    )]
    parallel: usize,
    #[structopt(
        short = "f",
        long = "format",
        help = "The format to download the tracks in. Default is flac.",
        default_value = "flac"
    )]
    format: Format,
    #[structopt(
        short = "F",
        long = "force",
        help = "Force download even if the file already exists"
    )]
    force: bool,
    #[structopt(
        long = "machine-readable",
        help = "Emit newline-delimited progress events for graphical clients",
        hidden = true
    )]
    machine_readable: bool,
    #[structopt(
        long = "playlist-track-numbers",
        help = "Write each playlist item's position and playlist length to its track-number tags"
    )]
    playlist_track_numbers: bool,
}

pub fn create_destination_if_required(destination: Option<String>) -> anyhow::Result<()> {
    if let Some(destination) = destination {
        if !std::path::Path::new(&destination).exists() {
            tracing::info!("Creating destination directory: {}", destination);
            std::fs::create_dir_all(destination)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::configure_logger()?;

    let opt = Opt::from_args();

    let auth_actions = [opt.login, opt.auth_status, opt.logout]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if auth_actions > 1 {
        anyhow::bail!("Choose only one of --login, --auth-status, or --logout");
    }
    if auth_actions > 0 && !opt.tracks.is_empty() {
        anyhow::bail!("Authentication commands cannot be combined with track URLs");
    }

    if opt.auth_status {
        println!(
            "AUTH_STATUS={}",
            if is_logged_in()? {
                "logged_in"
            } else {
                "logged_out"
            }
        );
        return Ok(());
    }
    if opt.logout {
        let removed = logout()?;
        println!("AUTH_STATUS=logged_out");
        if !removed {
            println!("No cached Spotify credentials were present.");
        }
        return Ok(());
    }
    if opt.login {
        println!("Opening Spotify login in your browser...");
        let _session = login().await?;
        println!("AUTH_STATUS=logged_in");
        return Ok(());
    }

    create_destination_if_required(opt.destination.clone())?;

    if opt.tracks.is_empty() {
        anyhow::bail!("No tracks provided. Pass a Spotify URL or use an authentication command.");
    }

    let session = create_session().await?;

    let track = get_tracks(opt.tracks, &session).await?;

    let downloader = Downloader::new(session);
    downloader
        .download_tracks(
            track,
            &DownloadOptions::new(
                opt.destination,
                opt.parallel,
                opt.format,
                opt.force,
                opt.machine_readable,
                opt.playlist_track_numbers,
            ),
        )
        .await
}
