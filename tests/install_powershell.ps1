param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$Root = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item $Root -ItemType Directory | Out-Null
$env:VSS_INSTALL = Join-Path $Root 'install'
$env:VSS_NO_MODIFY_PATH = '1'
$Archive = Join-Path $Root 'fixture.zip'
$Source = Join-Path $Root 'source'
New-Item $Source -ItemType Directory | Out-Null
Set-Content (Join-Path $Source 'vss.exe') 'verified binary' -NoNewline
Set-Content (Join-Path $Source 'unexpected.txt') 'not installed'
[IO.Compression.ZipFile]::CreateFromDirectory($Source, $Archive)
# The mock runs in the invoked installer's script scope. Keep fixture state
# explicit rather than resolving $script: variables in that caller's scope.
$global:VssInstallerTestState = @{
    Digest = (Get-FileHash -Algorithm SHA256 $Archive).Hash
    Corrupt = $false
    CurlCalls = @()
    Archive = $Archive
}
function global:curl.exe {
    $global:VssInstallerTestState.CurlCalls += ,@($args)
    $OutputPath = $args[[array]::IndexOf($args, '--output') + 1]
    $Url = $args[-1]
    if ($Url.EndsWith('.sha256')) {
        $Hash = if ($global:VssInstallerTestState.Corrupt) { '0' * 64 } else { $global:VssInstallerTestState.Digest }
        Set-Content $OutputPath "$Hash  vss_windows_amd64.zip"
    } else { Copy-Item $global:VssInstallerTestState.Archive $OutputPath }
    $global:LASTEXITCODE = 0
}
try {
    New-Item (Join-Path $env:VSS_INSTALL 'bin') -ItemType Directory | Out-Null
    $Destination = Join-Path $env:VSS_INSTALL 'bin/vss.exe'
    Set-Content $Destination 'old binary' -NoNewline
    $global:VssInstallerTestState.Corrupt = $true
    $Rejected = $false
    try { & $Installer } catch {
        if ($_.Exception.Message -notlike '*checksum mismatch*') { throw }
        $Rejected = $true
    }
    if (!$Rejected -or (Get-Content -Raw $Destination) -ne 'old binary') { throw 'Corrupt archive was installed' }
    $global:VssInstallerTestState.Corrupt = $false
    & $Installer
    if ((Get-Content -Raw $Destination) -ne 'verified binary') { throw 'Verified binary not installed' }
    if (Test-Path (Join-Path $env:VSS_INSTALL 'bin/unexpected.txt')) { throw 'Unexpected file was extracted' }
    foreach ($Call in $global:VssInstallerTestState.CurlCalls) {
        if ($Call[-1] -notlike 'https://github.com/veltiosoft/vss/releases/*') { throw 'Wrong release repository' }
        foreach ($Flag in @('--proto', '--proto-redir', '--tlsv1.2', '--fail')) {
            if ($Call -notcontains $Flag) { throw "Missing security flag: $Flag" }
        }
    }
} finally {
    Remove-Item $Root -Recurse -Force
    Remove-Variable VssInstallerTestState -Scope Global
}
