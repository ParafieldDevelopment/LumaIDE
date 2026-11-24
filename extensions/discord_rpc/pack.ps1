param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$LumaDir = (Join-Path $env:APPDATA "Luma")
)

# Builds the Discord RPC extension, zips it with extension.json at the root,
# and installs it into LUMA_DIR/extensions.
# Native tools (rustup/cargo) write progress to stderr, which "Stop" treats as
# a failure. Use Continue and check exit codes instead.
$ErrorActionPreference = "Continue"

$extensionDir = $PSScriptRoot
Push-Location $extensionDir
try {
    rustup target add $Target
    if ($LASTEXITCODE -ne 0) { throw "rustup target add failed" }
    cargo build --release --target $Target
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    $stage = Join-Path $extensionDir ".pack/$Target"
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
    $libraryDir = Join-Path $stage "native/$Target"
    New-Item -ItemType Directory -Force $libraryDir | Out-Null
    Copy-Item (Join-Path $extensionDir "extension.json") (Join-Path $stage "extension.json")
    Copy-Item (Join-Path $extensionDir "target/$Target/release/discord_rpc.dll") (Join-Path $libraryDir "discord_rpc.dll")

    $zip = Join-Path $extensionDir "discord-rpc-$Target.zip"
    Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -Force

    $installDir = Join-Path $LumaDir "extensions"
    New-Item -ItemType Directory -Force $installDir | Out-Null
    Copy-Item $zip $installDir -Force
    Write-Host "Installed $zip -> $installDir"
}
finally {
    Pop-Location
}
