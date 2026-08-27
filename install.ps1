# Kitsu Installer for Windows
# https://github.com/jmaxdev/Kitsu

[CmdletBinding()]
param (
    [string]$Version = "latest",
    [string]$InstallDir = "$HOME\.kitsu\bin"
)

$ErrorActionPreference = "Stop"

function Write-Info ($msg) {
    Write-Host "==> " -ForegroundColor Green -NoNewline
    Write-Host $msg -ForegroundColor White
}

function Write-WarnMsg ($msg) {
    Write-Host "warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $msg -ForegroundColor Yellow
}

function Write-ErrorMsg ($msg) {
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $msg -ForegroundColor Red
}

$Repo = "jmaxdev/Kitsu"
$AssetPattern = "x86_64-pc-windows-msvc.zip"

Write-Info "Installing Kitsu on Windows (x86_64)..."

# Ensure TLS 1.2+ is active
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

# Fetch release metadata from GitHub API
Write-Info "Fetching release information from GitHub..."

$Headers = @{
    "User-Agent" = "kitsu-installer"
    "Accept"     = "application/vnd.github.v3+json"
}

try {
    $Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases" -Headers $Headers -UseBasicParsing
} catch {
    Write-ErrorMsg "Failed to query GitHub Releases API: $_"
    exit 1
}

if (-not $Releases -or $Releases.Count -eq 0) {
    Write-ErrorMsg "No releases found for $Repo."
    exit 1
}

# Determine target release
$TargetRelease = $null
if ($Version -eq "latest") {
    foreach ($r in $Releases) {
        $matchingAsset = $r.assets | Where-Object { $_.name -like "*$AssetPattern*" }
        if ($matchingAsset) {
            $TargetRelease = $r
            break
        }
    }
} else {
    $tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
    $TargetRelease = $Releases | Where-Object { $_.tag_name -eq $tag }
}

if (-not $TargetRelease) {
    Write-ErrorMsg "Could not find a release matching version: $Version"
    exit 1
}

$Asset = $TargetRelease.assets | Where-Object { $_.name -like "*$AssetPattern*" } | Select-Object -First 1
if (-not $Asset) {
    Write-ErrorMsg "Could not find Windows asset ($AssetPattern) in release $($TargetRelease.tag_name)."
    exit 1
}

$DownloadUrl = $Asset.browser_download_url
$ReleaseTag = $TargetRelease.tag_name

Write-Info "Downloading Kitsu $ReleaseTag from $DownloadUrl..."

$TempDir = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$ZipFile = Join-Path $TempDir "kitsu.zip"

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipFile -UseBasicParsing -Headers $Headers
    
    Write-Info "Extracting archive..."
    Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force

    $ExtractedExe = Get-ChildItem -Path $TempDir -Filter "kitsu.exe" -Recurse | Select-Object -First 1
    if (-not $ExtractedExe) {
        Write-ErrorMsg "Could not find kitsu.exe inside downloaded archive."
        exit 1
    }

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $DestinationExe = Join-Path $InstallDir "kitsu.exe"
    Copy-Item -Path $ExtractedExe.FullName -Destination $DestinationExe -Force

    Write-Info "Installed Kitsu executable to $DestinationExe"
} finally {
    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Update User PATH environment variable
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$Paths = ($UserPath -split ";") | Where-Object { $_ -ne "" }

if ($Paths -notcontains $InstallDir) {
    $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Info "Added $InstallDir to user PATH environment variable."
}

# Also update current session PATH
if (($env:PATH -split ";") -notcontains $InstallDir) {
    $env:PATH = "$InstallDir;$env:PATH"
}

Write-Host ""
Write-Host "Kitsu $ReleaseTag was successfully installed!" -ForegroundColor Green
Write-Host ""
Write-Host "Verify installation by running:" -ForegroundColor Cyan
Write-Host "  kitsu --version" -ForegroundColor White
Write-Host "  kitsu ignite" -ForegroundColor White
Write-Host ""
