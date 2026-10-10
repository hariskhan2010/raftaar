<#
.SYNOPSIS
  Builds a signed Raftaar release and the latest.json the in-app updater reads.

.DESCRIPTION
  1. Bump "version" in src-tauri/tauri.conf.json (and package.json) first.
  2. Run:  .\scripts\release.ps1 -Notes "What changed"
  3. Upload ALL THREE files it prints to a GitHub release tagged v<version> on
     github.com/hariskhan2010/raftaar. Mark it "latest". Installed copies of
     Raftaar find the update on their next start.

  The update signing key lives in %USERPROFILE%\.tauri\raftaar-updater.key and
  must never be committed or shared. If it is lost, existing installs can no
  longer be updated (they would need a manual reinstall).
#>
param(
    [string]$Notes = '',
    [string]$Repo = 'hariskhan2010/raftaar'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$key = Join-Path $env:USERPROFILE '.tauri\raftaar-updater.key'
if (-not (Test-Path $key)) { throw "Update signing key not found at $key" }

$conf = Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$version = $conf.version

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Push-Location $root
try {
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
} finally {
    Pop-Location
}

$target = (Get-Content (Join-Path $root '.cargo\config.toml') -Raw | Select-String 'target-dir\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$nsis = Join-Path $target 'release\bundle\nsis'
$setup = Join-Path $nsis "Raftaar_${version}_x64-setup.exe"
$sig = "$setup.sig"
if (-not (Test-Path $sig)) { throw "Signature $sig was not created" }

$manifest = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = (Get-Content $sig -Raw).Trim()
            url       = "https://github.com/$Repo/releases/download/v$version/Raftaar_${version}_x64-setup.exe"
        }
    }
}
$json = Join-Path $nsis 'latest.json'
# No BOM: Windows PowerShell's "utf8" adds one, and the updater's JSON parser rejects it.
[IO.File]::WriteAllText($json, ($manifest | ConvertTo-Json -Depth 5), (New-Object System.Text.UTF8Encoding $false))

# Same installer under a fixed name: the website's Download button points at
# releases/latest/download/Raftaar-Setup.exe, so it always gets the newest version.
$fixed = Join-Path $nsis 'Raftaar-Setup.exe'
Copy-Item $setup $fixed -Force

Write-Output "Release v$version ready. Upload these three files to the GitHub release v${version}:"
Write-Output "  $setup"
Write-Output "  $fixed"
Write-Output "  $json"
Write-Output "Or in one go: gh release create v$version `"$setup`" `"$fixed`" `"$json`" --title `"Raftaar $version`" --notes `"...`" --latest"
