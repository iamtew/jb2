set shell := ["powershell.exe", "-NoProfile", "-Command"]

binary := "jb2.exe"
output_dir := "dist"

default:
    just --list

icon:
    magick webroot/img/jukeboks.png -background none -define icon:auto-resize=256,48,32,16 webroot/img/jukeboks.ico
    foreach ($n in @('home','settings','artboard','device-tv','copy','info-circle','refresh','x')) { magick -background none "src/tray_icons/$n.svg" -resize 32x32 "src/tray_icons/$n.png" }

build:
    cargo build --release
    Copy-Item -Path target/release/jb2.exe -Destination {{binary}} -Force

clean:
    cargo clean
    if (Test-Path {{binary}}) { Remove-Item {{binary}} -Force }
    if (Test-Path {{output_dir}}) { Remove-Item {{output_dir}} -Recurse -Force }

run:
    cargo run --release

test:
    cargo test

fmt:
    cargo fmt

package:
    if (Test-Path {{output_dir}}) { Remove-Item {{output_dir}} -Recurse -Force }
    New-Item -ItemType Directory -Path {{output_dir}} -Force | Out-Null
    cargo build --release
    Copy-Item -Path target/release/jb2.exe -Destination {{output_dir}}/{{binary}} -Force
    Copy-Item -Path webroot -Destination {{output_dir}}/webroot -Recurse -Force

# Unpacked dist plus zip and MSI. Version is Cargo.toml.
build-dist: package
    $ver = ([regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"')).Groups[1].Value; if (-not $ver) { Write-Error "no version in Cargo.toml"; exit 1 }; Compress-Archive -Path '{{output_dir}}\{{binary}}','{{output_dir}}\webroot' -DestinationPath "{{output_dir}}\jb2-$ver-win-x64.zip" -Force
    @just msi-only

# Per-user MSI from a fresh package. Needs wix on PATH: dotnet tool install -g wix
msi: package
    @just msi-only

# MSI from an existing dist/ tree (no cargo rebuild).
msi-only:
    if (-not (Get-Command wix -ErrorAction SilentlyContinue)) { Write-Error "wix not on PATH. Install: dotnet tool install -g wix"; exit 1 }
    $ver = ([regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"')).Groups[1].Value; if (-not $ver) { Write-Error "no version in Cargo.toml"; exit 1 }; $payload = (Resolve-Path '{{output_dir}}').Path; wix build -wx -arch x64 -ext WixToolset.UI.wixext -b "payload=$payload" -d Version=$ver -o "{{output_dir}}\jb2-$ver-win-x64.msi" packaging/jb2.wxs; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

verify-package:
    @just package
    if (-not (Test-Path {{output_dir}}/{{binary}})) { Write-Error "missing exe"; exit 1 }
    if (-not (Test-Path {{output_dir}}/webroot/admin/index.html)) { Write-Error "missing admin"; exit 1 }
    if (-not (Test-Path {{output_dir}}/webroot/overlay/index.html)) { Write-Error "missing overlay"; exit 1 }
