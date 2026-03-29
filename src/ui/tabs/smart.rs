use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

fn format_size(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1000.0 {
        format!("{:.1} TB", gb / 1024.0)
    } else {
        format!("{:.1} GB", gb)
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.selected_disk_index.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "No disk selected.\nGo to Disk List tab and select a disk.",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details "),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    if app.smart_data.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Loading SMART data...")
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Details "),
                )
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let smart = app.smart_data.as_ref().unwrap();

    if smart.permission_error
        || smart.overall_health == "Need Root"
        || smart.overall_health == "Not Installed"
    {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "SMART data requires root access.\nRun with: sudo ./dumbctl",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details "),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    if smart.overall_health == "Not Installed" || smart.permission_error {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "smartmontools not found.\nInstall: sudo apt install smartmontools",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details "),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    if !smart.smart_enabled && smart.overall_health == "Unknown" && smart.attributes.is_empty() {
        let debug_msg = format!("No SMART data.\nDebug: {}", smart.debug_status);
        f.render_widget(
            ratatui::widgets::Paragraph::new(debug_msg)
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Details "),
                )
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(area);

    let disk_info = &smart.disk;
    let health_color = if smart.overall_health == "PASSED" {
        Color::Green
    } else if smart.overall_health == "FAILED" {
        Color::Red
    } else {
        Color::Yellow
    };

    let smart_available = smart.smart_enabled || smart.overall_health != "Unknown";

    let media_str = match disk_info.media_type {
        crate::disk::MediaType::Hdd => "HDD",
        crate::disk::MediaType::Ssd => "SSD",
        crate::disk::MediaType::Unknown => "Unknown",
    };

    let health_str = if smart_available {
        "Available"
    } else {
        "Not Available"
    };
    let health_str_color = if smart_available {
        Color::Green
    } else {
        Color::Red
    };

    let poh_str = if smart.power_on_hours > 0 {
        format!("{} hrs", smart.power_on_hours)
    } else {
        "N/A".to_string()
    };

    let temp_str = smart
        .temperature
        .map(|t| format!("{}°C", t))
        .unwrap_or_else(|| "N/A".to_string());
    let temp_color = smart
        .temperature
        .map(|t| {
            if t > 50 {
                Color::Red
            } else if t > 40 {
                Color::Yellow
            } else {
                Color::Green
            }
        })
        .unwrap_or(Color::DarkGray);

    let realloc_color = if smart.reallocated_sectors > 0 {
        Color::Red
    } else {
        Color::Green
    };
    let pending_color = if smart.pending_sectors > 0 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let uncorr_color = if smart.uncorrectable_errors > 0 {
        Color::Red
    } else {
        Color::Green
    };

    let info_rows = vec![
        Row::new(vec![
            Cell::from("Device:"),
            Cell::from(disk_info.device.as_str()).style(Style::default().fg(Color::LightBlue)),
        ]),
        Row::new(vec![
            Cell::from("Model:"),
            Cell::from(disk_info.model.trim()),
        ]),
        Row::new(vec![
            Cell::from("Serial:"),
            Cell::from(disk_info.serial.trim()),
        ]),
        Row::new(vec![
            Cell::from("Size:"),
            Cell::from(format_size(disk_info.size)),
        ]),
        Row::new(vec![
            Cell::from("Media:"),
            Cell::from(media_str).style(Style::default().fg(Color::Cyan)),
        ]),
        Row::new(vec![
            Cell::from("SMART Status:"),
            Cell::from(health_str).style(Style::default().fg(health_str_color)),
        ]),
        Row::new(vec![
            Cell::from("Health:"),
            Cell::from(smart.overall_health.as_str()).style(
                Style::default()
                    .fg(health_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]),
        Row::new(vec![Cell::from("Power-On Hours:"), Cell::from(poh_str)]),
        Row::new(vec![
            Cell::from("Temperature:"),
            Cell::from(temp_str).style(Style::default().fg(temp_color)),
        ]),
        Row::new(vec![
            Cell::from("Reallocated:"),
            Cell::from(smart.reallocated_sectors.to_string())
                .style(Style::default().fg(realloc_color)),
        ]),
        Row::new(vec![
            Cell::from("Pending:"),
            Cell::from(smart.pending_sectors.to_string()).style(Style::default().fg(pending_color)),
        ]),
        Row::new(vec![
            Cell::from("Uncorrectable:"),
            Cell::from(smart.uncorrectable_errors.to_string())
                .style(Style::default().fg(uncorr_color)),
        ]),
    ];

    let info_table = Table::new(info_rows, &[Constraint::Length(20), Constraint::Min(10)]).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Disk Information "),
    );

    f.render_widget(info_table, chunks[0]);

    if !smart.attributes.is_empty() {
        let attr_rows: Vec<Row> = smart
            .attributes
            .iter()
            .map(|attr| {
                let status_color = match attr.status {
                    crate::disk::SmartStatus::Ok => Color::Green,
                    crate::disk::SmartStatus::Warning => Color::Yellow,
                    crate::disk::SmartStatus::Critical => Color::Red,
                };
                Row::new(vec![
                    Cell::from(attr.id.to_string()),
                    Cell::from(attr.name.as_str()),
                    Cell::from(attr.value.to_string()),
                    Cell::from(attr.worst.to_string()),
                    Cell::from(attr.threshold.to_string()),
                    Cell::from(attr.raw.to_string()).style(Style::default().fg(status_color)),
                ])
            })
            .collect();

        let attrs_table = Table::new(
            attr_rows,
            &[
                Constraint::Length(5),
                Constraint::Length(30),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Min(10),
            ],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" SMART Attributes "),
        )
        .header(
            Row::new(vec![
                Cell::from("ID"),
                Cell::from("Name"),
                Cell::from("Value"),
                Cell::from("Worst"),
                Cell::from("Thresh"),
                Cell::from("Raw"),
            ])
            .style(
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        );

        f.render_widget(attrs_table, chunks[1]);
    } else {
        let attr_msg =
            if smart.overall_health == "Not Installed" || smart.overall_health == "Need Root" {
                "No SMART access."
            } else if smart.overall_health != "Unknown" {
                "No detailed attributes available."
            } else {
                "No SMART data available."
            };
        f.render_widget(
            ratatui::widgets::Paragraph::new(attr_msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Attributes "),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
