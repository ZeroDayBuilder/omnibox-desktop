# Build OmniBox Desktop 0.1.1

This project is a complete source tree. Unit tests for the QEMU argument builder and ISO detector pass.
A full GUI binary needs **your** machine (GTK/WebKit or platform equivalents are not available in every CI sandbox).

## Prerequisites

| Tool | Version |
|------|---------|
| Node.js | 20+ |
| Rust | stable ≥ 1.77 (`rustup update`) |
| QEMU | system package (see README) |
| Tauri system deps | https://v2.tauri.app/start/prerequisites/ |

### Linux (Ubuntu/Debian)

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  qemu-system-x86 qemu-system-arm qemu-utils swtpm ovmf
```

### macOS

```bash
brew install qemu swtpm
# Xcode CLT required for Tauri
```

### Windows

- Install QEMU from https://qemu.weilnetz.de/w64/
- Enable *Windows Hypervisor Platform*
- Visual Studio Build Tools / MSVC for Tauri

## Build steps

```bash
cd omnibox-desktop
npm install
npm run icons          # generates real icons from app-icon.svg (required once)
npm run typecheck
cd src-tauri && cargo test && cargo clippy --all-targets && cd ..
npm run tauri dev      # development
npm run tauri build    # installers under src-tauri/target/release/bundle
```

## What the unit tests cover

```bash
cd src-tauri && cargo test
```

- Windows 11 → UEFI + TPM + SATA + e1000e
- Windows 7 → legacy BIOS
- Linux → KVM + virtio
- aarch64 → virt machine + virtio (no AHCI)
- Missing OVMF for Win11 → hard error
- No KVM → TCG + note

## Honest limits of this environment

The agent environment cannot:

- Link against system WebKit/GTK (no `libwebkit2gtk` packages)
- Produce signed `.dmg` / `.msi` / notarized macOS builds
- Boot real guest ISOs

Source is structured so that **on a normal developer machine with the prerequisites above, `npm run tauri build` is the expected path to a working app.**
