use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::ui::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let theme_colors = app.settings.theme.colors();
    let title_color = theme_colors.title;
    let healthy_color = theme_colors.healthy;
    let warning_color = theme_colors.warning;
    let border_style = Style::default().fg(theme_colors.border);
    let normal_style = Style::default().fg(theme_colors.fg);

    let disk_name = app
        .disks
        .get(app.selected_disk_index.unwrap_or(0))
        .map(|d| d.device.as_str())
        .unwrap_or("No disk selected");

    let instruction = format!("Selected: {} | 's': start | 'c': clear history", disk_name);
    f.render_widget(
        ratatui::widgets::Paragraph::new(instruction)
            .style(
                ratatui::style::Style::default()
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
            .gauge_style(
                ratatui::style::Style::default()
                    .fg(warning_color)
                    .bg(theme_colors.muted),
            )
            .percent(percent as u16);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        f.render_widget(progress_bar, chunks[1]);

        f.render_widget(
            ratatui::widgets::Paragraph::new(phase)
                .style(ratatui::style::Style::default().fg(theme_colors.fg))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
        return;
    }

    let has_results = app
        .benchmark_results
        .as_ref()
        .map(|r| !r.is_empty())
        .unwrap_or(false);
    let has_history = !app.benchmark_history.is_empty();

    if app.disks.is_empty() {
        f.render_widget(
            ratatui::widgets::Paragraph::new("No disks available")
                .style(ratatui::style::Style::default().fg(theme_colors.muted))
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

    if has_results {
        let results = app.benchmark_results.as_ref().unwrap();
        let rows: Vec<Row> = results
            .iter()
            .map(|r| {
                Row::new(vec![
                    Cell::from(r.block_size_kb.to_string()).style(normal_style),
                    Cell::from(format!("{:.2}", r.read_speed_mbps))
                        .style(ratatui::style::Style::default().fg(title_color)),
                    Cell::from(format!("{:.2}", r.write_speed_mbps))
                        .style(ratatui::style::Style::default().fg(healthy_color)),
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
                ratatui::style::Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        );

        if has_history {
            let content_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(chunks[1]);

            f.render_widget(table, content_chunks[0]);
            render_history(f, app, content_chunks[1], &theme_colors, title_color);
        } else {
            f.render_widget(table, chunks[1]);
        }
    } else if has_history {
        render_history(f, app, chunks[1], &theme_colors, title_color);
    } else {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Press 's' to start benchmark")
                .style(ratatui::style::Style::default().fg(theme_colors.muted))
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

fn render_history(
    f: &mut Frame,
    app: &App,
    area: Rect,
    theme_colors: &crate::settings::ThemeColors,
    title_color: ratatui::style::Color,
) {
    use ratatui::style::Style;
    use std::collections::BTreeMap;

    let border_style = Style::default().fg(theme_colors.border);
    let normal_style = Style::default().fg(theme_colors.fg);

    let mut by_ts: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for h in &app.benchmark_history {
        by_ts.entry(h.timestamp.clone()).or_default().push(h);
    }

    let timestamps: Vec<_> = by_ts.keys().rev().cloned().collect();
    let selected_idx = app.benchmark_history_selected.unwrap_or(0);
    let expanded_list = &app.benchmark_history_expanded;

    let mut rows: Vec<Row> = Vec::new();
    for (i, ts) in timestamps.iter().enumerate() {
        let is_selected = i == selected_idx;
        let is_expanded = expanded_list.contains(ts);
        let icon = if is_expanded { "[-]" } else { "[+]" };

        rows.push(Row::new(vec![Cell::from(format!(
            "{:>2}. {} {}",
            i + 1,
            icon,
            ts
        ))
        .style(if is_selected {
            ratatui::style::Style::default()
                .fg(theme_colors.selected)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            normal_style
        })]));

        if is_expanded {
            rows.push(Row::new(vec![Cell::from("    BLOCK    READ     WRITE")
                .style(ratatui::style::Style::default().fg(theme_colors.muted))]));

            for e in by_ts.get(ts).unwrap() {
                rows.push(Row::new(vec![Cell::from(format!(
                    "    {:4}KB   {:>6.1}   {:>6.1}",
                    e.block_size_kb, e.read_speed_mbps, e.write_speed_mbps
                ))
                .style(ratatui::style::Style::default().fg(title_color))]));
            }
        }
    }

    let history_table = Table::new(rows, &[Constraint::Min(45)]).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Benchmark History ")
            .border_style(border_style),
    );

    f.render_widget(history_table, area);
}
