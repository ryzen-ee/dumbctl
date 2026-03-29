use super::{DiskInfo, SmartAttribute, SmartData, SmartStatus};
use std::path::PathBuf;
use std::process::Command;

pub fn get_smart_data(disk: &DiskInfo) -> SmartData {
    let smartctl_path = "/usr/sbin/smartctl";

    if !std::path::Path::new(smartctl_path).exists() {
        return SmartData {
            disk: disk.clone(),
            overall_health: "Not Installed".to_string(),
            power_on_hours: 0,
            temperature: None,
            reallocated_sectors: 0,
            pending_sectors: 0,
            uncorrectable_errors: 0,
            attributes: vec![],
            smart_enabled: false,
            smart_capable: false,
            permission_error: false,
            debug_status: "smartctl not found".to_string(),
        };
    }

    let output = Command::new(smartctl_path)
        .args(["-a", "-j", &disk.path.to_string_lossy()])
        .output();

    let output_str = output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let stderr_str = output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
        .unwrap_or_default();

    let permission_denied =
        output_str.contains("Permission denied") || stderr_str.contains("Permission denied");

    if permission_denied {
        return SmartData {
            disk: disk.clone(),
            overall_health: "Need Root".to_string(),
            power_on_hours: 0,
            temperature: None,
            reallocated_sectors: 0,
            pending_sectors: 0,
            uncorrectable_errors: 0,
            attributes: vec![],
            smart_enabled: false,
            smart_capable: false,
            permission_error: true,
            debug_status: "perm_denied".to_string(),
        };
    }

    if output_str.is_empty() || !output_str.starts_with("{") {
        return SmartData {
            disk: disk.clone(),
            overall_health: "Unknown".to_string(),
            power_on_hours: 0,
            temperature: None,
            reallocated_sectors: 0,
            pending_sectors: 0,
            uncorrectable_errors: 0,
            attributes: vec![],
            smart_enabled: false,
            smart_capable: false,
            permission_error: false,
            debug_status: "no_json".to_string(),
        };
    }

    let (
        overall_health,
        smart_enabled,
        smart_capable,
        attrs,
        power_on_hours,
        temperature,
        reallocated,
        pending,
        uncorrectable,
    ) = parse_smartctl_json(&output_str);

    SmartData {
        disk: disk.clone(),
        overall_health,
        power_on_hours,
        temperature,
        reallocated_sectors: reallocated,
        pending_sectors: pending,
        uncorrectable_errors: uncorrectable,
        attributes: attrs,
        smart_enabled,
        smart_capable,
        permission_error: false,
        debug_status: "ok".to_string(),
    }
}

fn parse_smartctl_json(
    output: &str,
) -> (
    String,
    bool,
    bool,
    Vec<SmartAttribute>,
    u64,
    Option<i32>,
    u64,
    u64,
    u64,
) {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        let smart_support = json
            .get("smart_support")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let smart_capable = json
            .get("smart_support")
            .and_then(|v| v.get("available"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let health = json
            .get("smart_status")
            .and_then(|v| v.get("passed"))
            .and_then(|v| v.as_bool())
            .map(|p| {
                if p {
                    "PASSED".to_string()
                } else {
                    "FAILED".to_string()
                }
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let power_on_hours = json
            .get("power_on_time")
            .and_then(|v| v.get("hours"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let temperature = json
            .get("temperature")
            .and_then(|v| v.get("current"))
            .and_then(|v| v.as_i64())
            .map(|t| t as i32);

        let reallocated = json
            .get("attributes")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|a| a.get("id").and_then(|v| v.as_u64()) == Some(5))
            })
            .and_then(|a| a.get("raw").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let pending = json
            .get("attributes")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|a| a.get("id").and_then(|v| v.as_u64()) == Some(197))
            })
            .and_then(|a| a.get("raw").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let uncorrectable = json
            .get("attributes")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|a| a.get("id").and_then(|v| v.as_u64()) == Some(198))
            })
            .and_then(|a| a.get("raw").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let attrs: Vec<SmartAttribute> = json
            .get("attributes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let id = a.get("id")?.as_u64()? as u8;
                        let name = a.get("name")?.as_str()?.to_string();
                        let value = a.get("value")?.as_u64()? as u8;
                        let worst = a.get("worst")?.as_u64()? as u8;
                        let threshold = a.get("thresh")?.as_u64()? as u8;
                        let raw = a.get("raw")?.as_str()?.parse::<u64>().unwrap_or(0);
                        let status = determine_status(id, value, raw);

                        Some(SmartAttribute {
                            id,
                            name,
                            value,
                            worst,
                            threshold,
                            raw,
                            status,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        (
            health,
            smart_support,
            smart_capable,
            attrs,
            power_on_hours,
            temperature,
            reallocated,
            pending,
            uncorrectable,
        )
    } else {
        fallback_smart_data("")
    }
}

fn fallback_smart_data(
    device: &str,
) -> (
    String,
    bool,
    bool,
    Vec<SmartAttribute>,
    u64,
    Option<i32>,
    u64,
    u64,
    u64,
) {
    let (power_on_hours, temperature, reallocated, pending, uncorrectable) =
        get_sysfs_smart(device);
    let attrs = get_smart_attributes_from_sysfs(device);
    (
        "Unknown".to_string(),
        true,
        true,
        attrs,
        power_on_hours,
        temperature,
        reallocated,
        pending,
        uncorrectable,
    )
}

fn get_sysfs_smart(device: &str) -> (u64, Option<i32>, u64, u64, u64) {
    let base = format!("/sys/block/{}", device);

    let power_on_hours = std::fs::read_to_string(format!("{}/device/power_on_time", base))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|ms| ms / 3600)
        .unwrap_or(0);

    let temperature = std::fs::read_to_string(format!("{}/device/temp", base))
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok());

    let reallocated = std::fs::read_to_string(format!("{}/device/reallocated_sectors_count", base))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);

    let pending = std::fs::read_to_string(format!("{}/device/current_pending_sector", base))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);

    let uncorrectable =
        std::fs::read_to_string(format!("{}/device/uncorrectable_sectors_count", base))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

    (
        power_on_hours,
        temperature,
        reallocated,
        pending,
        uncorrectable,
    )
}

fn get_smart_attributes_from_sysfs(device: &str) -> Vec<SmartAttribute> {
    let base = format!("/sys/block/{}", device);
    let attrs_dir = PathBuf::from(&base).join("device/smart_attributes");

    let mut attrs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&attrs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "revision" || name == "格式" {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let parts: Vec<&str> = content.trim().split_whitespace().collect();
                if parts.len() >= 7 {
                    if let (Ok(id), Ok(value), Ok(worst), Ok(threshold), Ok(raw)) = (
                        parts[0].parse::<u8>(),
                        parts[1].parse::<u8>(),
                        parts[2].parse::<u8>(),
                        parts[3].parse::<u8>(),
                        parts[6].parse::<u64>(),
                    ) {
                        let attr_name = get_attr_name(id);
                        let status = determine_status(id, value, raw);

                        attrs.push(SmartAttribute {
                            id,
                            name: attr_name,
                            value,
                            worst,
                            threshold,
                            raw,
                            status,
                        });
                    }
                }
            }
        }
    }

    attrs.sort_by_key(|a| a.id);
    attrs
}

fn get_attr_name(id: u8) -> String {
    match id {
        1 => "Raw Read Error Rate".to_string(),
        2 => "Throughput Performance".to_string(),
        3 => "Spin-Up Time".to_string(),
        4 => "Start/Stop Count".to_string(),
        5 => "Reallocated Sectors Count".to_string(),
        7 => "Seek Error Rate".to_string(),
        8 => "Seek Time Performance".to_string(),
        9 => "Power-On Hours".to_string(),
        10 => "Spin Retry Count".to_string(),
        11 => "Calibration Retry Count".to_string(),
        12 => "Power Cycle Count".to_string(),
        170 => "Available Reserved Space".to_string(),
        171 => "SSD Program Fail Count".to_string(),
        172 => "SSD Erase Fail Count".to_string(),
        173 => "SSD Wear Leveling Count".to_string(),
        174 => "Unexpected Power Loss Count".to_string(),
        175 => "Power Loss Protection Failure".to_string(),
        176 => "Erase Fail Count".to_string(),
        177 => "Wear Range Delta".to_string(),
        178 => "Used Reserved Block Count".to_string(),
        179 => "Used Reserved Block Count Total".to_string(),
        180 => "Unused Reserved Block Count Total".to_string(),
        181 => "Program Fail Count Total".to_string(),
        182 => "Erase Fail Count".to_string(),
        183 => "SATA Downshift Error Count".to_string(),
        184 => "End-to-End Error".to_string(),
        185 => "Head Stability".to_string(),
        186 => "Induced Op-Vibration Detection".to_string(),
        187 => "Reported Uncorrectable Errors".to_string(),
        188 => "Command Timeout".to_string(),
        189 => "High Fly Writes".to_string(),
        190 => "Temperature".to_string(),
        191 => "G-Sense Error Rate".to_string(),
        192 => "Power-Off Retract Count".to_string(),
        193 => "Load Cycle Count".to_string(),
        194 => "Temperature Celsius".to_string(),
        195 => "Hardware ECC Recovered".to_string(),
        196 => "Reallocation Event Count".to_string(),
        197 => "Current Pending Sector Count".to_string(),
        198 => "Uncorrectable Sector Count".to_string(),
        199 => "UltraDMA CRC Error Count".to_string(),
        200 => "Multi-Zone Error Rate".to_string(),
        201 => "Soft Read Error Rate".to_string(),
        202 => "Data Address Mark Errors".to_string(),
        203 => "Run Out Cancel".to_string(),
        204 => "Soft ECC Correction".to_string(),
        205 => "Thermal Asperity Rate".to_string(),
        206 => "Flying Height".to_string(),
        207 => "Spin High Current".to_string(),
        208 => "Spin Buzz".to_string(),
        209 => "Offline Seek Performance".to_string(),
        220 => "Disk Shift".to_string(),
        221 => "G-Sense Error Rate".to_string(),
        222 => "Loaded Hours".to_string(),
        223 => "Load/Unload Retry Count".to_string(),
        224 => "Load Friction".to_string(),
        225 => "Load/Unload Cycle Count".to_string(),
        226 => "Load In-Time".to_string(),
        227 => "Torque Amplification Count".to_string(),
        228 => "Power-Off Retract Cycle".to_string(),
        230 => "GMR Head Amplitude".to_string(),
        231 => "Temperature".to_string(),
        232 => "Available Reserved Space".to_string(),
        233 => "Media Wearout Indicator".to_string(),
        234 => "Average Erase Count".to_string(),
        235 => "Good Block Count".to_string(),
        240 => "Head Flying Hours".to_string(),
        241 => "Total LBAs Written".to_string(),
        242 => "Total LBAs Read".to_string(),
        243 => "Total LBAs Written Expanded".to_string(),
        244 => "Total LBAs Read Expanded".to_string(),
        249 => "NAND Writes 1GiB".to_string(),
        250 => "Read Error Retry Rate".to_string(),
        251 => "Minimum Spares Remaining".to_string(),
        252 => "Newly Added Bad Flash Block".to_string(),
        254 => "Free Fall Protection".to_string(),
        _ => format!("Attribute {}", id),
    }
}

fn determine_status(id: u8, value: u8, raw: u64) -> SmartStatus {
    match id {
        5 if raw > 0 => SmartStatus::Critical,
        197 if raw > 0 => SmartStatus::Warning,
        198 if raw > 0 => SmartStatus::Critical,
        196 if raw > 0 => SmartStatus::Warning,
        199 if raw > 10 => SmartStatus::Warning,
        200 if raw > 10 => SmartStatus::Warning,
        _ if value < 100 && value > 0 => SmartStatus::Warning,
        _ if value <= 0 => SmartStatus::Critical,
        _ => SmartStatus::Ok,
    }
}
