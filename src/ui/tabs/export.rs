use crate::ui::{App, ExportFormat};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Gauge, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme_colors = app.settings.theme.colors();
    let warning_color = theme_colors.warning;
    let normal_style = Style::default().fg(theme_colors.fg);
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
                    .title(" Export ")
                    .border_style(border_style),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    let disk_name = app
        .disks
        .get(app.selected_disk_index.unwrap())
        .map(|d| d.device.as_str())
        .unwrap_or("Unknown");

    let format_label = match app.export_format {
        ExportFormat::Json => "JSON",
        ExportFormat::Csv => "CSV",
        ExportFormat::Html => "HTML",
    };

    let content_label = app.export_content.label();

    let rows = vec![
        Row::new(vec![
            Cell::from("Format:").style(normal_style),
            Cell::from(format_label).style(
                Style::default()
                    .fg(if app.export_format == ExportFormat::Json {
                        theme_colors.healthy
                    } else {
                        theme_colors.title
                    })
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("").style(Style::default().fg(theme_colors.muted)),
        ]),
        Row::new(vec![
            Cell::from("Include:").style(normal_style),
            Cell::from(content_label).style(
                Style::default()
                    .fg(theme_colors.selected)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Cell::from("↑/↓ to toggle").style(Style::default().fg(theme_colors.muted)),
        ]),
    ];

    let table = Table::new(
        rows,
        &[
            Constraint::Length(20),
            Constraint::Length(25),
            Constraint::Min(0),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Export Options ")
            .border_style(border_style),
    );

    f.render_widget(table, chunks[1]);

    f.render_widget(
        ratatui::widgets::Paragraph::new(format!("Selected: {} | {}", disk_name, content_label))
            .style(
                Style::default()
                    .fg(theme_colors.selected)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Selected ")
                    .border_style(border_style),
            )
            .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    let action_text: String = if app.export_running {
        "Exporting...".to_string()
    } else if let Some(ref status) = app.export_status {
        if status.contains("Exported") || status.contains("failed") {
            status.clone()
        } else {
            "Press Enter to export".to_string()
        }
    } else {
        "Press Enter to export".to_string()
    };

    let action_color = if app.export_running {
        theme_colors.warning
    } else if app
        .export_status
        .as_ref()
        .map(|s| s.contains("Exported"))
        .unwrap_or(false)
    {
        theme_colors.healthy
    } else if app
        .export_status
        .as_ref()
        .map(|s| s.contains("failed"))
        .unwrap_or(false)
    {
        theme_colors.critical
    } else {
        theme_colors.healthy
    };

    if app.export_running {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Exporting... ")
                    .border_style(border_style),
            )
            .gauge_style(Style::default().fg(theme_colors.selected))
            .label(&action_text)
            .ratio(0.5);

        f.render_widget(gauge, chunks[2]);
    } else {
        f.render_widget(
            ratatui::widgets::Paragraph::new(action_text)
                .style(
                    Style::default()
                        .fg(action_color)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Action ")
                        .border_style(border_style),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
    }
}
