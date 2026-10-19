<#
.SYNOPSIS
Builds the browser version of the game into target\web.

.DESCRIPTION
Builds the game for WebAssembly and lays out the Fight page beside it: the
page with the game's version, the engine's, and what is new in this version
filled in (from Cargo.toml, the engine Cargo pins, and CHANGELOG.md), the
wasm, and the two scripts that ship in crates, miniquad's loader (gl.js) and
the engine's patched quad-snd audio plugin (audio.js). The engine's site
serves this folder as its /fight/ page, beside the sandbox; the page reaches
the site's own visit counting, app manifest, and icons one folder up.

.EXAMPLE
.\tools\build_web.ps1 -Serve
Builds it and serves it at http://localhost:8081/ (the parts one folder up are
missing there; the engine's site build brings them).
#>
param(
    [switch]$Serve,
    [int]$Port = 8081
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$wasmTarget = "wasm32-unknown-unknown"
$siteDir = Join-Path $repoRoot "target/web"

function Get-Cargo {
    $fromPath = Get-Command cargo -ErrorAction SilentlyContinue
    if ($fromPath) { return $fromPath.Source }
    $candidate = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $candidate) { return $candidate }
    throw "Could not find cargo. Install Rust with: winget install Rustlang.Rustup; then restart PowerShell."
}

# v0.1 for 0.1.0 and v0.1.2 for 0.1.2, as the game and the engine write it.
function Get-Label([string]$Version) {
    $parts = $Version.Split(".")
    if ($parts.Count -eq 3 -and $parts[2] -eq "0") {
        return "v$($parts[0]).$($parts[1])"
    }
    return "v$Version"
}

# The changes listed under the newest entry of CHANGELOG.md, which must be the
# entry for this version.
function Get-NewestChanges([string]$Label) {
    $lines = [IO.File]::ReadAllLines((Join-Path $repoRoot "CHANGELOG.md"))
    $first = 0
    while ($first -lt $lines.Count -and -not $lines[$first].StartsWith("## ")) { $first++ }
    $heading = if ($first -lt $lines.Count) { $lines[$first] } else { "" }
    if ($heading -notmatch "^## $([regex]::Escape($Label)) \(\d{4}-\d{2}-\d{2}\)\s*$") {
        throw "CHANGELOG.md should start with an entry for $Label, headed '## $Label (YYYY-MM-DD)', not '$heading'. Bump Cargo.toml and CHANGELOG.md together."
    }
    $changes = @()
    for ($i = $first + 1; $i -lt $lines.Count -and -not $lines[$i].StartsWith("## "); $i++) {
        if ($lines[$i].StartsWith("- ")) { $changes += $lines[$i].Substring(2).Trim() }
    }
    if ($changes.Count -eq 0) { throw "CHANGELOG.md's entry for $Label lists no changes ('- ' lines)." }
    return $changes
}

$cargo = Get-Cargo
Push-Location $repoRoot
try {
    $json = & $cargo metadata --format-version 1 --filter-platform $wasmTarget
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed with exit code $LASTEXITCODE" }
    $metadata = ($json -join "`n") | ConvertFrom-Json
    $package = { param($name) $metadata.packages | Where-Object { $_.name -eq $name } | Select-Object -First 1 }
    $game = & $package "realistic_physics_fight"
    $engine = & $package "realistic_physics"
    $label = Get-Label $game.version
    $engineLabel = Get-Label $engine.version
    $changes = Get-NewestChanges $label

    Write-Host "==> Build the game for the web (wasm32, Release)"
    & $cargo build --release --target $wasmTarget --bin realistic_physics_fight
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

$wasm = Join-Path $repoRoot "target/$wasmTarget/release/realistic_physics_fight.wasm"
if (Test-Path -LiteralPath $siteDir) { Remove-Item -LiteralPath $siteDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $siteDir | Out-Null
Copy-Item -LiteralPath $wasm -Destination $siteDir
# The page's loader has to match the miniquad the game was built with, and the
# sound plugin is the engine's patched quad-snd's.
foreach ($script in @(@{ Package = "miniquad"; File = "js/gl.js" }, @{ Package = "quad-snd"; File = "js/audio.js" })) {
    $crate = & $package $script.Package
    $path = Join-Path (Split-Path -Parent $crate.manifest_path) $script.File
    if (-not (Test-Path -LiteralPath $path)) { throw "$($script.Package) $($crate.version) does not ship $($script.File)." }
    Copy-Item -LiteralPath $path -Destination $siteDir
}

# A returning visitor's browser may hold the last build's files for a while,
# so each URL carries its content's hash and every build loads its own.
function Get-Stamp([string]$Name) {
    return (Get-FileHash -LiteralPath (Join-Path $siteDir $Name) -Algorithm SHA256).Hash.Substring(0, 12).ToLowerInvariant()
}
$page = [IO.File]::ReadAllText((Join-Path $repoRoot "web/index.html"))
$news = ($changes | ForEach-Object { "<li>$([Net.WebUtility]::HtmlEncode($_))</li>" }) -join ""
$replacements = @(
    @{ From = '<span class="version"></span>'; To = "<span class=`"version`">$label</span>"; Count = 2 },
    @{ From = '<span class="engine-version"></span>'; To = "<span class=`"engine-version`">$engineLabel</span>"; Count = 1 },
    @{ From = '<ul class="news"></ul>'; To = "<ul class=`"news`">$news</ul>"; Count = 1 },
    @{ From = 'src="gl.js"'; To = "src=`"gl.js?v=$(Get-Stamp 'gl.js')`""; Count = 1 },
    @{ From = 'src="audio.js"'; To = "src=`"audio.js?v=$(Get-Stamp 'audio.js')`""; Count = 1 },
    @{ From = 'load("realistic_physics_fight.wasm")'; To = "load(`"realistic_physics_fight.wasm?v=$(Get-Stamp 'realistic_physics_fight.wasm')`")"; Count = 1 }
)
foreach ($replacement in $replacements) {
    $found = ([regex]::Matches($page, [regex]::Escape($replacement.From))).Count
    if ($found -ne $replacement.Count) {
        throw "web/index.html should contain $($replacement.From) $($replacement.Count) time(s), found $found."
    }
    $page = $page.Replace($replacement.From, $replacement.To)
}
[IO.File]::WriteAllText((Join-Path $siteDir "index.html"), $page, [Text.UTF8Encoding]::new($false))

$wasmKb = (Get-Item -LiteralPath (Join-Path $siteDir "realistic_physics_fight.wasm")).Length / 1KB
Write-Host ""
Write-Host ("Built the Fight page {0} on engine {1} in {2} (wasm {3:N0} KB)" -f $label, $engineLabel, $siteDir, $wasmKb)

if ($Serve) {
    & (Join-Path $PSScriptRoot "serve_web.ps1") -Port $Port -Root $siteDir
}
