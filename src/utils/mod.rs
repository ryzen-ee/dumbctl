use crate::disk::{DiskInfo, SmartData, benchmark::BenchmarkResult};
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

pub fn export_to_json(path: &PathBuf, disk_data: &Option<DiskInfo>, smart_data: &Option<SmartData>, benchmark: &Option<Vec<BenchmarkResult>>) -> Result<(), String> {
    let mut disks = Vec::new();
    
    if let (Some(disk), Some(smart)) = (disk_data, smart_data) {
        disks.push(DiskExport {
            device: disk.device.clone(),
            model: disk.model.clone(),
            serial: disk.serial.clone(),
            size: disk.size,
            media_type: format!("{:?}", disk.media_type),
            smart_data: Some(SmartExport {
                overall_health: smart.overall_health.clone(),
                power_on_hours: smart.power_on_hours,
                temperature: smart.temperature,
                reallocated_sectors: smart.reallocated_sectors,
                pending_sectors: smart.pending_sectors,
                uncorrectable_errors: smart.uncorrectable_errors,
                smart_enabled: smart.smart_enabled,
                smart_capable: smart.smart_capable,
                attributes: smart.attributes.iter().map(|a| AttrExport {
                    id: a.id,
                    name: a.name.clone(),
                    value: a.value,
                    worst: a.worst,
                    threshold: a.threshold,
                    raw: a.raw,
                }).collect(),
            }),
            benchmark: benchmark.as_ref().map(|b| BenchmarkExport { results: b.clone() }),
        });
    }

    let export_data = ExportData {
        export_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        disks,
    };

    let json = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

pub fn export_to_csv(path: &PathBuf, disk_data: &Option<DiskInfo>, smart_data: &Option<SmartData>, benchmark: &Option<Vec<BenchmarkResult>>) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| format!("Failed to create CSV: {}", e))?;

    wtr.write_record(&["Device", "Model", "Serial", "Size (bytes)", "Media Type", "SMART Health", "Power On Hours", "Temperature", "Reallocated Sectors", "Pending Sectors", "Uncorrectable"])
        .map_err(|e| format!("CSV write error: {}", e))?;

    if let (Some(disk), Some(smart)) = (disk_data, smart_data) {
        wtr.write_record(&[
            &disk.device,
            &disk.model,
            &disk.serial,
            &disk.size.to_string(),
            &format!("{:?}", disk.media_type),
            &smart.overall_health,
            &smart.power_on_hours.to_string(),
            &smart.temperature.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string()),
            &smart.reallocated_sectors.to_string(),
            &smart.pending_sectors.to_string(),
            &smart.uncorrectable_errors.to_string(),
        ]).map_err(|e| format!("CSV write error: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("CSV flush error: {}", e))?;
    
    if let Some(bench_results) = benchmark {
        let bench_path = path.with_extension("csv");
        let bench_csv = File::create(&bench_path)
            .map_err(|e| format!("Failed to create benchmark CSV: {}", e))?;
        
        let mut bench_wtr = csv::Writer::from_writer(bench_csv);
        
        bench_wtr.write_record(&["Block Size (KB)", "Read Speed (MB/s)", "Write Speed (MB/s)"])
            .map_err(|e| format!("CSV write error: {}", e))?;
        
        for result in bench_results {
            bench_wtr.write_record(&[
                &result.block_size_kb.to_string(),
                &format!("{:.2}", result.read_speed_mbps),
                &format!("{:.2}", result.write_speed_mbps),
            ]).map_err(|e| format!("CSV write error: {}", e))?;
        }
        
        bench_wtr.flush().map_err(|e| format!("CSV flush error: {}", e))?;
    }

    Ok(())
}
