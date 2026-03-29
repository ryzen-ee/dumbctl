use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
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
    let theme_colors = app.settings.theme.colors();
    let warning_color = theme_colors.warning;
    let border_style = Style::default().fg(theme_colors.border);

    if app.selected_disk_index.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "No disk selected.\nGo to Disk List tab and select a disk.",
            )
            .style(Style::default().fg(warning_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details ")
                    .border_style(border_style),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    if app.smart_data.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Loading SMART data...")
                .style(Style::default().fg(warning_color))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Details ")
                        .border_style(border_style),
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
            .style(Style::default().fg(warning_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details ")
                    .border_style(border_style),
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
            .style(Style::default().fg(warning_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART Details ")
                    .border_style(border_style),
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
                .style(Style::default().fg(warning_color))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Details ")
                        .border_style(border_style),
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
    let healthy_color = theme_colors.healthy;
    let warning_color = theme_colors.warning;
    let critical_color = theme_colors.critical;
    let title_color = theme_colors.title;
    let normal_style = Style::default().fg(theme_colors.fg);

    let health_color = if smart.overall_health == "PASSED" {
        healthy_color
    } else if smart.overall_health == "FAILED" {
        critical_color
    } else {
        warning_color
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
        healthy_color
    } else {
        critical_color
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
                critical_color
            } else if t > 40 {
                warning_color
            } else {
                healthy_color
            }
        })
        .unwrap_or(theme_colors.muted);

    let realloc_color = if smart.reallocated_sectors > 0 {
        critical_color
    } else {
        healthy_color
    };
    let pending_color = if smart.pending_sectors > 0 {
        warning_color
    } else {
        theme_colors.muted
    };
    let uncorr_color = if smart.uncorrectable_errors > 0 {
        critical_color
    } else {
        healthy_color
    };

    let info_rows = vec![
        Row::new(vec![
            Cell::from("Device:").style(normal_style),
            Cell::from(disk_info.device.as_str()).style(Style::default().fg(title_color)),
        ]),
        Row::new(vec![
            Cell::from("Model:").style(normal_style),
            Cell::from(disk_info.model.trim()).style(normal_style),
        ]),
        Row::new(vec![
            Cell::from("Serial:").style(normal_style),
            Cell::from(disk_info.serial.trim()).style(normal_style),
        ]),
        Row::new(vec![
            Cell::from("Size:").style(normal_style),
            Cell::from(format_size(disk_info.size)).style(normal_style),
        ]),
        Row::new(vec![
            Cell::from("Media:").style(normal_style),
            Cell::from(media_str).style(Style::default().fg(title_color)),
        ]),
        Row::new(vec![
            Cell::from("SMART Status:").style(normal_style),
            Cell::from(health_str).style(Style::default().fg(health_str_color)),
        ]),
        Row::new(vec![
            Cell::from("Health:").style(normal_style),
            Cell::from(smart.overall_health.as_str()).style(
                Style::default()
                    .fg(health_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]),
        Row::new(vec![
            Cell::from("Power-On Hours:").style(normal_style),
            Cell::from(poh_str).style(normal_style),
        ]),
        Row::new(vec![
            Cell::from("Temperature:").style(normal_style),
            Cell::from(temp_str).style(Style::default().fg(temp_color)),
        ]),
        Row::new(vec![
            Cell::from("Reallocated:").style(normal_style),
            Cell::from(smart.reallocated_sectors.to_string())
                .style(Style::default().fg(realloc_color)),
        ]),
        Row::new(vec![
            Cell::from("Pending:").style(normal_style),
            Cell::from(smart.pending_sectors.to_string()).style(Style::default().fg(pending_color)),
        ]),
        Row::new(vec![
            Cell::from("Uncorrectable:").style(normal_style),
            Cell::from(smart.uncorrectable_errors.to_string())
                .style(Style::default().fg(uncorr_color)),
        ]),
    ];

    let info_table = Table::new(info_rows, &[Constraint::Length(20), Constraint::Min(10)]).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Disk Information ")
            .border_style(border_style),
    );

    f.render_widget(info_table, chunks[0]);

    if !smart.attributes.is_empty() {
        let attr_rows: Vec<Row> = smart
            .attributes
            .iter()
            .map(|attr| {
                let status_color = match attr.status {
                    crate::disk::SmartStatus::Ok => healthy_color,
                    crate::disk::SmartStatus::Warning => warning_color,
                    crate::disk::SmartStatus::Critical => critical_color,
                };
                Row::new(vec![
                    Cell::from(attr.id.to_string()).style(normal_style),
                    Cell::from(attr.name.as_str()).style(normal_style),
                    Cell::from(attr.value.to_string()).style(normal_style),
                    Cell::from(attr.worst.to_string()).style(normal_style),
                    Cell::from(attr.threshold.to_string()).style(normal_style),
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
                .title(" SMART Attributes ")
                .border_style(border_style),
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
                    .fg(title_color)
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
                .style(Style::default().fg(theme_colors.muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" SMART Attributes ")
                        .border_style(border_style),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
