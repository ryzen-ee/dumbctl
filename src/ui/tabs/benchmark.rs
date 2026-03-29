use crate::ui::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.selected_disk_index.is_none() {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "No disk selected. Go to Disk List tab and select a disk.",
            )
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Benchmark "))
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

    let instruction = format!("Selected: {} | Press 's' to start benchmark", disk_name);
    f.render_widget(
        ratatui::widgets::Paragraph::new(instruction)
            .style(
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(" Benchmark "))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    if app.benchmark_running {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Running benchmark... please wait...")
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
        return;
    }

    if let Some(results) = &app.benchmark_results {
        if results.is_empty() {
            f.render_widget(
                ratatui::widgets::Paragraph::new("No benchmark results. Press 's' to start.")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL))
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
                    Cell::from(r.block_size_kb.to_string()),
                    Cell::from(read_speed).style(Style::default().fg(Color::Cyan)),
                    Cell::from(write_speed).style(Style::default().fg(Color::Green)),
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
        .block(Block::default().borders(Borders::ALL).title(" Results "))
        .header(
            Row::new(vec![
                Cell::from("Block Size"),
                Cell::from("Read (MB/s)"),
                Cell::from("Write (MB/s)"),
            ])
            .style(
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        );

        f.render_widget(table, chunks[1]);
    } else {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Press 's' to start benchmark")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
