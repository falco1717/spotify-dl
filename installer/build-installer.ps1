#requires -Version 5.1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$artifactRoot = Join-Path $repositoryRoot 'artifacts'
$publishDirectory = Join-Path $artifactRoot 'publish'
$installerDirectory = Join-Path $artifactRoot 'installer'
$guiDirectory = Join-Path $repositoryRoot 'gui'

Push-Location $repositoryRoot
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'The Rust CLI build failed.' }

    dotnet publish (Join-Path $guiDirectory 'SpotifyDlGui.csproj') -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -o $publishDirectory
    if ($LASTEXITCODE -ne 0) { throw 'The GUI publish failed.' }

    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'target\release\spotify-dl.exe') -Destination (Join-Path $publishDirectory 'spotify-dl.exe') -Force

    dotnet build (Join-Path $PSScriptRoot 'Package\Package.wixproj') -c Release -p:PayloadDir=$publishDirectory -p:GuiDir=$guiDirectory -o $installerDirectory
    if ($LASTEXITCODE -ne 0) { throw 'The MSI package build failed.' }

    $msiPath = Join-Path $installerDirectory 'Spotify-DL.msi'
    dotnet build (Join-Path $PSScriptRoot 'Bundle\Bundle.wixproj') -c Release -p:MsiPath=$msiPath -p:GuiDir=$guiDirectory -o $installerDirectory
    if ($LASTEXITCODE -ne 0) { throw 'The installer bundle build failed.' }

    $setupPath = Join-Path $installerDirectory 'Spotify-DL-Setup.exe'
    $checksumPath = "$setupPath.sha256"
    $checksum = (Get-FileHash -LiteralPath $setupPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText($checksumPath, "$checksum  Spotify-DL-Setup.exe`n", [System.Text.Encoding]::ASCII)
    Write-Host "Installer created at $setupPath"
    Write-Host "Checksum created at $checksumPath"
}
finally {
    Pop-Location
}
