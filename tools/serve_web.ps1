<#
.SYNOPSIS
Serves a web build locally, with the content types browsers need.

.DESCRIPTION
Serves target\web (or -Root) at http://localhost:8081/ until Ctrl+C. Browsers
only compile WebAssembly served as application/wasm, which opening the page as
a file or many quick servers do not give.
#>
param(
    [int]$Port = 8081,
    [string]$Root = ""
)

$ErrorActionPreference = "Stop"
if (-not $Root) { $Root = Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")).Path "target/web" }
if (-not (Test-Path -LiteralPath (Join-Path $Root "index.html"))) {
    throw "No web build found in $Root. Run .\tools\build_web.ps1 first."
}
$Root = (Resolve-Path -LiteralPath $Root).Path
$rootPrefix = $Root.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
$types = @{
    ".html" = "text/html; charset=utf-8"
    ".js" = "text/javascript; charset=utf-8"
    ".wasm" = "application/wasm"
    ".webmanifest" = "application/manifest+json"
    ".png" = "image/png"
}

$url = "http://localhost:$Port/"
$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add($url)
$listener.Start()
Write-Host "Serving $Root at $url (add ?stats for frame timings). Press Ctrl+C to stop."
try {
    while ($listener.IsListening) {
        $pending = $listener.GetContextAsync()
        while (-not $pending.Wait(250)) { }
        $context = $pending.Result
        $response = $context.Response
        try {
            $relative = [Uri]::UnescapeDataString($context.Request.Url.AbsolutePath).TrimStart('/')
            if (-not $relative -or $relative.EndsWith('/')) { $relative += "index.html" }
            $path = [IO.Path]::GetFullPath((Join-Path $Root $relative))
            $type = $types[[IO.Path]::GetExtension($path).ToLowerInvariant()]
            if ($path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase) -and $type -and (Test-Path -LiteralPath $path -PathType Leaf)) {
                $bytes = [IO.File]::ReadAllBytes($path)
                $response.ContentType = $type
                $response.Headers["Cache-Control"] = "no-store"
                $response.ContentLength64 = $bytes.Length
                if ($context.Request.HttpMethod -ne "HEAD") { $response.OutputStream.Write($bytes, 0, $bytes.Length) }
            } else {
                $response.StatusCode = 404
            }
        } catch {
            Write-Warning "$($context.Request.HttpMethod) $($context.Request.Url.AbsolutePath): $($_.Exception.Message)"
        } finally {
            try { $response.Close() } catch { }
        }
    }
} finally {
    $listener.Stop()
    $listener.Close()
}
