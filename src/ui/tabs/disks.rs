use crate::ui::App;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme_colors = app.settings.theme.colors();
    let warning_color = theme_colors.warning;

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
                    .fg(theme_colors.bg)
                    .bg(theme_colors.selected)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(theme_colors.fg)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Disk List - Select with ↑/↓, Enter to view ")
                .title_style(Style::default().fg(theme_colors.title))
                .border_style(Style::default().fg(theme_colors.border)),
        )
        .highlight_style(
            Style::default()
                .fg(theme_colors.bg)
                .bg(theme_colors.selected)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

    f.render_widget(list, area);
}
