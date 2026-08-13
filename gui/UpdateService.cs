using System.Diagnostics;
using System.Net.Http.Headers;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json;

namespace SpotifyDlGui;

internal sealed record UpdateRelease(Version Version, string Tag, string Notes, string InstallerUrl, string ChecksumUrl);

internal static class UpdateService
{
    private const string LatestReleaseApi = "https://api.github.com/repos/falco1717/spotify-dl/releases/latest";
    private const string InstallerName = "Spotify-DL-Setup.exe";
    private const string ChecksumName = "Spotify-DL-Setup.exe.sha256";
    private static readonly HttpClient Client = CreateClient();

    public static Version CurrentVersion => Assembly.GetExecutingAssembly().GetName().Version ?? new Version(0, 0);

    public static async Task<UpdateRelease?> CheckAsync(CancellationToken cancellationToken = default)
    {
        using var response = await Client.GetAsync(LatestReleaseApi, cancellationToken);
        response.EnsureSuccessStatusCode();
        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);
        var root = document.RootElement;
        var tag = root.GetProperty("tag_name").GetString() ?? "";
        if (!Version.TryParse(tag.TrimStart('v', 'V'), out var version)) return null;
        var notes = root.TryGetProperty("body", out var body) ? body.GetString() ?? "" : "";
        string? installer = null;
        string? checksum = null;
        foreach (var asset in root.GetProperty("assets").EnumerateArray())
        {
            var name = asset.GetProperty("name").GetString();
            var url = asset.GetProperty("browser_download_url").GetString();
            if (name == InstallerName) installer = url;
            if (name == ChecksumName) checksum = url;
        }
        return version > CurrentVersion && installer is not null && checksum is not null
            ? new UpdateRelease(version, tag, notes, installer, checksum)
            : null;
    }

    public static async Task<string> DownloadVerifiedInstallerAsync(UpdateRelease release, IProgress<int>? progress = null, CancellationToken cancellationToken = default)
    {
        var directory = Path.Combine(Path.GetTempPath(), "SpotifyDlUpdate", release.Version.ToString());
        Directory.CreateDirectory(directory);
        var installerPath = Path.Combine(directory, InstallerName);
        var expectedText = await Client.GetStringAsync(release.ChecksumUrl, cancellationToken);
        var expectedHash = expectedText.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault()?.Trim();
        if (expectedHash is null || expectedHash.Length != 64) throw new InvalidDataException("The release checksum file is invalid.");

        using var response = await Client.GetAsync(release.InstallerUrl, HttpCompletionOption.ResponseHeadersRead, cancellationToken);
        response.EnsureSuccessStatusCode();
        var total = response.Content.Headers.ContentLength;
        await using (var source = await response.Content.ReadAsStreamAsync(cancellationToken))
        await using (var destination = new FileStream(installerPath, FileMode.Create, FileAccess.Write, FileShare.None, 81920, true))
        {
            var buffer = new byte[81920];
            long downloaded = 0;
            int read;
            while ((read = await source.ReadAsync(buffer, cancellationToken)) > 0)
            {
                await destination.WriteAsync(buffer.AsMemory(0, read), cancellationToken);
                downloaded += read;
                if (total > 0) progress?.Report((int)(downloaded * 100 / total.Value));
            }
        }

        await using var file = File.OpenRead(installerPath);
        var actualHash = Convert.ToHexString(await SHA256.HashDataAsync(file, cancellationToken));
        if (!actualHash.Equals(expectedHash, StringComparison.OrdinalIgnoreCase))
        {
            File.Delete(installerPath);
            throw new InvalidDataException("The downloaded installer failed SHA-256 verification.");
        }
        return installerPath;
    }

    public static void LaunchInstaller(string path)
    {
        Process.Start(new ProcessStartInfo(path, "/install /passive /norestart") { UseShellExecute = true });
    }

    private static HttpClient CreateClient()
    {
        var client = new HttpClient { Timeout = TimeSpan.FromMinutes(10) };
        client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("Spotify-DL", CurrentVersion.ToString(3)));
        client.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/vnd.github+json"));
        return client;
    }
}
