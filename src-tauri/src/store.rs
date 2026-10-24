use crate::models::VmConfig;
use anyhow::Result;
use std::{fs, path::{Path, PathBuf}};

fn configs_dir(data: &Path) -> PathBuf { data.join("configs") }

pub fn default_base(data: &Path) -> PathBuf { data.join("machines") }

pub fn vm_dir(cfg: &VmConfig) -> PathBuf { Path::new(&cfg.base_folder).join(&cfg.id) }

pub fn list(data: &Path) -> Vec<VmConfig> {
    let mut out: Vec<VmConfig> = fs::read_dir(configs_dir(data))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str(&s).ok())
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn load(data: &Path, id: &str) -> Result<VmConfig> {
    let s = fs::read_to_string(configs_dir(data).join(format!("{id}.json")))?;
    Ok(serde_json::from_str(&s)?)
}

pub fn save(data: &Path, cfg: &VmConfig) -> Result<()> {
    fs::create_dir_all(configs_dir(data))?;
    let tmp = configs_dir(data).join(format!("{}.json.tmp", cfg.id));
    fs::write(&tmp, serde_json::to_string_pretty(cfg)?)?;
    fs::rename(tmp, configs_dir(data).join(format!("{}.json", cfg.id)))?;
    Ok(())
}

pub fn delete(data: &Path, id: &str) -> Result<()> {
    let _ = fs::remove_file(configs_dir(data).join(format!("{id}.json")));
    Ok(())
}
