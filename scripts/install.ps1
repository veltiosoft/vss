#!/usr/bin/env pwsh
# Copyright 2023 TSURUTA Takumi. All rights reserved. MIT license.
# Adapted from https://github.com/denoland/deno_install
# Copyright 2019 the Deno authors. All rights reserved. MIT license.

$ErrorActionPreference = 'Stop'
if ($Args.Length -gt 1) { throw 'Usage: install.ps1 [VERSION]' }
$Version = if ($Args.Length -eq 1) { $Args[0] } elseif ($v) { "v${v}" } else { $null }
if ($Version -and $Version -notmatch '^[A-Za-z0-9.+-]+$') { throw 'Invalid release version' }

$vssInstall = if ($env:VSS_INSTALL) { $env:VSS_INSTALL } else { Join-Path $Home '.vss' }
$BinDir = Join-Path $vssInstall 'bin'
$vssExe = Join-Path $BinDir 'vss.exe'
$ArchiveName = 'vss_windows_amd64.zip'
$BaseUrl = 'https://github.com/veltiosoft/vss/releases/'
$BaseUrl += if ($Version) { "download/$Version" } else { 'latest/download' }
$DownloadUrl = "$BaseUrl/$ArchiveName"
$Work = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item $Work -ItemType Directory | Out-Null

try {
    $ZipPath = Join-Path $Work $ArchiveName
    $ChecksumPath = Join-Path $Work 'checksum'
    curl.exe --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --show-error --location --output $ZipPath $DownloadUrl
    if ($LASTEXITCODE -ne 0) { throw 'Archive download failed' }
    curl.exe --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --show-error --location --output $ChecksumPath "$DownloadUrl.sha256"
    if ($LASTEXITCODE -ne 0) { throw 'Checksum download failed' }

    $Checksum = (Get-Content -Raw $ChecksumPath).Trim()
    if ($Checksum -notmatch ('^([a-fA-F0-9]{64}) [ *]' + [regex]::Escape($ArchiveName) + '$')) {
        throw 'Invalid SHA-256 checksum file'
    }
    $Expected = $Matches[1]
    if ((Get-FileHash -Algorithm SHA256 $ZipPath).Hash -ne $Expected) {
        throw 'SHA-256 checksum mismatch; vss was not installed'
    }

    # Copy exactly the flat executable without extracting arbitrary archive paths.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $Zip = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
    try {
        $Entry = $Zip.GetEntry('vss.exe')
        if (!$Entry -or $Entry.Length -eq 0) { throw 'Archive contains no vss.exe' }
        $InputStream = $Entry.Open()
        try {
            $BinaryPath = Join-Path $Work 'vss.exe'
            $OutputStream = [System.IO.File]::Create($BinaryPath)
            try { $InputStream.CopyTo($OutputStream) } finally { $OutputStream.Dispose() }
        } finally { $InputStream.Dispose() }
    } finally { $Zip.Dispose() }

    New-Item $BinDir -ItemType Directory -Force | Out-Null
    Move-Item $BinaryPath $vssExe -Force
} finally {
    Remove-Item $Work -Recurse -Force
}

# Tests and managed environments can opt out of persistent user PATH changes.
if (!$env:VSS_NO_MODIFY_PATH) {
    $User = [System.EnvironmentVariableTarget]::User
    $UserPath = [System.Environment]::GetEnvironmentVariable('Path', $User)
    if (!(";${UserPath};".ToLower() -like "*;${BinDir};*".ToLower())) {
        [System.Environment]::SetEnvironmentVariable('Path', "${UserPath};${BinDir}", $User)
        $Env:Path += ";${BinDir}"
    }
}
Write-Output "vss was installed successfully to ${vssExe}"
Write-Output "Run 'vss --help' to get started"
