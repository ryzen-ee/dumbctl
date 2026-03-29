use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult {
    pub read_speed_mbps: f64,
    pub write_speed_mbps: f64,
    pub block_size_kb: u32,
    pub duration_ms: u64,
}

#[derive(Clone)]
pub struct BenchmarkProgress {
    pub current_block: u32,
    pub total_blocks: u32,
    pub phase: String,
    pub percent: u32,
}

pub struct Benchmark {
    pub device: String,
    pub block_size_kb: u32,
    pub test_size_mb: u32,
    pub results: Vec<BenchmarkResult>,
    pub progress: Arc<AtomicU32>,
    pub current_phase: Arc<std::sync::Mutex<String>>,
}

impl Benchmark {
    pub fn new(device: String) -> Self {
        Self {
            device,
            block_size_kb: 1024,
            test_size_mb: 256,
            results: Vec::new(),
            progress: Arc::new(AtomicU32::new(0)),
            current_phase: Arc::new(std::sync::Mutex::new(String::new())),
        }
    }

    pub fn run(&mut self) -> Vec<BenchmarkResult> {
        self.results.clear();

        let block_sizes = vec![4, 64, 1024];
        let total = block_sizes.len() as u32 * 2;

        for (i, &block_kb) in block_sizes.iter().enumerate() {
            self.block_size_kb = block_kb;

            {
                let mut phase = self.current_phase.lock().unwrap();
                *phase = format!("Reading {} KB blocks...", block_kb);
            }
            self.progress.store((i * 2 + 1) as u32, Ordering::SeqCst);
            let read_result = self.run_read_test();

            {
                let mut phase = self.current_phase.lock().unwrap();
                *phase = format!("Writing {} KB blocks...", block_kb);
            }
            self.progress.store((i * 2 + 2) as u32, Ordering::SeqCst);
            let write_result = self.run_write_test();

            self.results.push(BenchmarkResult {
                read_speed_mbps: read_result,
                write_speed_mbps: write_result,
                block_size_kb: block_kb,
                duration_ms: 0,
            });
        }

        self.progress.store(total, Ordering::SeqCst);
        self.results.clone()
    }

    pub fn get_progress(&self) -> BenchmarkProgress {
        let current = self.progress.load(Ordering::SeqCst);
        let phase = self.current_phase.lock().unwrap().clone();
        let percent = ((current as f64 / 6.0) * 100.0) as u32;
        BenchmarkProgress {
            current_block: current,
            total_blocks: 6,
            phase,
            percent,
        }
    }

    fn run_read_test(&self) -> f64 {
        let temp_file = self.get_temp_path();

        if let Err(e) = self.write_test_file(&temp_file) {
            eprintln!("Warning: Could not create test file: {}", e);
            return 0.0;
        }

        let start = Instant::now();
        let bytes_read = self.read_file(&temp_file);
        let elapsed = start.elapsed();

        let _ = std::fs::remove_file(&temp_file);

        if elapsed.as_secs_f64() > 0.0 {
            (bytes_read as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        }
    }

    fn run_write_test(&self) -> f64 {
        let temp_file = self.get_temp_path();

        let start = Instant::now();
        let bytes_written = self.write_file(&temp_file);
        let elapsed = start.elapsed();

        let _ = std::fs::remove_file(&temp_file);

        if elapsed.as_secs_f64() > 0.0 {
            (bytes_written as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        }
    }

    fn get_temp_path(&self) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(format!(".dumbctl_test_{}", self.device.replace("/", "_")))
    }

    fn write_test_file(&self, path: &PathBuf) -> std::io::Result<u64> {
        let file = File::create(path)?;
        let buffer = vec![0u8; (self.block_size_kb * 1024) as usize];
        let mut file = file;
        let mut written = 0u64;

        let iterations = (self.test_size_mb * 1024) / self.block_size_kb;

        for _ in 0..iterations {
            file.write_all(&buffer)?;
            written += buffer.len() as u64;
        }

        file.sync_all()?;
        Ok(written)
    }

    fn read_file(&self, path: &PathBuf) -> u64 {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return 0,
        };

        let mut buffer = vec![0u8; (self.block_size_kb * 1024) as usize];
        let mut total_read = 0u64;

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => total_read += n as u64,
                Err(_) => break,
            }
        }

        total_read
    }

    fn write_file(&self, path: &PathBuf) -> u64 {
        let file = match File::create(path) {
            Ok(f) => f,
            Err(_) => return 0,
        };

        let buffer = vec![0u8; (self.block_size_kb * 1024) as usize];
        let mut file = file;
        let mut written = 0u64;

        let iterations = (self.test_size_mb * 1024) / self.block_size_kb;

        for _ in 0..iterations {
            if file.write_all(&buffer).is_err() {
                break;
            }
            written += buffer.len() as u64;
        }

        let _ = file.sync_all();
        written
    }
}
