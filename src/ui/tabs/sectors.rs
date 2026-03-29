use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Gauge, Row, Table},
    Frame,
};

fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1000.0 {
        format!("{:.2} TB", gb / 1024.0)
    } else {
        format!("{:.2} GB", gb)
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.selected_disk_index.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "No disk selected.\nGo to Disk List tab and select a disk.",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Sectors "))
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let Some(smart) = &app.smart_data else {
        f.render_widget(
            ratatui::widgets::Paragraph::new("No SMART data available.\nGo to SMART tab first.")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(" Sectors "))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    };

    if smart.permission_error
        || smart.overall_health == "Need Root"
        || smart.overall_health == "Not Installed"
    {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "SMART data requires root access.\nRun with: sudo ./dumbctl",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Sectors "))
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let disk = &smart.disk;
    let total_sectors = disk.size / 512;

    let good_sectors = if smart.reallocated_sectors > 0
        || smart.pending_sectors > 0
        || smart.uncorrectable_errors > 0
    {
        total_sectors.saturating_sub(
            smart.reallocated_sectors + smart.pending_sectors + smart.uncorrectable_errors,
        )
    } else {
        total_sectors
    };

    let reallocated = smart.reallocated_sectors;
    let pending = smart.pending_sectors;
    let uncorrectable = smart.uncorrectable_errors;
    let total_bad = reallocated + pending + uncorrectable;

    let good_pct = if total_sectors > 0 {
        (good_sectors as f64 / total_sectors as f64 * 100.0).min(100.0)
    } else {
        100.0
    };

    let bad_pct = if total_sectors > 0 {
        total_bad as f64 / total_sectors as f64 * 100.0
    } else {
        0.0
    };

    let sector_size = 512;
    let good_bytes = good_sectors * sector_size;
    let reallocated_bytes = reallocated * sector_size;
    let pending_bytes = pending * sector_size;
    let uncorrectable_bytes = uncorrectable * sector_size;

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        ratatui::widgets::Paragraph::new(format!(
            "Disk: {} | Total: {}",
            disk.device,
            format_bytes(disk.size)
        ))
        .style(
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Sector Overview "),
        )
        .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    let rows = vec![
        Row::new(vec![
            Cell::from("Status"),
            Cell::from("Sectors"),
            Cell::from("Size"),
            Cell::from("Description"),
        ])
        .style(
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Row::new(vec![
            Cell::from("Good"),
            Cell::from(good_sectors.to_string()).style(Style::default().fg(Color::Green)),
            Cell::from(format_bytes(good_bytes)),
            Cell::from("Healthy sectors"),
        ]),
        Row::new(vec![
            Cell::from("Reallocated"),
            Cell::from(reallocated.to_string()).style(Style::default().fg(if reallocated > 0 {
                Color::Red
            } else {
                Color::Green
            })),
            Cell::from(format_bytes(reallocated_bytes)),
            Cell::from("Bad sectors remapped"),
        ]),
        Row::new(vec![
            Cell::from("Pending"),
            Cell::from(pending.to_string()).style(Style::default().fg(if pending > 0 {
                Color::Yellow
            } else {
                Color::Green
            })),
            Cell::from(format_bytes(pending_bytes)),
            Cell::from("Sectors waiting to be remapped"),
        ]),
        Row::new(vec![
            Cell::from("Uncorrectable"),
            Cell::from(uncorrectable.to_string()).style(Style::default().fg(
                if uncorrectable > 0 {
                    Color::Red
                } else {
                    Color::Green
                },
            )),
            Cell::from(format_bytes(uncorrectable_bytes)),
            Cell::from("Read errors - could not recover"),
        ]),
        Row::new(vec![
            Cell::from("Total Bad"),
            Cell::from(total_bad.to_string()).style(
                Style::default()
                    .fg(if total_bad > 0 {
                        Color::Red
                    } else {
                        Color::Green
                    })
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from(format_bytes(total_bad * sector_size)),
            Cell::from(if total_bad == 0 {
                "Disk is healthy"
            } else {
                "Warning: Bad sectors detected"
            }),
        ]),
    ];

    let table = Table::new(
        rows,
        &[
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Min(10),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Sector Details "),
    );

    f.render_widget(table, chunks[1]);

    let bar_label = format!("Good: {:.4}% | Bad: {:.4}%", good_pct, bad_pct);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Disk Health "),
        )
        .gauge_style(Style::default().fg(Color::Green))
        .label(bar_label)
        .ratio(good_pct / 100.0);

    f.render_widget(gauge, chunks[2]);

    let health_status = if total_bad == 0 {
        ("Excellent".to_string(), Color::Green)
    } else if total_bad < 10 {
        ("Warning".to_string(), Color::Yellow)
    } else {
        ("Critical".to_string(), Color::Red)
    };

    let recommendation = if total_bad == 0 {
        "Your disk is in good condition. Continue monitoring regularly."
    } else if total_bad < 10 {
        "Consider backing up important data and monitoring the disk closely."
    } else {
        "Disk is showing signs of failure. Back up data immediately and consider replacement."
    };

    let status_text = format!("Health: {} | {}", health_status.0, recommendation);

    f.render_widget(
        ratatui::widgets::Paragraph::new(status_text)
            .style(Style::default().fg(health_status.1))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Recommendation "),
            )
            .alignment(ratatui::layout::Alignment::Center),
        chunks[3],
    );
}
