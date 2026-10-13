set shell := ["powershell.exe", "-NoProfile", "-Command"]

binary := "jb2.exe"
archive_name := "jb2-dist"
output_dir := "dist"

default:
    just --list

icon:
    magick webroot/img/jukeboks.png -background none -define icon:auto-resize=256,48,32,16 webroot/img/jukeboks.ico

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

verify-package:
    @just package
    if (-not (Test-Path {{output_dir}}/{{binary}})) { Write-Error "missing exe"; exit 1 }
    if (-not (Test-Path {{output_dir}}/webroot/admin/index.html)) { Write-Error "missing admin"; exit 1 }
    if (-not (Test-Path {{output_dir}}/webroot/overlay/index.html)) { Write-Error "missing overlay"; exit 1 }
