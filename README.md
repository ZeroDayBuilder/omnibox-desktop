# OmniBox Desktop

A desktop manager for [QEMU](https://www.qemu.org/) virtual machines. Drop in an ISO — OmniBox detects the OS, suggests CPU/RAM/disk, builds a sensible QEMU command line, and supervises the guest (start, pause, resume, shut down).

It does **not** reimplement a hypervisor. It is a front-end: correct args, process control over QMP, and a simple UI.

| Layer | Stack |
| ----- | ----- |
| UI | Tauri 2 · React 18 · Tailwind (dark UI) |
| Backend | Rust (Tokio) — `qemu-system-*`, `qemu-img`, `swtpm`, QMP, `sysinfo` |
| Acceleration | KVM (Linux), Hypervisor.framework (macOS), WHPX (Windows), TCG fallback |

**Version: 0.1.1** — unit tests green (`cargo test` 10/10). Manually exercised on a Linux/KVM host with real ISOs (see below). Still early: not a VirtualBox/VMware replacement.

## What works today

| Area | Status |
| ---- | ------ |
| ISO wizard | Drag/drop or browse; volume label + name heuristics for Windows 7/10/11/Server, many Linux distros, BSDs |
| Lifecycle | Start, pause, resume, ACPI shut down, force off |
| Dashboard | Machine list, live CPU/memory sampling, Overview / VMs / ISO Library / Settings / Logs / About |
| Firmware | UEFI (OVMF) or legacy BIOS; Win7 → BIOS; Win11 → UEFI (+ Secure Boot firmware when `*.ms.fd` / `*.secboot.fd` is installed) |
| TPM 2.0 | `swtpm` per VM when available (important for Windows 11 on Linux) |
| Disks | Default **qcow2 sparse** (`preallocation=off`) — host space grows as the guest writes (like dynamic VBox disks). VDI/raw also supported |
| Post-install boot | Prefers hard disk when the image already has data; can auto-detach installer ISO |
| Windows drivers path | SATA + e1000e + VGA so Setup sees a disk without virtio drivers first |

### Guests we have actually installed (Linux host + KVM)

- Windows 11  
- Windows 10  
- Kali Linux  
- Kodachi Linux  

Other Linux ISOs and Windows variants should work with the same recipes; treat them as “expected”, not every distro certified.

```bash
cd src-tauri && cargo test
# 10 passed (ISO detect + QEMU plan builder)
```

## Requirements

Install QEMU (and helpers) **before** running OmniBox. It searches `PATH` and common install dirs; override with `OMNIBOX_QEMU_DIR`.

**Debian / Ubuntu / Kali**
```bash
sudo apt install -y qemu-system-x86 qemu-utils ovmf swtpm
sudo usermod -aG kvm "$USER"   # log out and back in
```

**Fedora**
```bash
sudo dnf install qemu-system-x86-core qemu-img edk2-ovmf swtpm
```

**Arch**
```bash
sudo pacman -S qemu-desktop swtpm edk2-ovmf
```

**macOS**
```bash
brew install qemu swtpm
```

**Windows 10/11 (as host)**  
1. [QEMU for Windows](https://qemu.weilnetz.de/w64/) → e.g. `C:\Program Files\qemu`  
2. Enable *Windows Hypervisor Platform*, reboot  
3. Note: `swtpm` is awkward on Windows hosts — Win11 guests may need Setup LabConfig bypass (see limitations)

## Build from source

Need: **Node 18+**, **Rust stable**, [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
git clone https://github.com/<you>/omnibox-desktop.git
cd omnibox-desktop
npm install
npm run icons          # once — generates icons from app-icon.svg
npm run tauri dev      # development
# npm run tauri build  # installers under src-tauri/target/release/bundle
```

Checks:
```bash
npm run typecheck
cd src-tauri && cargo test && cargo clippy --all-targets
```

## Guest configuration (defaults)

| Guest | Machine | Firmware | Disk | NIC | GPU | TPM |
| ----- | ------- | -------- | ---- | --- | --- | --- |
| Windows 11 | q35 (+ SMM when needed) | UEFI (prefer MS/secboot OVMF) | SATA | e1000e | VGA | swtpm if present |
| Windows 10 | q35 | UEFI | SATA | e1000e | VGA | optional |
| Windows 7 | q35 | **BIOS** | SATA | e1000e | VGA | off |
| Linux / BSD x86_64 | q35 | UEFI | virtio-blk | virtio-net | virtio-vga | off |
| aarch64 | virt | UEFI | virtio | virtio | virtio-gpu / ramfb | optional |

After Windows install, you can switch the disk to virtio once virtio-win drivers are installed.

**Network:** NAT is the default (guest internet via user-mode networking + DNS). Bridged mode needs a host bridge/TAP and is advanced.

## Windows 11 Setup tips

If Setup says the PC must support **Secure Boot** or **TPM**:

1. Prefer host packages that ship `OVMF_*ms*.fd` / `*secboot*.fd`, then restart OmniBox.  
2. Or in Setup: `Shift+F10` → run:
```text
reg add "HKLM\SYSTEM\Setup\LabConfig" /f
reg add "HKLM\SYSTEM\Setup\LabConfig" /v BypassSecureBootCheck /t REG_DWORD /d 1 /f
reg add "HKLM\SYSTEM\Setup\LabConfig" /v BypassTPMCheck /t REG_DWORD /d 1 /f
```
3. Wi-Fi pages in OOBE: VMs use **virtual Ethernet**, not Wi-Fi — use “I don’t have internet” / limited setup, or `OOBE\BYPASSNRO` from `Shift+F10`.

## Known limitations

- **macOS guests** are not supported (licence + reliability). ISOs are detected and refused with an explanation.  
- **Not** a drop-in VirtualBox/VMware clone: no snapshots yet, display is QEMU’s own window (not embedded), USB is manual vendor/product IDs.  
- **TPM on Windows hosts:** `swtpm` generally unavailable — use LabConfig bypass for Win11 if needed.  
- **Cross-arch** guests fall back to TCG (slow).  
- **3D GL** only for Linux guests on Linux hosts.  
- **Bridged networking** and polished shared-folder UX still need host setup / more UI work.  
- Release installers (if built) may not bundle QEMU — install QEMU on the host.

## Roadmap

- **0.2** — snapshots (create / restore / list)  
- Port-forward UI, better shared folders & USB picker  
- Embedded display (SPICE/VNC)  
- Optional bundled QEMU for Windows/macOS hosts  

## Project layout

```
omnibox-desktop/
├── src/                  # React UI (App, VmRow, IsoWizard, Settings, …)
├── src-tauri/src/
│   ├── qemu.rs           # pure QEMU argv builder (unit-tested)
│   ├── iso.rs            # ISO detect + hardware recommendations
│   ├── runtime.rs        # spawn, QMP, stats
│   ├── accel.rs          # host tools + OVMF discovery
│   ├── models.rs store.rs commands.rs main.rs
├── scripts/              # e.g. Win11 LabConfig notes
└── .github/workflows/    # ci.yml, release.yml
```

**Data directory (Linux):** `~/.local/share/dev.omnibox.desktop/`  
— `configs/`, `machines/<id>/` (disk, UEFI vars, TPM state, `qemu.log`)

## License

MIT — see [LICENSE](LICENSE).
