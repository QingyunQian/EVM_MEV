//! Minimal CSV and JSON writers for experiment output.

use std::path::Path;

use anyhow::Result;

pub fn write_csv<T: serde::Serialize>(rows: &[T], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut w = csv::Writer::from_path(path)?;
    for r in rows {
        w.serialize(r)?;
    }
    w.flush()?;
    Ok(())
}

#[allow(dead_code)]
pub fn write_json<T: serde::Serialize>(value: &T, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let f = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(f, value)?;
    Ok(())
}
