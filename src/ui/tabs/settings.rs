use crate::ui::{App, SettingsField};
use ratatui::{
    layout::{Constraint, Direction, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme_colors = app.settings.theme.colors();

    let chunks = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(5),
        ])
        .split(area);

    let title_style = Style::default()
        .fg(theme_colors.title)
        .add_modifier(ratatui::style::Modifier::BOLD);
    let normal_style = Style::default().fg(theme_colors.fg);
    let selected_style = Style::default()
        .fg(theme_colors.selected)
        .add_modifier(ratatui::style::Modifier::BOLD);

    f.render_widget(
        Paragraph::new("Settings")
            .style(title_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Settings ")
                    .title_style(title_style)
                    .border_style(Style::default().fg(theme_colors.border)),
            )
            .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    let export_path_value =
        if app.settings_editing && app.settings_edit_field == SettingsField::ExportPath {
            app.settings_input_buffer.clone()
        } else if app.settings.export_path.is_empty() {
            "Default (home directory)".to_string()
        } else {
            app.settings.export_path.clone()
        };

    let setting1_label = if app.settings_edit_field == SettingsField::ExportPath {
        "▶ 1. Export Path: "
    } else {
        "  1. Export Path: "
    };
    let export_style =
        if app.settings_editing && app.settings_edit_field == SettingsField::ExportPath {
            Style::default()
                .fg(theme_colors.selected)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else if app.settings_edit_field == SettingsField::ExportPath {
            selected_style
        } else {
            normal_style
        };
    f.render_widget(
        Paragraph::new(format!("{}{}", setting1_label, export_path_value))
            .style(export_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Export ")
                    .border_style(Style::default().fg(theme_colors.border)),
            ),
        chunks[1],
    );

    let setting2_label = if app.settings_edit_field == SettingsField::Theme {
        "▶ 2. Theme: "
    } else {
        "  2. Theme: "
    };
    f.render_widget(
        Paragraph::new(format!("{}{}", setting2_label, app.settings.theme.label()))
            .style(if app.settings_edit_field == SettingsField::Theme {
                selected_style
            } else {
                normal_style
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Appearance ")
                    .border_style(Style::default().fg(theme_colors.border)),
            ),
        chunks[2],
    );

    let auto_refresh_str = if app.settings.auto_refresh_interval == 0 {
        "Disabled".to_string()
    } else {
        format!("{} seconds", app.settings.auto_refresh_interval)
    };
    let setting3_label = if app.settings_edit_field == SettingsField::AutoRefresh {
        "▶ 3. Auto-refresh: "
    } else {
        "  3. Auto-refresh: "
    };
    f.render_widget(
        Paragraph::new(format!("{}{}", setting3_label, auto_refresh_str))
            .style(if app.settings_edit_field == SettingsField::AutoRefresh {
                selected_style
            } else {
                normal_style
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SMART ")
                    .border_style(Style::default().fg(theme_colors.border)),
            ),
        chunks[3],
    );

    let setting4_label = if app.settings_edit_field == SettingsField::BenchmarkSize {
        "▶ 4. Benchmark Size: "
    } else {
        "  4. Benchmark Size: "
    };
    f.render_widget(
        Paragraph::new(format!(
            "{}{} MB",
            setting4_label, app.settings.benchmark_size_mb
        ))
        .style(if app.settings_edit_field == SettingsField::BenchmarkSize {
            selected_style
        } else {
            normal_style
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Benchmark ")
                .border_style(Style::default().fg(theme_colors.border)),
        ),
        chunks[4],
    );

    let help_text = format!(
        "Navigate: ↑/↓ | Change: Enter/Space | Save: Ctrl+S | Reset: Ctrl+R\nConfig: ~/.config/dumbctl/settings.json"
    );
    f.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(theme_colors.muted))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Help ")
                    .border_style(Style::default().fg(theme_colors.border)),
            )
            .alignment(ratatui::layout::Alignment::Center),
        chunks[5],
    );
}
