using System.Diagnostics;
using System.Drawing.Drawing2D;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace SpotifyDlGui;

public sealed class MainForm : Form
{
    private static readonly Color Canvas = Color.FromArgb(11, 13, 16);
    private static readonly Color Surface = Color.FromArgb(21, 24, 28);
    private static readonly Color Input = Color.FromArgb(30, 34, 39);
    private static readonly Color Primary = Color.FromArgb(42, 220, 113);
    private static readonly Color TextPrimary = Color.FromArgb(242, 246, 243);
    private static readonly Color TextMuted = Color.FromArgb(140, 150, 145);
    private static readonly Regex SpotifyInput = new(@"^(https://open\.spotify\.com/(track|album|playlist|episode|show)/[A-Za-z0-9]+(?:\?.*)?|spotify:(track|album|playlist|episode|show):[A-Za-z0-9]+)$", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private readonly TextBox urlBox = new();
    private readonly TextBox destinationBox = new();
    private readonly ComboBox formatBox = new();
    private readonly Button browseButton = new();
    private readonly Button downloadButton = new();
    private readonly Button accountButton = new();
    private readonly RichTextBox outputBox = new();
    private readonly Label statusLabel = new();
    private readonly Label currentItemLabel = new();
    private readonly Label accountStatusLabel = new();
    private readonly ProgressBar itemProgress = new();
    private readonly System.Windows.Forms.Timer countdownTimer = new() { Interval = 1000 };
    private Process? activeProcess;
    private bool isLoggedIn;
    private bool isDownloadRunning;
    private bool cancellationRequested;
    private int countdownSeconds;
    private string countdownPrefix = "Waiting";

    public MainForm()
    {
        Text = "Spotify DL";
        StartPosition = FormStartPosition.CenterScreen;
        MinimumSize = new Size(800, 640);
        Size = new Size(940, 760);
        Font = new Font("Segoe UI", 10F);
        BackColor = Canvas;
        ForeColor = TextPrimary;
        AllowDrop = true;
        DoubleBuffered = true;
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        BuildLayout();
        countdownTimer.Tick += (_, _) => { if (countdownSeconds > 0) countdownSeconds--; UpdateCountdown(); };
        LoadSettings();
        Shown += async (_, _) => { await RefreshAuthStatusAsync(); urlBox.Focus(); };
        DragEnter += HandleDragEnter;
        DragDrop += HandleDragDrop;
        FormClosing += (_, _) => { SaveSettings(); if (activeProcess is { HasExited: false }) activeProcess.Kill(true); };
    }

    private void BuildLayout()
    {
        var root = new TableLayoutPanel { Dock = DockStyle.Fill, Padding = new Padding(34, 26, 34, 22), ColumnCount = 1, RowCount = 5 };
        root.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        root.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        root.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        root.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        root.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        root.Controls.Add(BuildHeader());
        root.Controls.Add(BuildInputCard());
        root.Controls.Add(BuildActions());
        root.Controls.Add(BuildActivityCard());
        root.Controls.Add(BuildFooter());
        Controls.Add(root);
        AcceptButton = downloadButton;
    }

    private Control BuildHeader()
    {
        var header = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 2, Margin = new Padding(0, 0, 0, 22) };
        header.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        header.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        var logo = new PictureBox { Size = new Size(70, 70), SizeMode = PictureBoxSizeMode.Zoom, Margin = new Padding(0, 0, 18, 0) };
        var logoPath = Path.Combine(AppContext.BaseDirectory, "assets", "spotify-dl-logo.png");
        if (File.Exists(logoPath)) logo.Image = Image.FromFile(logoPath);
        var copy = new TableLayoutPanel { AutoSize = true, Dock = DockStyle.Fill, RowCount = 2 };
        copy.Controls.Add(new Label { Text = "Spotify DL", AutoSize = true, Font = new Font("Segoe UI Variable Display Semibold", 28F, FontStyle.Bold), ForeColor = TextPrimary, Margin = new Padding(0, 0, 0, 1) });
        copy.Controls.Add(new Label { Text = "Your music, saved beautifully.", AutoSize = true, Font = new Font("Segoe UI", 11F), ForeColor = TextMuted });
        header.Controls.Add(logo, 0, 0);
        header.Controls.Add(copy, 1, 0);
        return header;
    }

    private Control BuildInputCard()
    {
        var card = CreateCard();
        card.RowCount = 4;
        card.Controls.Add(CreateCaption("SPOTIFY URL"));
        StyleTextBox(urlBox);
        urlBox.PlaceholderText = "Paste or drop a track, album, or playlist URL";
        urlBox.Margin = new Padding(0, 8, 0, 18);
        card.Controls.Add(urlBox);
        card.Controls.Add(CreateCaption("SAVE LOCATION"));
        var row = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 2, Margin = new Padding(0, 8, 0, 0) };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        StyleTextBox(destinationBox);
        StyleButton(browseButton, false);
        browseButton.Text = "Browse";
        browseButton.Margin = new Padding(12, 0, 0, 0);
        browseButton.Click += BrowseForFolder;
        row.Controls.Add(destinationBox, 0, 0);
        row.Controls.Add(browseButton, 1, 0);
        card.Controls.Add(row);
        return card;
    }

    private Control BuildActions()
    {
        var row = new FlowLayoutPanel { Dock = DockStyle.Top, AutoSize = true, FlowDirection = FlowDirection.RightToLeft, Padding = new Padding(0, 16, 0, 16), Margin = new Padding(0) };
        StyleButton(downloadButton, true);
        downloadButton.Text = "Download";
        downloadButton.Click += async (_, _) => { if (isDownloadRunning) CancelDownload(); else await DownloadAsync(); };
        formatBox.DropDownStyle = ComboBoxStyle.DropDownList;
        formatBox.Items.AddRange(["flac", "mp3"]);
        formatBox.SelectedIndex = 0;
        formatBox.Width = 98;
        formatBox.BackColor = Input;
        formatBox.ForeColor = TextPrimary;
        formatBox.FlatStyle = FlatStyle.Flat;
        formatBox.Font = new Font("Segoe UI Semibold", 10F);
        formatBox.Margin = new Padding(0, 4, 12, 0);
        row.Controls.Add(downloadButton);
        row.Controls.Add(formatBox);
        return row;
    }

    private Control BuildActivityCard()
    {
        var card = CreateCard();
        card.Dock = DockStyle.Fill;
        card.RowCount = 5;
        card.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        card.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        card.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        card.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        card.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        card.Controls.Add(CreateCaption("DOWNLOAD ACTIVITY"));
        currentItemLabel.Text = "Nothing downloading yet";
        currentItemLabel.AutoEllipsis = true;
        currentItemLabel.Dock = DockStyle.Top;
        currentItemLabel.Font = new Font("Segoe UI Semibold", 10.5F);
        currentItemLabel.ForeColor = Color.FromArgb(220, 227, 223);
        currentItemLabel.Margin = new Padding(0, 12, 0, 8);
        card.Controls.Add(currentItemLabel);
        itemProgress.Dock = DockStyle.Top;
        itemProgress.Height = 7;
        itemProgress.Style = ProgressBarStyle.Continuous;
        itemProgress.Margin = new Padding(0, 0, 0, 12);
        card.Controls.Add(itemProgress);
        statusLabel.Text = "Ready";
        statusLabel.AutoSize = true;
        statusLabel.ForeColor = Primary;
        statusLabel.Margin = new Padding(0, 0, 0, 10);
        card.Controls.Add(statusLabel);
        outputBox.ReadOnly = true;
        outputBox.BorderStyle = BorderStyle.None;
        outputBox.ScrollBars = RichTextBoxScrollBars.Vertical;
        outputBox.Dock = DockStyle.Fill;
        outputBox.BackColor = Color.FromArgb(15, 18, 21);
        outputBox.ForeColor = Color.FromArgb(177, 188, 182);
        outputBox.Font = new Font("Cascadia Mono", 9.25F);
        outputBox.DetectUrls = false;
        card.Controls.Add(outputBox);
        return card;
    }

    private Control BuildFooter()
    {
        var footer = new TableLayoutPanel { Dock = DockStyle.Bottom, AutoSize = true, ColumnCount = 3, Padding = new Padding(0, 18, 0, 0) };
        footer.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        footer.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        footer.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        footer.Controls.Add(new Label { Text = "Secure browser authentication", AutoSize = true, ForeColor = Color.FromArgb(87, 96, 91), Margin = new Padding(0, 10, 0, 0) }, 0, 0);
        accountStatusLabel.Text = "Checking Spotify login…";
        accountStatusLabel.AutoSize = true;
        accountStatusLabel.ForeColor = TextMuted;
        accountStatusLabel.Margin = new Padding(0, 10, 14, 0);
        footer.Controls.Add(accountStatusLabel, 1, 0);
        StyleButton(accountButton, false);
        accountButton.Text = "Log in";
        accountButton.Click += async (_, _) => await ToggleAccountAsync();
        footer.Controls.Add(accountButton, 2, 0);
        return footer;
    }

    private static RoundedPanel CreateCard() => new() { Dock = DockStyle.Top, AutoSize = true, BackColor = Surface, BorderColor = Color.FromArgb(39, 44, 49), CornerRadius = 14, Padding = new Padding(22), Margin = new Padding(0), ColumnCount = 1 };
    private static Label CreateCaption(string text) => new() { Text = text, AutoSize = true, Font = new Font("Segoe UI Semibold", 8.5F, FontStyle.Bold), ForeColor = Color.FromArgb(126, 136, 131) };
    private static void StyleTextBox(TextBox box) { box.Dock = DockStyle.Fill; box.BorderStyle = BorderStyle.FixedSingle; box.BackColor = Input; box.ForeColor = TextPrimary; box.Font = new Font("Segoe UI", 10.5F); }
    private static void StyleButton(Button button, bool primary)
    {
        button.AutoSize = true;
        button.MinimumSize = new Size(106, 38);
        button.Padding = new Padding(12, 3, 12, 3);
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = primary ? 0 : 1;
        button.FlatAppearance.BorderColor = Color.FromArgb(62, 70, 66);
        button.BackColor = primary ? Primary : Input;
        button.ForeColor = primary ? Color.FromArgb(7, 24, 13) : Color.FromArgb(228, 234, 230);
        button.Font = new Font("Segoe UI Semibold", 9.5F, FontStyle.Bold);
        button.Cursor = Cursors.Hand;
    }

    private void BrowseForFolder(object? sender, EventArgs e)
    {
        using var dialog = new FolderBrowserDialog { Description = "Choose where Spotify DL should save files", UseDescriptionForTitle = true, SelectedPath = Directory.Exists(destinationBox.Text) ? destinationBox.Text : "" };
        if (dialog.ShowDialog(this) == DialogResult.OK) destinationBox.Text = dialog.SelectedPath;
    }

    private async Task DownloadAsync()
    {
        var url = urlBox.Text.Trim();
        var destination = destinationBox.Text.Trim();
        if (!SpotifyInput.IsMatch(url)) { ShowValidation("Enter a valid Spotify track, album, playlist, episode, or show URL/URI.", urlBox); return; }
        if (string.IsNullOrWhiteSpace(destination)) { ShowValidation("Choose a destination folder.", destinationBox); return; }
        try { Directory.CreateDirectory(destination); } catch (Exception ex) { ShowValidation($"The destination folder could not be created:\n{ex.Message}", destinationBox); return; }
        var executable = FindSpotifyDl();
        if (executable is null) { ShowMissingCli(); return; }
        SaveSettings();
        cancellationRequested = false;
        SetRunning(true, true);
        outputBox.Clear();
        currentItemLabel.Text = "Preparing download…";
        itemProgress.Value = 0;
        statusLabel.Text = "Downloading…";
        var startInfo = NewCliStartInfo(executable);
        foreach (var argument in new[] { "--destination", destination, "--format", formatBox.SelectedItem?.ToString() ?? "flac", "--machine-readable", url }) startInfo.ArgumentList.Add(argument);
        try
        {
            activeProcess = new Process { StartInfo = startInfo, EnableRaisingEvents = true };
            activeProcess.OutputDataReceived += (_, e) => HandleDownloadOutput(e.Data);
            activeProcess.ErrorDataReceived += (_, e) => HandleDownloadOutput(e.Data);
            activeProcess.Start();
            activeProcess.BeginOutputReadLine();
            activeProcess.BeginErrorReadLine();
            await activeProcess.WaitForExitAsync();
            StopCountdown();
            statusLabel.Text = cancellationRequested ? "Cancelled" : activeProcess.ExitCode == 0 ? "Download complete" : $"Download failed (exit code {activeProcess.ExitCode})";
            if (!cancellationRequested && activeProcess.ExitCode == 0) itemProgress.Value = 100;
        }
        catch (Exception ex) { AppendOutput(ex.Message, Color.FromArgb(244, 112, 112)); statusLabel.Text = "Download failed"; }
        finally { StopCountdown(); activeProcess?.Dispose(); activeProcess = null; SetRunning(false); cancellationRequested = false; }
    }

    private void HandleDownloadOutput(string? line)
    {
        if (string.IsNullOrWhiteSpace(line)) return;
        if (InvokeRequired) { BeginInvoke(() => HandleDownloadOutput(line)); return; }
        var fields = line.Split('\t');
        switch (fields[0])
        {
            case "QUEUE" when fields.Length >= 2: AppendOutput($"QUEUED  {fields[1]} item(s)", TextMuted); break;
            case "ITEM_START" when fields.Length >= 2: StopCountdown(); currentItemLabel.Text = fields[1]; itemProgress.Value = 0; statusLabel.Text = "Downloading"; AppendOutput($"START   {fields[1]}", Color.FromArgb(126, 192, 255)); break;
            case "ITEM_PROGRESS" when fields.Length >= 3 && int.TryParse(fields[2], out var percent): StopCountdown(); itemProgress.Value = Math.Clamp(percent, 0, 100); if (percent == 100 || percent % 5 == 0) AppendOutput($"{percent,3}%    {fields[1]}", Color.FromArgb(164, 176, 169)); break;
            case "ITEM_STAGE" when fields.Length >= 3: AppendOutput($"{fields[2].ToUpperInvariant(),-7} {fields[1]}", Color.FromArgb(224, 190, 113)); break;
            case "ITEM_RETRY" when fields.Length >= 5 && int.TryParse(fields[4], out var retryDelay): currentItemLabel.Text = fields[1]; itemProgress.Value = 0; StartCountdown(retryDelay, "Retrying in"); AppendOutput($"RETRY   {fields[2]}/{fields[3]} — waiting {FormatDuration(retryDelay)} — {fields[1]}", Color.FromArgb(244, 180, 88)); break;
            case "ITEM_RETRY_RESUME" when fields.Length >= 3: StopCountdown(); currentItemLabel.Text = fields[1]; statusLabel.Text = "Reconnecting and retrying…"; AppendOutput($"RESUME  Retry {fields[2]} — new Spotify connection — {fields[1]}", Primary); break;
            case "ITEM_SKIP" when fields.Length >= 2: AppendOutput($"SKIP    {fields[1]}", TextMuted); break;
            case "ITEM_DONE" when fields.Length >= 2: itemProgress.Value = 100; AppendOutput($"DONE    {fields[1]}", Primary); break;
            case "ITEM_ERROR" when fields.Length >= 3: AppendOutput($"FAILED  {fields[1]} — {fields[2]}", Color.FromArgb(244, 112, 112)); break;
            default: AppendOutput(line, TextMuted); break;
        }
    }

    private async Task RefreshAuthStatusAsync() { var result = await RunAuthCommandAsync("--auth-status", "Checking login…", false); SetAuthState(result?.Contains("AUTH_STATUS=logged_in", StringComparison.Ordinal) == true); }
    private async Task ToggleAccountAsync() { if (isLoggedIn) await LogoutAsync(); else await LoginAsync(); }
    private async Task LoginAsync() { outputBox.Clear(); var result = await RunAuthCommandAsync("--login", "Complete login in your browser…", true); SetAuthState(result?.Contains("AUTH_STATUS=logged_in", StringComparison.Ordinal) == true); }
    private async Task LogoutAsync()
    {
        if (MessageBox.Show(this, "Log out of Spotify on this computer? Downloaded files and settings will remain.", "Log out of Spotify", MessageBoxButtons.YesNo, MessageBoxIcon.Question) != DialogResult.Yes) return;
        outputBox.Clear();
        var result = await RunAuthCommandAsync("--logout", "Logging out…", true);
        SetAuthState(result?.Contains("AUTH_STATUS=logged_out", StringComparison.Ordinal) != true);
    }

    private async Task<string?> RunAuthCommandAsync(string argument, string busyStatus, bool showOutput)
    {
        var executable = FindSpotifyDl();
        if (executable is null) { ShowMissingCli(); return null; }
        SetRunning(true);
        accountStatusLabel.Text = busyStatus;
        try
        {
            activeProcess = new Process { StartInfo = NewCliStartInfo(executable) };
            activeProcess.StartInfo.ArgumentList.Add(argument);
            activeProcess.Start();
            var stdout = activeProcess.StandardOutput.ReadToEndAsync();
            var stderr = activeProcess.StandardError.ReadToEndAsync();
            await activeProcess.WaitForExitAsync();
            var output = (await stdout) + (await stderr);
            if (showOutput && !string.IsNullOrWhiteSpace(output)) AppendOutput(output.TrimEnd(), TextMuted);
            return activeProcess.ExitCode == 0 ? output : null;
        }
        catch (Exception ex) { AppendOutput(ex.Message, Color.FromArgb(244, 112, 112)); return null; }
        finally { activeProcess?.Dispose(); activeProcess = null; SetRunning(false); }
    }

    private static ProcessStartInfo NewCliStartInfo(string executable) => new() { FileName = executable, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
    private void SetAuthState(bool loggedIn) { isLoggedIn = loggedIn; accountStatusLabel.Text = loggedIn ? "Signed in" : "Not signed in"; accountStatusLabel.ForeColor = loggedIn ? Primary : TextMuted; accountButton.Text = loggedIn ? "Log out" : "Log in"; accountButton.Enabled = activeProcess is null; }
    private static string? FindSpotifyDl()
    {
        var bundledPath = Path.Combine(AppContext.BaseDirectory, "spotify-dl.exe");
        if (File.Exists(bundledPath)) return bundledPath;
        var cargoPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".cargo", "bin", "spotify-dl.exe");
        if (File.Exists(cargoPath)) return cargoPath;
        foreach (var folder in (Environment.GetEnvironmentVariable("PATH") ?? "").Split(Path.PathSeparator)) { var candidate = Path.Combine(folder.Trim(), "spotify-dl.exe"); if (File.Exists(candidate)) return candidate; }
        return null;
    }
    private void CancelDownload() { if (activeProcess is not { HasExited: false }) return; cancellationRequested = true; StopCountdown(); downloadButton.Enabled = false; downloadButton.Text = "Cancelling…"; activeProcess.Kill(true); statusLabel.Text = "Cancelling…"; AppendOutput("CANCEL  Download stopping", Color.FromArgb(244, 180, 88)); }
    private void SetRunning(bool running, bool cancellable = false) { isDownloadRunning = running && cancellable; downloadButton.Text = isDownloadRunning ? "Cancel" : "Download"; downloadButton.BackColor = isDownloadRunning ? Color.FromArgb(190, 65, 65) : Primary; downloadButton.ForeColor = isDownloadRunning ? Color.White : Color.FromArgb(7, 24, 13); downloadButton.Enabled = !running || cancellable; AcceptButton = running ? null : downloadButton; urlBox.Enabled = !running; destinationBox.Enabled = !running; browseButton.Enabled = !running; formatBox.Enabled = !running; accountButton.Enabled = !running; }
    private void StartCountdown(int seconds, string prefix) { countdownSeconds = Math.Max(0, seconds); countdownPrefix = prefix; UpdateCountdown(); countdownTimer.Start(); }
    private static string FormatDuration(int seconds) => seconds % 60 == 0 ? $"{seconds / 60} minute{(seconds == 60 ? "" : "s")}" : $"{seconds} seconds";
    private void UpdateCountdown() { var minutes = countdownSeconds / 60; var seconds = countdownSeconds % 60; statusLabel.Text = countdownSeconds > 0 ? $"{countdownPrefix} {minutes}:{seconds:00}" : "Resuming…"; if (countdownSeconds <= 0) countdownTimer.Stop(); }
    private void StopCountdown() { countdownTimer.Stop(); countdownSeconds = 0; }
    private void AppendOutput(string? text, Color color) { if (string.IsNullOrEmpty(text)) return; if (InvokeRequired) { BeginInvoke(() => AppendOutput(text, color)); return; } outputBox.SelectionStart = outputBox.TextLength; outputBox.SelectionColor = color; outputBox.AppendText(text + Environment.NewLine); outputBox.SelectionColor = outputBox.ForeColor; outputBox.ScrollToCaret(); }
    private void ShowValidation(string message, Control focus) { MessageBox.Show(this, message, "Check your entry", MessageBoxButtons.OK, MessageBoxIcon.Warning); focus.Focus(); }
    private void ShowMissingCli() => MessageBox.Show(this, "spotify-dl.exe was not found. Reinstall it or add it to PATH.", "Spotify DL not found", MessageBoxButtons.OK, MessageBoxIcon.Error);
    private void HandleDragEnter(object? sender, DragEventArgs e) { if (e.Data?.GetDataPresent(DataFormats.Text) == true) e.Effect = DragDropEffects.Copy; }
    private void HandleDragDrop(object? sender, DragEventArgs e) { if (e.Data?.GetData(DataFormats.Text) is string text) urlBox.Text = text.Trim(); }
    private static string SettingsPath => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "SpotifyDlGui", "settings.json");
    private void LoadSettings() { try { if (!File.Exists(SettingsPath)) { destinationBox.Text = Environment.GetFolderPath(Environment.SpecialFolder.MyMusic); return; } var settings = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(SettingsPath)); destinationBox.Text = settings?.Destination ?? Environment.GetFolderPath(Environment.SpecialFolder.MyMusic); if (settings?.Format is string format && formatBox.Items.Contains(format)) formatBox.SelectedItem = format; } catch { destinationBox.Text = Environment.GetFolderPath(Environment.SpecialFolder.MyMusic); } }
    private void SaveSettings() { try { Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!); File.WriteAllText(SettingsPath, JsonSerializer.Serialize(new AppSettings(destinationBox.Text.Trim(), formatBox.SelectedItem?.ToString() ?? "flac"))); } catch { } }
    private sealed record AppSettings(string Destination, string Format);
}

internal sealed class RoundedPanel : TableLayoutPanel
{
    public int CornerRadius { get; set; } = 14;
    public Color BorderColor { get; set; } = Color.Transparent;
    public RoundedPanel() => SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint, true);
    protected override void OnResize(EventArgs e) { base.OnResize(e); if (Width <= 1 || Height <= 1) return; using var path = RoundedRectangle(ClientRectangle, CornerRadius); Region = new Region(path); }
    protected override void OnPaint(PaintEventArgs e) { e.Graphics.SmoothingMode = SmoothingMode.AntiAlias; using var path = RoundedRectangle(new Rectangle(0, 0, Width - 1, Height - 1), CornerRadius); using var brush = new SolidBrush(BackColor); using var pen = new Pen(BorderColor); e.Graphics.FillPath(brush, path); e.Graphics.DrawPath(pen, path); base.OnPaint(e); }
    private static GraphicsPath RoundedRectangle(Rectangle bounds, int radius) { var d = radius * 2; var path = new GraphicsPath(); path.AddArc(bounds.Left, bounds.Top, d, d, 180, 90); path.AddArc(bounds.Right - d, bounds.Top, d, d, 270, 90); path.AddArc(bounds.Right - d, bounds.Bottom - d, d, d, 0, 90); path.AddArc(bounds.Left, bounds.Bottom - d, d, d, 90, 90); path.CloseFigure(); return path; }
}
