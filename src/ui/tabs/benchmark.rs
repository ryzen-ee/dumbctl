use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme_colors = app.settings.theme.colors();
    let warning_color = theme_colors.warning;
    let healthy_color = theme_colors.healthy;
    let title_color = theme_colors.title;
    let normal_style = Style::default().fg(theme_colors.fg);

    let border_style = Style::default().fg(theme_colors.border);

    if app.selected_disk_index.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "No disk selected. Go to Disk List tab and select a disk.",
            )
            .style(Style::default().fg(warning_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Benchmark ")
                    .border_style(border_style),
            )
            .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let disk_name = app
        .disks
        .get(app.selected_disk_index.unwrap())
        .map(|d| d.device.as_str())
        .unwrap_or("Unknown");

    let instruction = format!("Selected: {} | 's': start | 'b': background", disk_name);
    f.render_widget(
        ratatui::widgets::Paragraph::new(instruction)
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Benchmark ")
                    .border_style(border_style),
            )
            .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    if app.benchmark_running {
        let progress = app.benchmark_progress.as_ref();
        let phase = progress.map(|p| p.phase.as_str()).unwrap_or("Running...");
        let percent = progress.map(|p| p.percent).unwrap_or(0);

        let progress_bar = ratatui::widgets::Gauge::default()
            .gauge_style(Style::default().fg(warning_color).bg(theme_colors.muted))
            .percent(percent as u16);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        f.render_widget(
            ratatui::widgets::Paragraph::new(format!("Running benchmark... {}%", percent))
                .style(
                    Style::default()
                        .fg(warning_color)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );

        f.render_widget(progress_bar, chunks[1]);

        f.render_widget(
            ratatui::widgets::Paragraph::new(phase)
                .style(Style::default().fg(theme_colors.fg))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
        return;
    }

    if let Some(results) = &app.benchmark_results {
        if results.is_empty() {
            f.render_widget(
                ratatui::widgets::Paragraph::new("No benchmark results. Press 's' to start.")
                    .style(Style::default().fg(theme_colors.muted))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    )
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[1],
            );
            return;
        }

        let rows: Vec<Row> = results
            .iter()
            .map(|r| {
                let read_speed = format!("{:.2}", r.read_speed_mbps);
                let write_speed = format!("{:.2}", r.write_speed_mbps);
                Row::new(vec![
                    Cell::from(r.block_size_kb.to_string()).style(normal_style),
                    Cell::from(read_speed).style(Style::default().fg(title_color)),
                    Cell::from(write_speed).style(Style::default().fg(healthy_color)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Length(15),
                Constraint::Length(20),
                Constraint::Length(20),
            ],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Results ")
                .border_style(border_style),
        )
        .header(
            Row::new(vec![
                Cell::from("Block Size").style(normal_style),
                Cell::from("Read (MB/s)").style(normal_style),
                Cell::from("Write (MB/s)").style(normal_style),
            ])
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        );

        f.render_widget(table, chunks[1]);
    } else {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Press 's' to start benchmark")
                .style(Style::default().fg(theme_colors.muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
