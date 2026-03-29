use crate::disk::MediaType;
use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

static SORT_FIELD: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static SORT_ORDER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn get_sort_field() -> usize {
    SORT_FIELD.load(std::sync::atomic::Ordering::SeqCst) as usize
}

fn get_sort_order() -> bool {
    SORT_ORDER.load(std::sync::atomic::Ordering::SeqCst) == 1
}

pub fn toggle_sort(field: usize) {
    let current = SORT_FIELD.load(std::sync::atomic::Ordering::SeqCst) as usize;
    if current == field {
        let current_order = SORT_ORDER.load(std::sync::atomic::Ordering::SeqCst);
        SORT_ORDER.store(1 - current_order, std::sync::atomic::Ordering::SeqCst);
    } else {
        SORT_FIELD.store(field as u8, std::sync::atomic::Ordering::SeqCst);
        SORT_ORDER.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme_colors = app.settings.theme.colors();
    let warning_color = theme_colors.warning;
    let title_color = theme_colors.title;

    if app.disks.is_empty() {
        let msg = "No disks detected. Press r to refresh.";
        f.render_widget(
            ratatui::widgets::Paragraph::new(msg)
                .style(Style::default().fg(warning_color))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Disk List ")
                        .border_style(Style::default().fg(theme_colors.border)),
                )
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let sort_indicator = if get_sort_order() { "↑" } else { "↓" };
    let sort_field_name = match get_sort_field() {
        0 => "Device",
        1 => "Model",
        2 => "Size",
        3 => "Type",
        _ => "Device",
    };
    let sort_hint = format!("Sort: {}{} | ", sort_indicator, sort_field_name);

    let search_hint = if app.search_active {
        format!(
            "{}Search: {} | Press / to close | 1-4: sort by field",
            sort_hint, app.search_query
        )
    } else {
        format!("{}Press / to search | 1-4: sort by field", sort_hint)
    };

    f.render_widget(
        ratatui::widgets::Paragraph::new(search_hint)
            .style(Style::default().fg(theme_colors.fg))
            .alignment(ratatui::layout::Alignment::Left),
        chunks[0],
    );

    let mut filtered_disks: Vec<(usize, &crate::disk::DiskInfo)> = app
        .disks
        .iter()
        .enumerate()
        .filter(|(_, disk)| {
            if app.search_query.is_empty() {
                true
            } else {
                let query = app.search_query.to_lowercase();
                disk.device.to_lowercase().contains(&query)
                    || disk.model.to_lowercase().contains(&query)
                    || disk.serial.to_lowercase().contains(&query)
            }
        })
        .collect();

    let sort_field = get_sort_field();
    let sort_asc = get_sort_order();

    filtered_disks.sort_by(|(_, a), (_, b)| {
        let cmp = match sort_field {
            0 => a.device.cmp(&b.device),
            1 => a.model.cmp(&b.model),
            2 => a.size.cmp(&b.size),
            3 => {
                let a_type = match a.media_type {
                    MediaType::Hdd => 0u8,
                    MediaType::Ssd => 1u8,
                    MediaType::Unknown => 2u8,
                };
                let b_type = match b.media_type {
                    MediaType::Hdd => 0u8,
                    MediaType::Ssd => 1u8,
                    MediaType::Unknown => 2u8,
                };
                a_type.cmp(&b_type)
            }
            _ => a.device.cmp(&b.device),
        };
        if sort_asc {
            cmp
        } else {
            cmp.reverse()
        }
    });

    if filtered_disks.is_empty() {
        f.render_widget(
            ratatui::widgets::Paragraph::new("No disks match your search.")
                .style(Style::default().fg(warning_color))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Disk List ")
                        .border_style(Style::default().fg(theme_colors.border)),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
        return;
    }

    let rows: Vec<Row> = filtered_disks
        .iter()
        .map(|(idx, disk)| {
            let size_gb = disk.size / (1024 * 1024 * 1024);
            let media = match disk.media_type {
                MediaType::Hdd => "HDD",
                MediaType::Ssd => "SSD",
                MediaType::Unknown => "Unknown",
            };
            let is_selected = app.selected_disk_index == Some(*idx);

            Row::new(vec![
                Cell::from(disk.device.clone()).style(if is_selected {
                    Style::default()
                        .fg(theme_colors.bg)
                        .bg(theme_colors.selected)
                } else {
                    Style::default().fg(theme_colors.fg)
                }),
                Cell::from(disk.model.trim().to_string()).style(if is_selected {
                    Style::default()
                        .fg(theme_colors.bg)
                        .bg(theme_colors.selected)
                } else {
                    Style::default().fg(theme_colors.fg)
                }),
                Cell::from(format!("{} GB", size_gb)).style(if is_selected {
                    Style::default()
                        .fg(theme_colors.bg)
                        .bg(theme_colors.selected)
                } else {
                    Style::default().fg(title_color)
                }),
                Cell::from(media.to_string()).style(if is_selected {
                    Style::default()
                        .fg(theme_colors.bg)
                        .bg(theme_colors.selected)
                } else {
                    let color = match disk.media_type {
                        MediaType::Ssd => theme_colors.healthy,
                        MediaType::Hdd => theme_colors.warning,
                        MediaType::Unknown => theme_colors.muted,
                    };
                    Style::default().fg(color)
                }),
                Cell::from(disk.serial.clone()).style(if is_selected {
                    Style::default()
                        .fg(theme_colors.bg)
                        .bg(theme_colors.selected)
                } else {
                    Style::default().fg(theme_colors.muted)
                }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Length(12),
            Constraint::Length(30),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Length(20),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Disk List - ↑/↓: navigate, Enter: select, /: search, r: refresh ")
            .title_style(Style::default().fg(theme_colors.title))
            .border_style(Style::default().fg(theme_colors.border)),
    )
    .header(
        Row::new(vec![
            Cell::from("Device").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("Model").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("Size").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("Type").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("Serial").style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ])
        .style(
            Style::default()
                .fg(title_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    )
    .highlight_style(
        Style::default()
            .fg(theme_colors.bg)
            .bg(theme_colors.selected)
            .add_modifier(ratatui::style::Modifier::BOLD),
    );

    f.render_widget(table, chunks[1]);
}
