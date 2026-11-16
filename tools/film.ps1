<#
.SYNOPSIS
Films the game: plays a script of the players' moves and makes an MP4.

.DESCRIPTION
Runs the game's capture mode (--capture --script --every, src/capture.rs)
into a folder of frames beside the film, then joins the frames into an MP4
with ffmpeg, at the speed the game plays (60 steps a second) or slower with
-Slow. Builds the game first (release) unless -SkipBuild. Writes only the
film and its frames; touches nothing else.

.PARAMETER Out
The MP4 to write, such as output\round.mp4. Its frames go in a folder of
the same name beside it, emptied first.

.PARAMETER Script
The players' moves, as --script takes them (src/capture.rs lists the steps),
such as "wait=40:in1:wait=50:jab1:wait=60".

.PARAMETER Gesture
Both fighters' moves as the engine writes them (--gesture, src/capture.rs), in
place of -Script: an engine random fight printed with RP_SEED and RP_GESTURE=1
plays in the game step for step.

.PARAMETER Every
Save a frame every N steps (default 2: 30 frames a second of play).

.PARAMETER Slow
How many times slower than real the film plays (default 1).

.PARAMETER From
The step the film starts at (default 0): the script still plays from the
start, but only frames from this step on go in the film.

.EXAMPLE
.\tools\film.ps1 -Out output\round.mp4 -Script "wait=40:in1:wait=50:cross1:wait=90" -Seed 3
A round against the sparring partner as it plays, at 1280x720.

.EXAMPLE
.\tools\film.ps1 -Out output\phone.mp4 -Size 844x390 -Touch -Script "wait=30:in1:wait=60"
The same on a phone held sideways, with its buttons as fingers see them.
#>
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [string]$Script = "",
    [string]$Gesture = "",
    [int]$Seed = 1,
    [string]$Size = "1280x720",
    [int]$Every = 2,
    [double]$Slow = 1.0,
    [int]$From = 0,
    [int]$Frames = 0,
    [switch]$Touch,
    [switch]$Sandbox,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if (-not $ffmpeg) {
    throw "ffmpeg is not on PATH. Install it with: winget install Gyan.FFmpeg; then restart PowerShell."
}

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        Write-Host "==> Build the game (Release)"
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}
$exe = Join-Path $repoRoot "target/release/realistic_physics_fight.exe"
if (-not (Test-Path -LiteralPath $exe)) { throw "No game at $exe; build it first." }

$film = [IO.Path]::GetFullPath($Out)
$name = [IO.Path]::GetFileNameWithoutExtension($film)
$framesDir = Join-Path ([IO.Path]::GetDirectoryName($film)) $name
New-Item -ItemType Directory -Force -Path $framesDir | Out-Null
Get-ChildItem -LiteralPath $framesDir -Filter "$name*.png" | Remove-Item

$arguments = @("--capture", (Join-Path $framesDir "$name.png"), "--every", "$Every", "--seed", "$Seed", "--size", $Size)
if ($Script) { $arguments += @("--script", $Script) }
if ($Gesture) { $arguments += @("--gesture", $Gesture) }
if ($Frames -gt 0) { $arguments += @("--frames", "$Frames") }
if ($Touch) { $arguments += "--touch" }
if ($Sandbox) { $arguments += "--sandbox" }
Write-Host "==> Play the script and save every $Every steps"
& $exe @arguments | Select-Object -Last 1
if ($LASTEXITCODE -ne 0) { throw "the game's capture failed with exit code $LASTEXITCODE" }

# The frames are numbered by step (name-0002.png, name-0004.png, ...), which
# ffmpeg's numbered input cannot follow, so it reads them from a list.
$shots = @(Get-ChildItem -LiteralPath $framesDir -Filter "$name-*.png" | Sort-Object Name |
        Where-Object { [int]($_.BaseName.Substring($name.Length + 1)) -ge $From })
if ($shots.Count -eq 0) { throw "the capture saved no frames in $framesDir from step $From on" }
# A frame saved while the display sleeps, or the window is hidden, comes out
# empty (1x1): a PNG's width and height are big-endian at bytes 16 and 20.
$wide, $tall = $Size -split "[xX]" | ForEach-Object { [int]$_ }
$empty = @($shots | Where-Object {
        $head = [byte[]]::new(24)
        $stream = [IO.File]::OpenRead($_.FullName)
        try { [void]$stream.Read($head, 0, 24) } finally { $stream.Dispose() }
        $w = ($head[16] -shl 24) -bor ($head[17] -shl 16) -bor ($head[18] -shl 8) -bor $head[19]
        $h = ($head[20] -shl 24) -bor ($head[21] -shl 16) -bor ($head[22] -shl 8) -bor $head[23]
        # High-density screens save more pixels than the window's size.
        $w -lt $wide -or $h -lt $tall
    })
if ($empty.Count -gt 0) {
    throw "$($empty.Count) of $($shots.Count) frames came out smaller than $Size, the first $($empty[0].Name): the display slept or the window was hidden while it played. Wake the display and run it again."
}
$rate = 60.0 / $Every / $Slow
$seconds = (1.0 / $rate).ToString("0.000000", [Globalization.CultureInfo]::InvariantCulture)
$list = Join-Path $framesDir "frames.txt"
$lines = foreach ($shot in $shots) { "file '$($shot.Name)'"; "duration $seconds" }
$lines += "file '$($shots[-1].Name)'"
[IO.File]::WriteAllLines($list, $lines)
$fps = [Math]::Max(30, [Math]::Ceiling($rate)).ToString([Globalization.CultureInfo]::InvariantCulture)
Write-Host "==> Join $($shots.Count) frames into $film"
& $ffmpeg.Source -y -loglevel error -f concat -safe 0 -i $list -vf "fps=$fps,format=yuv420p" -c:v libx264 -crf 20 -movflags +faststart $film
if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed with exit code $LASTEXITCODE" }
$length = $shots.Count / $rate
Write-Host ("Wrote {0} ({1:N1} s, {2} frames)" -f $film, $length, $shots.Count)
