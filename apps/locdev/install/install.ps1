# Installs the standalone LocDev CLI (locdev.exe + locdevd.exe) from a GitHub
# release, verifying the archive against the release's SHA256SUMS first.
#
#   irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
#
# Options (environment):
#   LOCDEV_VERSION      tag to install (default: the latest release), e.g. v0.8.0
#   LOCDEV_INSTALL_DIR  where the binaries go (default: %LOCALAPPDATA%\LocDev\cli)
#   LOCDEV_REPO         owner/name of the repository (default: trbros/locdev)
#
# No administrator rights: it writes the two programs in the folder above and
# adds that folder to your user Path.
$ErrorActionPreference = 'Stop'

$repo = if ($env:LOCDEV_REPO) { $env:LOCDEV_REPO } else { 'trbros/locdev' }
$dir = if ($env:LOCDEV_INSTALL_DIR) { $env:LOCDEV_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'LocDev\cli' }
$target = 'x86_64-pc-windows-msvc'

$tag = $env:LOCDEV_VERSION
if (-not $tag) {
  $tag = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest").tag_name
  if (-not $tag) { throw "could not find the latest release of $repo" }
}

$name = "locdev-$tag-$target"
$base = "https://github.com/$repo/releases/download/$tag"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Host "Downloading $name.zip..."
  Invoke-WebRequest "$base/$name.zip" -OutFile (Join-Path $tmp "$name.zip") -UseBasicParsing
  Invoke-WebRequest "$base/SHA256SUMS" -OutFile (Join-Path $tmp 'SHA256SUMS') -UseBasicParsing

  $line = Get-Content (Join-Path $tmp 'SHA256SUMS') | Where-Object { ($_ -split '\s+')[1] -in @("$name.zip", "*$name.zip") }
  if (-not $line) { throw "$name.zip is not in SHA256SUMS" }
  $expected = ($line -split '\s+')[0].ToLower()
  $actual = (Get-FileHash (Join-Path $tmp "$name.zip") -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { throw 'checksum mismatch: the download is not the published file' }

  Expand-Archive (Join-Path $tmp "$name.zip") -DestinationPath $tmp
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  foreach ($bin in 'locdev.exe', 'locdevd.exe') {
    Copy-Item (Join-Path $tmp "$name\$bin") (Join-Path $dir $bin) -Force
  }
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$path = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (("$path" -split ';') -contains $dir)) {
  [Environment]::SetEnvironmentVariable('Path', (("$path".TrimEnd(';')), $dir -join ';').TrimStart(';'), 'User')
  Write-Host "Added $dir to your user Path (open a new terminal)."
}
Write-Host "Installed $(& (Join-Path $dir 'locdev.exe') --version) in $dir"
Write-Host 'Next: `locdev start`, then `locdev` opens the terminal UI (use Windows Terminal).'
