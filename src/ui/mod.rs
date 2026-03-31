pub mod components;
pub mod tabs;

use crate::database;
use crate::disk::benchmark::{Benchmark, BenchmarkProgress, BenchmarkResult};
use crate::disk::{self, DiskInfo, SmartData};
use crate::settings::Settings;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Disks,
    Smart,
    Sectors,
    Benchmark,
    Export,
    Settings,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Tab::Disks => Tab::Smart,
            Tab::Smart => Tab::Sectors,
            Tab::Sectors => Tab::Benchmark,
            Tab::Benchmark => Tab::Export,
            Tab::Export => Tab::Settings,
            Tab::Settings => Tab::Disks,
        }
    }

    fn prev(self) -> Self {
        match self {
            Tab::Disks => Tab::Settings,
            Tab::Smart => Tab::Disks,
            Tab::Sectors => Tab::Smart,
            Tab::Benchmark => Tab::Sectors,
            Tab::Export => Tab::Benchmark,
            Tab::Settings => Tab::Export,
        }
    }

    #[allow(dead_code)]
    fn title(self) -> &'static str {
        match self {
            Tab::Disks => "Disk List",
            Tab::Smart => "SMART Details",
            Tab::Sectors => "Sectors",
            Tab::Benchmark => "Benchmark",
            Tab::Export => "Export",
            Tab::Settings => "Settings",
        }
    }
}

pub struct App {
    pub disks: Vec<DiskInfo>,
    pub selected_disk_index: Option<usize>,
    pub smart_data: Option<SmartData>,
    pub benchmark_results: Option<Vec<BenchmarkResult>>,
    pub current_tab: Tab,
    pub disk_list_state: ListState,
    pub export_format: ExportFormat,
    pub export_content: ExportContent,
    #[allow(dead_code)]
    pub export_path: String,
    pub export_status: Option<String>,
    pub export_status_timestamp: Option<Instant>,
    pub export_running: bool,
    pub benchmark_running: bool,
    pub benchmark_progress: Option<BenchmarkProgress>,
    pub message: Option<String>,
    pub message_timestamp: Option<Instant>,
    pub benchmark_results_shared: Option<Arc<Mutex<Option<Vec<BenchmarkResult>>>>>,
    pub benchmark_progress_shared: Option<Arc<AtomicU32>>,
    pub benchmark_phase_shared: Option<Arc<std::sync::Mutex<String>>>,
    pub settings: Settings,
    pub settings_edit_field: SettingsField,
    pub settings_editing: bool,
    pub settings_input_buffer: String,
    pub search_query: String,
    pub search_active: bool,
    pub database: database::Database,
    pub benchmark_history: Vec<database::BenchmarkHistoryEntry>,
    pub benchmark_history_selected: Option<usize>,
    pub benchmark_history_expanded: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    None,
    ExportPath,
    Theme,
    AutoRefresh,
    BenchmarkSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Html,
    Pdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportContent {
    SmartOnly,
    BenchmarkOnly,
    Both,
}

impl ExportContent {
    #[allow(dead_code)]
    pub fn next(self) -> Self {
        match self {
            ExportContent::SmartOnly => ExportContent::BenchmarkOnly,
            ExportContent::BenchmarkOnly => ExportContent::Both,
            ExportContent::Both => ExportContent::SmartOnly,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ExportContent::SmartOnly => "SMART only",
            ExportContent::BenchmarkOnly => "Benchmark only",
            ExportContent::Both => "SMART + Benchmark",
        }
    }
}

pub struct ListState {
    #[allow(dead_code)]
    pub offset: usize,
    pub selected: usize,
}

impl ListState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            selected: 0,
        }
    }

    pub fn select(&mut self, index: usize, total: usize) {
        if total == 0 {
            self.selected = 0;
            return;
        }
        self.selected = index.min(total - 1);
    }
}

impl Default for ListState {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let disks = disk::detect_disks();
        let settings = Settings::load();
        let database = database::Database::new().unwrap_or_else(|_| database::Database::default());

        let mut app = Self {
            disks,
            selected_disk_index: None,
            smart_data: None,
            benchmark_results: None,
            current_tab: Tab::Disks,
            disk_list_state: ListState::new(),
            export_format: ExportFormat::Json,
            export_content: ExportContent::Both,
            export_path: String::new(),
            export_status: None,
            export_status_timestamp: None,
            export_running: false,
            benchmark_running: false,
            benchmark_progress: None,
            message: None,
            message_timestamp: None,
            benchmark_results_shared: None,
            benchmark_progress_shared: None,
            benchmark_phase_shared: None,
            settings,
            settings_edit_field: SettingsField::None,
            settings_editing: false,
            settings_input_buffer: String::new(),
            search_query: String::new(),
            search_active: false,
            database,
            benchmark_history: Vec::new(),
            benchmark_history_selected: None,
            benchmark_history_expanded: Vec::new(),
        };

        if app.disks.len() > 1 {
            app.selected_disk_index = Some(0);
            app.disk_list_state.select(0, app.disks.len());
        }

        if let Some(idx) = app.selected_disk_index {
            if let Some(disk) = app.disks.get(idx) {
                if let Ok(history) = app.database.get_history(&disk.device, &disk.serial, 50) {
                    app.benchmark_history = history;
                }
            }
        }

        app
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<impl ratatui::backend::Backend>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            self.check_benchmark_completion();
            self.check_message_expiry();
            self.check_export_status_expiry();

            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::PageUp | KeyCode::PageDown => continue,
                                _ => {}
                            }
                        }

                        if key.kind == KeyEventKind::Press {
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                match key.code {
                                    KeyCode::Char('s') => {
                                        if self.current_tab == Tab::Settings {
                                            self.save_settings();
                                        }
                                    }
                                    KeyCode::Char('r') => {
                                        if self.current_tab == Tab::Settings {
                                            self.reset_settings();
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                let settings_editing =
                                    self.current_tab == Tab::Settings && self.settings_editing;

                                match key.code {
                                    KeyCode::Char('q') => return Ok(()),
                                    KeyCode::Esc => {
                                        if self.current_tab == Tab::Settings
                                            && self.settings_editing
                                        {
                                            self.settings_editing = false;
                                            self.settings_input_buffer.clear();
                                        } else if self.current_tab == Tab::Settings {
                                            self.settings_edit_field = SettingsField::None;
                                        } else {
                                            return Ok(());
                                        }
                                    }
                                    KeyCode::Right if !settings_editing => {
                                        self.current_tab = self.current_tab.next();
                                        if self.current_tab == Tab::Benchmark {
                                            self.load_benchmark_history();
                                        }
                                    }
                                    KeyCode::Left if !settings_editing => {
                                        self.current_tab = self.current_tab.prev();
                                        if self.current_tab == Tab::Benchmark {
                                            self.load_benchmark_history();
                                        }
                                    }
                                    KeyCode::Char('/')
                                        if self.current_tab == Tab::Disks && !settings_editing =>
                                    {
                                        self.search_active = !self.search_active;
                                        if !self.search_active {
                                            self.search_query.clear();
                                        }
                                    }
                                    KeyCode::Char('1')
                                        if self.current_tab == Tab::Disks && !settings_editing =>
                                    {
                                        crate::ui::tabs::disks::toggle_sort(0);
                                    }
                                    KeyCode::Char('2')
                                        if self.current_tab == Tab::Disks && !settings_editing =>
                                    {
                                        crate::ui::tabs::disks::toggle_sort(1);
                                    }
                                    KeyCode::Char('3')
                                        if self.current_tab == Tab::Disks && !settings_editing =>
                                    {
                                        crate::ui::tabs::disks::toggle_sort(2);
                                    }
                                    KeyCode::Char('4')
                                        if self.current_tab == Tab::Disks && !settings_editing =>
                                    {
                                        crate::ui::tabs::disks::toggle_sort(3);
                                    }
                                    KeyCode::Char('r') if !settings_editing => self.refresh(),
                                    KeyCode::Char('e') if !settings_editing => {
                                        self.current_tab = Tab::Export
                                    }
                                    KeyCode::Char('s') if self.current_tab == Tab::Benchmark => {
                                        self.start_benchmark();
                                    }
                                    KeyCode::Char('c') if self.current_tab == Tab::Benchmark => {
                                        if !self.benchmark_history.is_empty() {
                                            if let Err(e) = self.database.clear_all_history() {
                                                eprintln!("Failed to clear history: {}", e);
                                            }
                                            self.benchmark_history.clear();
                                            self.benchmark_history_selected = None;
                                            self.benchmark_history_expanded.clear();
                                            self.message =
                                                Some("Benchmark history cleared".to_string());
                                            self.message_timestamp = Some(Instant::now());
                                        }
                                    }
                                    KeyCode::Up if !settings_editing => {
                                        if self.current_tab == Tab::Benchmark
                                            && !self.benchmark_history.is_empty()
                                        {
                                            self.navigate_history(-1);
                                        } else {
                                            self.handle_up();
                                        }
                                    }
                                    KeyCode::Down if !settings_editing => {
                                        if self.current_tab == Tab::Benchmark
                                            && !self.benchmark_history.is_empty()
                                        {
                                            self.navigate_history(1);
                                        } else {
                                            self.handle_down();
                                        }
                                    }
                                    KeyCode::Enter => self.handle_enter(),
                                    KeyCode::Char(' ')
                                        if self.current_tab == Tab::Settings
                                            && !settings_editing =>
                                    {
                                        self.cycle_setting();
                                    }
                                    KeyCode::Char(c)
                                        if self.current_tab == Tab::Settings
                                            && self.settings_editing
                                            && self.settings_edit_field
                                                == SettingsField::ExportPath =>
                                    {
                                        self.settings_input_buffer.push(c);
                                    }
                                    KeyCode::Char(c) if self.search_active => {
                                        self.search_query.push(c);
                                    }
                                    KeyCode::Backspace
                                        if self.current_tab == Tab::Settings
                                            && self.settings_editing
                                            && self.settings_edit_field
                                                == SettingsField::ExportPath =>
                                    {
                                        self.settings_input_buffer.pop();
                                    }
                                    KeyCode::Backspace if self.search_active => {
                                        self.search_query.pop();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(Event::Resize(_, _)) => {}
                    _ => {}
                }
            }
        }
    }

    fn check_message_expiry(&mut self) {
        if let Some(timestamp) = self.message_timestamp {
            if timestamp.elapsed().as_secs() >= 5 {
                self.message = None;
                self.message_timestamp = None;
            }
        }
    }

    fn check_export_status_expiry(&mut self) {
        if let Some(timestamp) = self.export_status_timestamp {
            if timestamp.elapsed().as_secs() >= 5 {
                self.export_status = None;
                self.export_status_timestamp = None;
            }
        }
    }

    fn handle_up(&mut self) {
        match self.current_tab {
            Tab::Disks => {
                if let Some(idx) = self.selected_disk_index {
                    if idx > 0 {
                        self.selected_disk_index = Some(idx - 1);
                        self.disk_list_state.select(idx - 1, self.disks.len());
                    }
                } else if !self.disks.is_empty() {
                    self.selected_disk_index = Some(0);
                    self.disk_list_state.select(0, self.disks.len());
                }
            }
            Tab::Export => {
                self.toggle_export_prev();
            }
            Tab::Settings => {
                self.settings_edit_field = match self.settings_edit_field {
                    SettingsField::None => SettingsField::BenchmarkSize,
                    SettingsField::ExportPath => SettingsField::BenchmarkSize,
                    SettingsField::Theme => SettingsField::ExportPath,
                    SettingsField::AutoRefresh => SettingsField::Theme,
                    SettingsField::BenchmarkSize => SettingsField::AutoRefresh,
                };
            }
            _ => {}
        }
    }

    fn handle_down(&mut self) {
        match self.current_tab {
            Tab::Disks => {
                if let Some(idx) = self.selected_disk_index {
                    if idx < self.disks.len() - 1 {
                        self.selected_disk_index = Some(idx + 1);
                        self.disk_list_state.select(idx + 1, self.disks.len());
                    }
                } else if !self.disks.is_empty() {
                    self.selected_disk_index = Some(0);
                    self.disk_list_state.select(0, self.disks.len());
                }
            }
            Tab::Export => {
                self.toggle_export_next();
            }
            Tab::Settings => {
                self.settings_edit_field = match self.settings_edit_field {
                    SettingsField::None => SettingsField::ExportPath,
                    SettingsField::ExportPath => SettingsField::Theme,
                    SettingsField::Theme => SettingsField::AutoRefresh,
                    SettingsField::AutoRefresh => SettingsField::BenchmarkSize,
                    SettingsField::BenchmarkSize => SettingsField::ExportPath,
                };
            }
            _ => {}
        }
    }

    fn navigate_history(&mut self, delta: i32) {
        use std::collections::BTreeMap;

        if self.benchmark_history.is_empty() {
            return;
        }

        let mut by_ts: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for h in &self.benchmark_history {
            by_ts.entry(h.timestamp.clone()).or_default().push(h);
        }

        let timestamps: Vec<String> = by_ts.keys().rev().cloned().collect();

        if timestamps.is_empty() {
            return;
        }

        let current = self.benchmark_history_selected;

        let new_idx = if let Some(curr) = current {
            let new_i = (curr as i32 + delta) as usize;
            if new_i >= timestamps.len() {
                0
            } else {
                new_i
            }
        } else {
            0
        };

        self.benchmark_history_selected = Some(new_idx);
    }

    fn toggle_export_next(&mut self) {
        match (self.export_format, self.export_content) {
            (ExportFormat::Json, ExportContent::SmartOnly) => {
                self.export_content = ExportContent::BenchmarkOnly
            }
            (ExportFormat::Json, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Json, ExportContent::Both) => {
                self.export_format = ExportFormat::Csv;
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Csv, ExportContent::SmartOnly) => {
                self.export_content = ExportContent::BenchmarkOnly
            }
            (ExportFormat::Csv, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Csv, ExportContent::Both) => {
                self.export_format = ExportFormat::Html;
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Html, ExportContent::SmartOnly) => {
                self.export_content = ExportContent::BenchmarkOnly
            }
            (ExportFormat::Html, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Html, ExportContent::Both) => {
                self.export_format = ExportFormat::Pdf;
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Pdf, ExportContent::SmartOnly) => {
                self.export_content = ExportContent::BenchmarkOnly
            }
            (ExportFormat::Pdf, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Pdf, ExportContent::Both) => {
                self.export_format = ExportFormat::Json;
                self.export_content = ExportContent::SmartOnly;
            }
        }
    }

    fn toggle_export_prev(&mut self) {
        match (self.export_format, self.export_content) {
            (ExportFormat::Json, ExportContent::SmartOnly) => {
                self.export_format = ExportFormat::Pdf;
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Json, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Json, ExportContent::Both) => {
                self.export_content = ExportContent::BenchmarkOnly;
            }
            (ExportFormat::Csv, ExportContent::SmartOnly) => {
                self.export_format = ExportFormat::Json;
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Csv, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Csv, ExportContent::Both) => {
                self.export_content = ExportContent::BenchmarkOnly;
            }
            (ExportFormat::Html, ExportContent::SmartOnly) => {
                self.export_format = ExportFormat::Csv;
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Html, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Html, ExportContent::Both) => {
                self.export_content = ExportContent::BenchmarkOnly;
            }
            (ExportFormat::Pdf, ExportContent::SmartOnly) => {
                self.export_format = ExportFormat::Html;
                self.export_content = ExportContent::Both;
            }
            (ExportFormat::Pdf, ExportContent::BenchmarkOnly) => {
                self.export_content = ExportContent::SmartOnly;
            }
            (ExportFormat::Pdf, ExportContent::Both) => {
                self.export_content = ExportContent::BenchmarkOnly;
            }
        }
    }

    fn handle_enter(&mut self) {
        match self.current_tab {
            Tab::Disks => {
                if let Some(idx) = self.selected_disk_index {
                    if idx < self.disks.len() {
                        self.load_disk_data(idx);
                        self.current_tab = Tab::Smart;
                    }
                }
            }
            Tab::Benchmark => {
                if !self.benchmark_history.is_empty() {
                    use std::collections::BTreeMap;
                    let mut by_ts: BTreeMap<String, Vec<_>> = BTreeMap::new();
                    for h in &self.benchmark_history {
                        by_ts.entry(h.timestamp.clone()).or_default().push(h);
                    }
                    let timestamps: Vec<_> = by_ts.keys().rev().cloned().collect();

                    if timestamps.is_empty() {
                        return;
                    }

                    let selected = self.benchmark_history_selected.unwrap_or(0);
                    let selected_ts = timestamps[selected].clone();

                    if let Some(idx) = self
                        .benchmark_history_expanded
                        .iter()
                        .position(|t| t == &selected_ts)
                    {
                        self.benchmark_history_expanded.remove(idx);
                    } else {
                        self.benchmark_history_expanded.push(selected_ts);
                    }
                }
            }
            Tab::Export => {
                self.do_export();
            }
            Tab::Settings => {
                if self.settings_edit_field == SettingsField::None {
                    self.settings_edit_field = SettingsField::ExportPath;
                    self.settings_editing = true;
                    self.settings_input_buffer = self.settings.export_path.clone();
                } else if self.settings_edit_field == SettingsField::ExportPath {
                    if self.settings_editing {
                        self.settings.export_path = self.settings_input_buffer.clone();
                        self.settings_editing = false;
                    } else {
                        self.settings_editing = true;
                        self.settings_input_buffer = self.settings.export_path.clone();
                    }
                }
            }
            _ => {}
        }
    }

    fn cycle_setting(&mut self) {
        match self.settings_edit_field {
            SettingsField::Theme => {
                self.settings.theme = self.settings.theme.next();
            }
            SettingsField::AutoRefresh => {
                self.settings.auto_refresh_interval = match self.settings.auto_refresh_interval {
                    0 => 30,
                    30 => 60,
                    60 => 120,
                    120 => 300,
                    300 => 0,
                    _ => 0,
                };
            }
            SettingsField::BenchmarkSize => {
                self.settings.benchmark_size_mb = match self.settings.benchmark_size_mb {
                    128 => 256,
                    256 => 512,
                    512 => 1024,
                    1024 => 2048,
                    2048 => 128,
                    _ => 512,
                };
            }
            _ => {}
        }
    }

    fn save_settings(&mut self) {
        match self.settings.save() {
            Ok(_) => {
                self.message = Some("Settings saved".to_string());
            }
            Err(e) => {
                self.message = Some(format!("Failed to save: {}", e));
            }
        }
        self.message_timestamp = Some(Instant::now());
    }

    fn reset_settings(&mut self) {
        self.settings = Settings::default();
        self.settings_edit_field = SettingsField::None;
        self.settings_editing = false;
        self.settings_input_buffer.clear();
        self.message = Some("Settings reset to default".to_string());
        self.message_timestamp = Some(Instant::now());
    }

    fn refresh(&mut self) {
        self.disks = disk::detect_disks();

        if let Some(idx) = self.selected_disk_index {
            if idx < self.disks.len() {
                self.load_disk_data(idx);
            }
        }

        self.message = Some("Data refreshed".to_string());
        self.message_timestamp = Some(Instant::now());
    }

    fn load_benchmark_history(&mut self) {
        if let Some(idx) = self.selected_disk_index {
            if let Some(disk) = self.disks.get(idx) {
                if let Ok(history) = self.database.get_history(&disk.device, &disk.serial, 50) {
                    self.benchmark_history = history;
                    self.benchmark_history_selected = None;
                    self.benchmark_history_expanded = Vec::new();
                }
            }
        }
    }

    fn load_disk_data(&mut self, index: usize) {
        if index < self.disks.len() {
            let disk = &self.disks[index];
            self.smart_data = Some(crate::disk::smart::get_smart_data(disk));

            if let Ok(history) = self.database.get_history(&disk.device, &disk.serial, 50) {
                self.benchmark_history = history;
                self.benchmark_history_selected = None;
                self.benchmark_history_expanded = Vec::new();
            }
        }
    }

    fn start_benchmark(&mut self) {
        if self.benchmark_running {
            return;
        }

        if let Some(idx) = self.selected_disk_index {
            if idx >= self.disks.len() {
                return;
            }

            let device = self.disks[idx].device.clone();
            self.benchmark_running = true;
            self.benchmark_results = None;
            self.benchmark_progress = Some(BenchmarkProgress {
                current_block: 0,
                total_blocks: 6,
                phase: "Initializing...".to_string(),
                percent: 0,
            });

            let shared = Arc::new(Mutex::new(None::<Vec<BenchmarkResult>>));
            let shared_clone = shared.clone();

            let progress = Arc::new(AtomicU32::new(0));
            let progress_clone = progress.clone();

            let phase = Arc::new(Mutex::new(String::new()));
            let phase_clone = phase.clone();

            thread::spawn(move || {
                let mut bench = Benchmark::new(device);
                bench.progress = progress_clone.clone();
                bench.current_phase = phase_clone.clone();
                let results = bench.run();
                let mut guard = shared_clone.lock().unwrap();
                *guard = Some(results);
            });

            self.benchmark_results_shared = Some(shared);
            self.benchmark_progress_shared = Some(progress);
            self.benchmark_phase_shared = Some(phase);
        }
    }

    fn check_benchmark_completion(&mut self) {
        if !self.benchmark_running {
            return;
        }

        if let Some(ref progress_shared) = self.benchmark_progress_shared {
            let current = progress_shared.load(Ordering::SeqCst);
            let phase = self
                .benchmark_phase_shared
                .as_ref()
                .and_then(|p| p.lock().ok())
                .map(|g| g.clone())
                .unwrap_or_default();
            let percent = ((current as f64 / 6.0) * 100.0) as u32;

            self.benchmark_progress = Some(BenchmarkProgress {
                current_block: current,
                total_blocks: 6,
                phase,
                percent,
            });
        }

        if let Some(ref shared) = self.benchmark_results_shared {
            if let Ok(mut guard) = shared.try_lock() {
                if let Some(results) = guard.take() {
                    self.benchmark_results = Some(results.clone());
                    self.benchmark_running = false;

                    if let Some(idx) = self.selected_disk_index {
                        if let Some(disk) = self.disks.get(idx) {
                            let db_results: Vec<(i32, f64, f64)> = results
                                .iter()
                                .map(|r| {
                                    (
                                        r.block_size_kb as i32,
                                        r.read_speed_mbps,
                                        r.write_speed_mbps,
                                    )
                                })
                                .collect();
                            if let Err(e) = self.database.save_benchmark(
                                &disk.device,
                                &disk.serial,
                                &db_results,
                            ) {
                                eprintln!("Failed to save benchmark: {}", e);
                            }
                            if let Ok(history) =
                                self.database.get_history(&disk.device, &disk.serial, 50)
                            {
                                self.benchmark_history = history;
                            }
                        }
                    }

                    self.message = Some("Benchmark complete".to_string());
                    self.benchmark_progress = None;
                    self.message_timestamp = Some(Instant::now());
                }
            }
        }
    }

    fn do_export(&mut self) {
        self.export_running = true;

        let disk_data = self
            .selected_disk_index
            .and_then(|idx| self.disks.get(idx).cloned());

        if disk_data.is_none() {
            self.export_status = Some("No disk selected".to_string());
            self.export_status_timestamp = Some(Instant::now());
            self.export_running = false;
            return;
        }

        if let Some(idx) = self.selected_disk_index {
            if idx < self.disks.len() {
                self.smart_data = Some(crate::disk::smart::get_smart_data(&self.disks[idx]));
            }
        }

        let smart_data = match self.export_content {
            ExportContent::SmartOnly | ExportContent::Both => {
                if self.smart_data.is_some() {
                    self.smart_data.clone()
                } else {
                    None
                }
            }
            ExportContent::BenchmarkOnly => None,
        };

        let benchmark_data = match self.export_content {
            ExportContent::BenchmarkOnly | ExportContent::Both => self.benchmark_results.clone(),
            ExportContent::SmartOnly => None,
        };

        let base_dir = if self.settings.export_path.is_empty() {
            let home = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy().to_string();
                    if home_str.contains("root") && !sudo_user.is_empty() {
                        PathBuf::from(format!("/home/{}", sudo_user))
                    } else {
                        home
                    }
                } else {
                    PathBuf::from("/tmp")
                }
            } else {
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")))
            };
            home
        } else {
            PathBuf::from(&self.settings.export_path)
        };

        let _ext = match self.export_format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
        };
        let suffix = match self.export_content {
            ExportContent::SmartOnly => "_smart",
            ExportContent::BenchmarkOnly => "_benchmark",
            ExportContent::Both => "",
        };
        let filename = if suffix.is_empty() {
            format!("dumbctl_export.json")
        } else {
            format!("dumbctl_export{}.json", suffix)
        };

        let filename = match self.export_format {
            ExportFormat::Json => filename,
            ExportFormat::Csv => {
                if suffix.is_empty() {
                    format!("dumbctl_export.csv")
                } else {
                    format!("dumbctl_export{}.csv", suffix)
                }
            }
            ExportFormat::Html => {
                if suffix.is_empty() {
                    format!("dumbctl_export.html")
                } else {
                    format!("dumbctl_export{}.html", suffix)
                }
            }
            ExportFormat::Pdf => {
                if suffix.is_empty() {
                    format!("dumbctl_export.pdf")
                } else {
                    format!("dumbctl_export{}.pdf", suffix)
                }
            }
        };
        let path = base_dir.join(filename);

        let history_ref = if self.benchmark_history.is_empty() {
            None
        } else {
            Some(&self.benchmark_history)
        };

        let result = match self.export_format {
            ExportFormat::Json => {
                crate::utils::export_to_json(&path, &disk_data, &smart_data, &benchmark_data)
            }
            ExportFormat::Html => crate::utils::export_to_html(
                &path,
                &disk_data,
                &smart_data,
                &benchmark_data,
                history_ref,
            ),
            ExportFormat::Csv => {
                crate::utils::export_to_csv(&path, &disk_data, &smart_data, &benchmark_data)
            }
            ExportFormat::Pdf => crate::utils::export_to_pdf(
                &path,
                &disk_data,
                &smart_data,
                &benchmark_data,
                history_ref,
            ),
        };

        self.export_running = false;
        self.export_status = match result {
            Ok(_) => Some(format!("Exported to {:?}", path)),
            Err(e) => Some(format!("Export failed: {}", e)),
        };
        self.export_status_timestamp = Some(Instant::now());
    }

    fn draw(&self, f: &mut Frame) {
        let theme_colors = self.settings.theme.colors();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.size());

        f.render_widget(ratatui::widgets::Clear, f.size());

        f.render_widget(
            ratatui::widgets::Block::default()
                .style(ratatui::style::Style::default().bg(theme_colors.bg)),
            f.size(),
        );

        self.draw_tabs(f, chunks[0]);
        self.draw_content(f, chunks[1]);
        self.draw_status(f, chunks[2]);
    }

    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        let theme_colors = self.settings.theme.colors();
        let tabs_width = 75;
        let start_x = (area.width.saturating_sub(tabs_width)) / 2;
        let start_y = (area.height.saturating_sub(1)) / 2;

        let tabs_area = Rect {
            x: area.x + start_x,
            y: area.y + start_y,
            width: tabs_width.min(area.width),
            height: 1,
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(15),
                Constraint::Length(16),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(14),
            ])
            .split(tabs_area);

        let tabs = [
            (Tab::Disks, " Disks "),
            (Tab::Smart, " SMART "),
            (Tab::Sectors, " Sectors "),
            (Tab::Benchmark, " Benchmark "),
            (Tab::Export, " Export "),
            (Tab::Settings, " Settings "),
        ];

        for (i, (tab, title)) in tabs.iter().enumerate() {
            let is_active = self.current_tab == *tab;
            let style = if is_active {
                let selected_fg = match theme_colors.selected {
                    ratatui::style::Color::White
                    | ratatui::style::Color::LightBlue
                    | ratatui::style::Color::LightGreen
                    | ratatui::style::Color::LightCyan
                    | ratatui::style::Color::Blue
                    | ratatui::style::Color::Cyan
                    | ratatui::style::Color::Green => ratatui::style::Color::Black,
                    _ => ratatui::style::Color::White,
                };
                ratatui::style::Style::default()
                    .bg(theme_colors.selected)
                    .fg(selected_fg)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                ratatui::style::Style::default().fg(theme_colors.fg)
            };

            f.render_widget(
                ratatui::widgets::Paragraph::new(*title)
                    .style(style)
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[i],
            );
        }
    }

    fn draw_content(&self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            Tab::Disks => tabs::disks::render(f, area, self),
            Tab::Smart => tabs::smart::render(f, area, self),
            Tab::Sectors => tabs::sectors::render(f, area, self),
            Tab::Benchmark => tabs::benchmark::render(f, area, self),
            Tab::Export => tabs::export::render(f, area, self),
            Tab::Settings => tabs::settings::render(f, area, self),
        }
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        let theme_colors = self.settings.theme.colors();

        let is_editing = self.current_tab == Tab::Settings && self.settings_editing;

        let default_text = if is_editing {
            "EDITING: Type path | Enter: confirm | Esc: cancel"
        } else if self.current_tab == Tab::Settings {
            "←/→: switch | ↑/↓: navigate | Enter: edit | Space: cycle | Ctrl+S: save"
        } else if self.current_tab == Tab::Benchmark {
            "←/→: switch | ↑/↓: navigate | Enter: expand | s: start | c: clear"
        } else {
            "←/→: switch | ↑/↓: navigate | Enter: select | r: refresh | s: benchmark"
        };

        let text = if let Some(msg) = &self.message {
            msg.clone()
        } else if let Some(status) = &self.export_status {
            status.clone()
        } else {
            default_text.to_string()
        };

        let version = "dumbctl - v0.3.114";
        let version_width = version.len() as u16;
        let status_width = area.width.saturating_sub(version_width + 1);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(status_width),
                Constraint::Length(version_width),
            ])
            .split(area);

        f.render_widget(
            ratatui::widgets::Paragraph::new(text)
                .style(ratatui::style::Style::default().fg(theme_colors.fg)),
            chunks[0],
        );
        f.render_widget(
            ratatui::widgets::Paragraph::new(version)
                .style(ratatui::style::Style::default().fg(theme_colors.fg)),
            chunks[1],
        );
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
