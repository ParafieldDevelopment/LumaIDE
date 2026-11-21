param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$Wasm,
    [string]$LumaDir = (Join-Path $env:APPDATA "Luma")
)

# Builds the example extension, creates an extension ZIP with everything at
# the root, and installs it into LUMA_DIR/extensions for local testing.
$ErrorActionPreference = "Stop"

$extensionDir = $PSScriptRoot
Push-Location $extensionDir
try {
    if ($Wasm) {
        $buildTarget = "wasm32-unknown-unknown"
        rustup target add $buildTarget
        cargo build --release --target $buildTarget

        $stage = Join-Path $extensionDir ".pack/wasm"
        Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Force $stage | Out-Null
        Copy-Item (Join-Path $extensionDir "extension.wasm.json") (Join-Path $stage "extension.json")
        Copy-Item (Join-Path $extensionDir "target/$buildTarget/release/example_extension.wasm") (Join-Path $stage "example_extension.wasm")
        $zip = Join-Path $extensionDir "example-extension-wasm.zip"
    }
    else {
        rustup target add $Target
        cargo build --release --target $Target

        $stage = Join-Path $extensionDir ".pack/$Target"
        Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
        $libraryDir = Join-Path $stage "native/$Target"
        New-Item -ItemType Directory -Force $libraryDir | Out-Null
        Copy-Item (Join-Path $extensionDir "extension.json") (Join-Path $stage "extension.json")
        Copy-Item (Join-Path $extensionDir "target/$Target/release/example_extension.dll") (Join-Path $libraryDir "example_extension.dll")
        $zip = Join-Path $extensionDir "example-extension-$Target.zip"
    }

    Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -Force

    $installDir = Join-Path $LumaDir "extensions"
    New-Item -ItemType Directory -Force $installDir | Out-Null
    Copy-Item $zip $installDir -Force
    Write-Host "Installed $zip -> $installDir"
}
finally {
    Pop-Location
}
