use crate::ui::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.disks.is_empty() {
        let msg = "No disks detected. Press r to refresh.";
        f.render_widget(
            ratatui::widgets::Paragraph::new(msg)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(" Disk List "))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .disks
        .iter()
        .enumerate()
        .map(|(i, disk)| {
            let size_gb = disk.size / (1024 * 1024 * 1024);
            let media = match disk.media_type {
                crate::disk::MediaType::Hdd => "HDD",
                crate::disk::MediaType::Ssd => "SSD",
                crate::disk::MediaType::Unknown => "Unknown",
            };
            let label = format!(
                "{} | {} GB | {} | {}",
                disk.device,
                size_gb,
                disk.model.trim(),
                media
            );
            let style = if app.selected_disk_index == Some(i) {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Disk List - Select with ↑/↓, Enter to view "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

    f.render_widget(list, area);
}
