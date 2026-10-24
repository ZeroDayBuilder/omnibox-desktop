use crate::{accel, iso, models::*, runtime, store};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::State;

type Res<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }

fn host_numbers() -> (usize, u64) {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    (cores, sys.total_memory() / 1_048_576)
}

#[tauri::command]
pub fn host_info(state: State<'_, runtime::AppState>) -> HostInfo {
    let (cores, ram_mb) = host_numbers();
    let arch = accel::host_arch();
    HostInfo {
        os: std::env::consts::OS.into(),
        arch,
        cores,
        ram_mb,
        accel: accel::accel_name().into(),
        qemu: accel::find_tool(&format!("qemu-system-{}", arch.qemu_suffix())).map(|p| p.to_string_lossy().into_owned()),
        swtpm: accel::find_tool("swtpm").is_some() && accel::host_os() != crate::qemu::HostOs::Windows,
        uefi: accel::find_firmware(arch).is_some(),
        drivers_iso: state.data_dir.join("drivers").join("virtio-win.iso").exists(),
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        default_base: store::default_base(&state.data_dir).to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub async fn list_vms(state: State<'_, runtime::AppState>) -> Res<Vec<VmSummary>> {
    // Polls QMP query-status so Pause/Resume matches the real QEMU window state.
    Ok(runtime::summaries(&state, store::list(&state.data_dir)).await)
}

#[tauri::command]
pub fn inspect_iso(path: String) -> Res<iso::IsoInfo> {
    let (cores, ram) = host_numbers();
    iso::inspect(Path::new(&path), cores, ram).map_err(err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFromIso {
    pub iso_path: String,
    pub name: String,
    pub cpu_cores: u32,
    pub ram_mb: u32,
    pub disk_gb: u32,
}

#[tauri::command]
pub async fn create_vm_from_iso(state: State<'_, runtime::AppState>, req: CreateFromIso) -> Res<VmConfig> {
    let (cores, ram) = host_numbers();
    let info = iso::inspect(Path::new(&req.iso_path), cores, ram).map_err(err)?;
    if !info.supported {
        return Err(info.warnings.first().cloned().unwrap_or_else(|| "This guest OS isn't supported.".into()));
    }
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let base = store::default_base(&state.data_dir);
    let drivers = state.data_dir.join("drivers").join("virtio-win.iso");
    let cfg = VmConfig {
        disk: DiskConfig { path: base.join(&id).join("disk.qcow2").to_string_lossy().into_owned(), format: DiskFormat::Qcow2, size_gb: req.disk_gb },
        id,
        name: req.name.trim().to_string(),
        os_family: info.os_family,
        os_name: info.os_name,
        os_version: info.os_version,
        arch: info.arch,
        base_folder: base.to_string_lossy().into_owned(),
        firmware: Firmware::Auto,
        tpm: info.needs_tpm,
        ram_mb: req.ram_mb,
        cpu_cores: req.cpu_cores,
        acceleration: Accel::Auto,
        vram_mb: info.recommended.vram_mb,
        gl_3d: false,
        width: 1920,
        height: 1080,
        iso_path: Some(info.path),
        drivers_iso: (info.os_family == OsFamily::Windows && drivers.exists()).then(|| drivers.to_string_lossy().into_owned()),
        storage_bus: StorageBus::Auto,
        network: NetMode::Nat,
        bridge_iface: String::new(),
        usb_devices: vec![],
        shared_folders: vec![],
    };
    store::save(&state.data_dir, &cfg).map_err(err)?;
    Ok(cfg)
}

#[tauri::command]
pub async fn save_vm(state: State<'_, runtime::AppState>, mut config: VmConfig) -> Res<()> {
    runtime::reap(&state);
    if state.running.lock().unwrap().contains_key(&config.id) {
        return Err("Stop the machine before changing its settings.".into());
    }
    // Until the disk exists, keep its path in sync with the machine folder and format.
    if !Path::new(&config.disk.path).exists() {
        let ext = config.disk.format.qemu();
        config.disk.path = PathBuf::from(&config.base_folder).join(&config.id).join(format!("disk.{ext}")).to_string_lossy().into_owned();
    }
    store::save(&state.data_dir, &config).map_err(err)
}

#[tauri::command]
pub async fn delete_vm(state: State<'_, runtime::AppState>, id: String, delete_files: bool) -> Res<()> {
    let cfg = store::load(&state.data_dir, &id).map_err(err)?;
    let _ = runtime::stop_vm(&state, &id, true).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    runtime::reap(&state);
    if delete_files {
        let _ = tokio::fs::remove_dir_all(store::vm_dir(&cfg)).await;
    }
    store::delete(&state.data_dir, &id).map_err(err)
}

#[tauri::command]
pub async fn start_vm(state: State<'_, runtime::AppState>, id: String) -> Res<Vec<String>> {
    let cfg = store::load(&state.data_dir, &id).map_err(err)?;
    runtime::start_vm(&state, &cfg).await.map_err(err)
}

#[tauri::command]
pub async fn stop_vm(state: State<'_, runtime::AppState>, id: String, force: bool) -> Res<()> {
    runtime::stop_vm(&state, &id, force).await.map_err(err)
}

#[tauri::command]
pub async fn pause_vm(state: State<'_, runtime::AppState>, id: String) -> Res<()> {
    runtime::pause_vm(&state, &id).await.map_err(err)
}

#[tauri::command]
pub async fn resume_vm(state: State<'_, runtime::AppState>, id: String) -> Res<()> {
    runtime::resume_vm(&state, &id).await.map_err(err)
}

#[tauri::command]
pub async fn create_disk(path: String, format: DiskFormat, size_gb: u32) -> Res<()> {
    runtime::create_disk(Path::new(&path), format, size_gb).await.map_err(err)
}
