use crate::disk::{benchmark::BenchmarkResult, DiskInfo, SmartData};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct ExportData {
    pub export_time: String,
    pub disks: Vec<DiskExport>,
}

#[derive(Serialize)]
pub struct DiskExport {
    pub device: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub media_type: String,
    pub smart_data: Option<SmartExport>,
    pub benchmark: Option<BenchmarkExport>,
}

#[derive(Serialize)]
pub struct SmartExport {
    pub overall_health: String,
    pub power_on_hours: u64,
    pub temperature: Option<i32>,
    pub reallocated_sectors: u64,
    pub pending_sectors: u64,
    pub uncorrectable_errors: u64,
    pub smart_enabled: bool,
    pub smart_capable: bool,
    pub attributes: Vec<AttrExport>,
}

#[derive(Serialize)]
pub struct AttrExport {
    pub id: u8,
    pub name: String,
    pub value: u8,
    pub worst: u8,
    pub threshold: u8,
    pub raw: u64,
}

#[derive(Serialize)]
pub struct BenchmarkExport {
    pub results: Vec<BenchmarkResult>,
}

pub fn export_to_json(
    path: &PathBuf,
    disk_data: &Option<DiskInfo>,
    smart_data: &Option<SmartData>,
    benchmark: &Option<Vec<BenchmarkResult>>,
) -> Result<(), String> {
    let mut disks = Vec::new();

    if let Some(disk) = disk_data {
        disks.push(DiskExport {
            device: disk.device.clone(),
            model: disk.model.clone(),
            serial: disk.serial.clone(),
            size: disk.size,
            media_type: format!("{:?}", disk.media_type),
            smart_data: smart_data.clone().map(|smart| SmartExport {
                overall_health: smart.overall_health.clone(),
                power_on_hours: smart.power_on_hours,
                temperature: smart.temperature,
                reallocated_sectors: smart.reallocated_sectors,
                pending_sectors: smart.pending_sectors,
                uncorrectable_errors: smart.uncorrectable_errors,
                smart_enabled: smart.smart_enabled,
                smart_capable: smart.smart_capable,
                attributes: smart
                    .attributes
                    .iter()
                    .map(|a| AttrExport {
                        id: a.id,
                        name: a.name.clone(),
                        value: a.value,
                        worst: a.worst,
                        threshold: a.threshold,
                        raw: a.raw,
                    })
                    .collect(),
            }),
            benchmark: benchmark
                .as_ref()
                .map(|b| BenchmarkExport { results: b.clone() }),
        });
    }

    let export_data = ExportData {
        export_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        disks,
    };

    let json = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o644);
            let _ = std::fs::set_permissions(path, perms);
        }
    }

    Ok(())
}

pub fn export_to_csv(
    path: &PathBuf,
    disk_data: &Option<DiskInfo>,
    smart_data: &Option<SmartData>,
    benchmark: &Option<Vec<BenchmarkResult>>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let mut wtr =
        csv::Writer::from_path(path).map_err(|e| format!("Failed to create CSV: {}", e))?;

    wtr.write_record(&[
        "Device",
        "Model",
        "Serial",
        "Size (bytes)",
        "Media Type",
        "SMART Health",
        "Power On Hours",
        "Temperature",
        "Reallocated Sectors",
        "Pending Sectors",
        "Uncorrectable",
    ])
    .map_err(|e| format!("CSV write error: {}", e))?;

    if let Some(disk) = disk_data {
        let health = smart_data
            .as_ref()
            .map(|s| s.overall_health.clone())
            .unwrap_or_else(|| "N/A".to_string());
        let power_on = smart_data
            .as_ref()
            .map(|s| s.power_on_hours.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let temp = smart_data
            .as_ref()
            .and_then(|s| s.temperature)
            .map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let realloc = smart_data
            .as_ref()
            .map(|s| s.reallocated_sectors.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let pending = smart_data
            .as_ref()
            .map(|s| s.pending_sectors.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let uncorrect = smart_data
            .as_ref()
            .map(|s| s.uncorrectable_errors.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        wtr.write_record(&[
            &disk.device,
            &disk.model,
            &disk.serial,
            &disk.size.to_string(),
            &format!("{:?}", disk.media_type),
            &health,
            &power_on,
            &temp,
            &realloc,
            &pending,
            &uncorrect,
        ])
        .map_err(|e| format!("CSV write error: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("CSV flush error: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o644);
            let _ = std::fs::set_permissions(path, perms);
        }
    }

    if let Some(bench_results) = benchmark {
        let bench_path = path.with_extension("csv");
        let bench_csv = File::create(&bench_path)
            .map_err(|e| format!("Failed to create benchmark CSV: {}", e))?;

        let mut bench_wtr = csv::Writer::from_writer(bench_csv);

        bench_wtr
            .write_record(&["Block Size (KB)", "Read Speed (MB/s)", "Write Speed (MB/s)"])
            .map_err(|e| format!("CSV write error: {}", e))?;

        for result in bench_results {
            bench_wtr
                .write_record(&[
                    &result.block_size_kb.to_string(),
                    &format!("{:.2}", result.read_speed_mbps),
                    &format!("{:.2}", result.write_speed_mbps),
                ])
                .map_err(|e| format!("CSV write error: {}", e))?;
        }

        bench_wtr
            .flush()
            .map_err(|e| format!("CSV flush error: {}", e))?;
    }

    Ok(())
}

pub fn export_to_html(
    path: &PathBuf,
    disk_data: &Option<DiskInfo>,
    smart_data: &Option<SmartData>,
    benchmark: &Option<Vec<BenchmarkResult>>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let export_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut html = String::new();
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Disk Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }
        h1 { color: #333; }
        .card { background: white; border-radius: 8px; padding: 20px; margin: 20px 0; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .info-grid { display: grid; grid-template-columns: 150px 1fr; gap: 10px; }
        .info-label { font-weight: bold; color: #666; }
        .healthy { color: #22c55e; }
        .warning { color: #f59e0b; }
        .critical { color: #ef4444; }
        table { width: 100%; border-collapse: collapse; margin-top: 10px; }
        th, td { padding: 8px 12px; text-align: left; border-bottom: 1px solid #eee; }
        th { background: #f9f9f9; font-weight: 600; }
        .benchmark-read { color: #3b82f6; }
        .benchmark-write { color: #8b5cf6; }
    </style>
</head>
<body>
    <h1>Disk Report</h1>
    <p>Generated: "#);
    html.push_str(&export_time);
    html.push_str("</p>\n");

    if let Some(disk) = disk_data {
        html.push_str("    <div class=\"card\">\n        <h2>Disk Information</h2>\n        <div class=\"info-grid\">\n");

        let size_gb = disk.size / (1024 * 1024 * 1024);
        let media_type = format!("{:?}", disk.media_type);

        html.push_str(&format!(
            "                <span class=\"info-label\">Device:</span><span>{}</span>\n",
            disk.device
        ));
        html.push_str(&format!(
            "                <span class=\"info-label\">Model:</span><span>{}</span>\n",
            disk.model.trim()
        ));
        html.push_str(&format!(
            "                <span class=\"info-label\">Serial:</span><span>{}</span>\n",
            disk.serial
        ));
        html.push_str(&format!(
            "                <span class=\"info-label\">Size:</span><span>{} GB</span>\n",
            size_gb
        ));
        html.push_str(&format!(
            "                <span class=\"info-label\">Type:</span><span>{}</span>\n",
            media_type
        ));

        html.push_str("        </div>\n    </div>\n");

        if let Some(smart) = smart_data {
            let health_class = match smart.overall_health.as_str() {
                "PASSED" | "OK" | "Healthy" => "healthy",
                _ => "critical",
            };

            html.push_str("    <div class=\"card\">\n        <h2>SMART Status</h2>\n        <div class=\"info-grid\">\n");
            html.push_str(&format!("                <span class=\"info-label\">Health:</span><span class=\"{}\">{}</span>\n", health_class, smart.overall_health));

            if let Some(temp) = smart.temperature {
                html.push_str(&format!("                <span class=\"info-label\">Temperature:</span><span>{}°C</span>\n", temp));
            }

            html.push_str(&format!("                <span class=\"info-label\">Power On:</span><span>{} hours</span>\n", smart.power_on_hours));
            html.push_str(&format!("                <span class=\"info-label\">Reallocated:</span><span>{} sectors</span>\n", smart.reallocated_sectors));
            html.push_str(&format!("                <span class=\"info-label\">Pending:</span><span>{} sectors</span>\n", smart.pending_sectors));
            html.push_str(&format!("                <span class=\"info-label\">Uncorrectable:</span><span>{} errors</span>\n", smart.uncorrectable_errors));

            html.push_str("        </div>\n    </div>\n");

            if !smart.attributes.is_empty() {
                html.push_str("    <div class=\"card\">\n        <h2>SMART Attributes</h2>\n        <table>\n            <tr><th>ID</th><th>Name</th><th>Value</th><th>Worst</th><th>Threshold</th><th>Raw</th></tr>\n");
                for attr in &smart.attributes {
                    html.push_str(&format!(
                        "            <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                        attr.id, attr.name, attr.value, attr.worst, attr.threshold, attr.raw
                    ));
                }
                html.push_str("        </table>\n    </div>\n");
            }
        }

        if let Some(results) = benchmark {
            html.push_str("    <div class=\"card\">\n        <h2>Benchmark Results</h2>\n        <table>\n            <tr><th>Block Size (KB)</th><th>Read (MB/s)</th><th>Write (MB/s)</th></tr>\n");
            for result in results {
                html.push_str(&format!(
                    "            <tr><td>{}</td><td class=\"benchmark-read\">{:.2}</td><td class=\"benchmark-write\">{:.2}</td></tr>\n",
                    result.block_size_kb, result.read_speed_mbps, result.write_speed_mbps
                ));
            }
            html.push_str("        </table>\n    </div>\n");
        }
    }

    html.push_str("</body>\n</html>\n");

    std::fs::write(path, html).map_err(|e| format!("Failed to write HTML file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map(|m| m.permissions())
            .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o644));
        perms.set_mode(0o644);
        std::fs::set_permissions(path, perms).ok();
    }

    Ok(())
}
