pub mod benchmark;
pub mod smart;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub device: String,
    pub path: PathBuf,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MediaType {
    Hdd,
    Ssd,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAttribute {
    pub id: u8,
    pub name: String,
    pub value: u8,
    pub worst: u8,
    pub threshold: u8,
    pub raw: u64,
    pub status: SmartStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SmartStatus {
    Ok,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartData {
    pub disk: DiskInfo,
    pub overall_health: String,
    pub power_on_hours: u64,
    pub temperature: Option<i32>,
    pub reallocated_sectors: u64,
    pub pending_sectors: u64,
    pub uncorrectable_errors: u64,
    pub attributes: Vec<SmartAttribute>,
    pub smart_enabled: bool,
    pub smart_capable: bool,
    pub permission_error: bool,
    pub debug_status: String,
}

impl DiskInfo {
    pub fn new(device: String) -> Self {
        let path = PathBuf::from(format!("/dev/{}", device));
        let size = Self::get_size(&device);
        let (model, serial) = Self::get_model_serial(&device);
        let media_type = Self::detect_media_type(&device);

        Self {
            device,
            path,
            model,
            serial,
            size,
            media_type,
        }
    }

    fn get_size(device: &str) -> u64 {
        let size_path = format!("/sys/block/{}/size", device);
        std::fs::read_to_string(size_path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|sectors| sectors * 512)
            .unwrap_or(0)
    }

    fn get_model_serial(device: &str) -> (String, String) {
        let sys_path = format!("/sys/block/{}", device);
        let vendor_path = PathBuf::from(&sys_path).join("device/vendor");
        let model_path = PathBuf::from(&sys_path).join("device/model");
        let rev_path = PathBuf::from(&sys_path).join("device/rev");

        let vendor = std::fs::read_to_string(vendor_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let model = std::fs::read_to_string(model_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        let rev = std::fs::read_to_string(rev_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let model = if !vendor.is_empty() && !model.is_empty() {
            format!("{} {}", vendor.trim(), model)
        } else if !model.is_empty() {
            model
        } else {
            "Unknown".to_string()
        };

        let serial = if !rev.is_empty() {
            rev
        } else {
            "N/A".to_string()
        };

        (model, serial)
    }

    fn detect_media_type(device: &str) -> MediaType {
        let rotational_path = format!("/sys/block/{}/queue/rotational", device);
        if let Ok(rotational) = std::fs::read_to_string(&rotational_path) {
            if rotational.trim() == "0" {
                return MediaType::Ssd;
            } else if rotational.trim() == "1" {
                return MediaType::Hdd;
            }
        }
        MediaType::Unknown
    }
}

pub fn detect_disks() -> Vec<DiskInfo> {
    let mut disks = Vec::new();

    let block_dir = PathBuf::from("/sys/block");

    if let Ok(entries) = std::fs::read_dir(&block_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("sd") || name.starts_with("nvme") {
                let disk = DiskInfo::new(name);
                if disk.size > 0 {
                    disks.push(disk);
                }
            }
        }
    }

    disks
}
