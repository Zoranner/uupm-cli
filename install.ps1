param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.uupm\bin",
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"

$Repo = "Zoranner/uupm-cli"
$Target = "x86_64-pc-windows-msvc"
$Asset = "uupm-$Target.zip"

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "uupm release binaries currently support 64-bit Windows only."
}

if ($Version -eq "latest") {
    $Latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $ResolvedVersion = $Latest.tag_name
} else {
    $ResolvedVersion = $Version
}

if (-not $ResolvedVersion) {
    throw "cannot resolve uupm release version."
}

$Url = "https://github.com/$Repo/releases/download/$ResolvedVersion/$Asset"
$ExePath = Join-Path $InstallDir "uupm.exe"

$ExistingCommand = Get-Command uupm -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not (Test-Path -LiteralPath $ExePath) -and $ExistingCommand) {
    $ExePath = $ExistingCommand.Source
    $InstallDir = Split-Path -Parent $ExePath
}

if (Test-Path -LiteralPath $ExePath) {
    $Current = (& $ExePath --version 2>$null) -replace "^uupm\s+", "v"
    if ($Current -eq $ResolvedVersion) {
        Write-Host "uupm $ResolvedVersion is already installed at $ExePath"
        return
    }
    Write-Host "Updating uupm from $Current to $ResolvedVersion"
} else {
    Write-Host "Installing uupm $ResolvedVersion"
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("uupm-install-" + [Guid]::NewGuid())
$Archive = Join-Path $TempDir $Asset

New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

try {
    Write-Host "Downloading $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Archive

    Expand-Archive -Path $Archive -DestinationPath $TempDir -Force
    Copy-Item -LiteralPath (Join-Path $TempDir "uupm.exe") -Destination $ExePath -Force

    if (-not $NoPathUpdate -and -not $ExistingCommand) {
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $Entries = @()
        if ($UserPath) {
            $Entries = $UserPath -split ";" | Where-Object { $_ }
        }
        $Exists = $Entries | Where-Object { $_.TrimEnd("\") -ieq $InstallDir.TrimEnd("\") }
        if (-not $Exists) {
            $NewPath = (($Entries + $InstallDir) -join ";")
            [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
            $env:Path = "$env:Path;$InstallDir"
            Write-Host "Added $InstallDir to the user PATH. Restart your terminal if uupm is not found."
        }
    }

    & $ExePath --version
    Write-Host "Installed uupm to $InstallDir"
} finally {
    Remove-Item -LiteralPath $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
