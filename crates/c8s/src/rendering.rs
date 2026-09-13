use k9tui::{
    theme,
    widgets::{hotkey::Hotkey, table::DataTable, top_bar::TopBarView},
};
use ratatui::{
    Frame,
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    app::{APP_NAME, App},
    app_state::AppState,
    ui::widgets::hotkeys::{GLOBAL_HOTKEYS, log_hotkeys},
};

const TOPBAR_HEIGHT: u16 = 7;
const FOOTER_HEIGHT: u16 = 1;

impl App {
    pub fn render(&mut self, frame: &mut Frame) {
        match &self.state {
            AppState::Connecting => self.render_connecting(frame),
            AppState::ConnectError(message) => {
                self.render_connect_error(frame, message.clone());
            }
            AppState::List => self.render_list(frame),
            AppState::Logs { name, .. } => {
                self.render_logs(frame, name.clone());
            }
        }
    }

    fn render_connecting(&self, frame: &mut Frame) {
        let area = frame.area();
        Paragraph::new("Connecting to Docker daemon…")
            .style(theme::muted())
            .alignment(Alignment::Center)
            .render(centered(area), frame.buffer_mut());
    }

    fn render_connect_error(&self, frame: &mut Frame, message: String) {
        let area = frame.area();
        let text = format!("{message}\n\nPress r to retry, q to quit.");
        Paragraph::new(text)
            .style(Style::default().fg(ratatui::style::Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(centered(area), frame.buffer_mut());
    }

    fn render_list(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(TOPBAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(frame.area());

        let top_area = layout.first().copied().unwrap_or_default();
        let content_area = layout.get(1).copied().unwrap_or_default();
        let footer_area = layout.get(2).copied().unwrap_or_default();

        let global: Vec<Hotkey> = GLOBAL_HOTKEYS.to_vec();
        let n = self.containers.model.items.len();
        let summary = format!("Containers: {n}");

        frame.render_widget(
            TopBarView {
                summary: &summary,
                recent_hotkeys: &[],
                hotkeys: &self.hotkeys,
                global_hotkeys: &global,
                app_name: APP_NAME,
                build_info: Some(self.build_info.clone()),
            },
            top_area,
        );

        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(format!(" Containers [{n}] "))
            .title_alignment(Alignment::Center);
        let inner = block.inner(content_area);
        frame.render_widget(block, content_area);

        if n == 0 {
            Paragraph::new("No containers found")
                .style(theme::muted())
                .alignment(Alignment::Center)
                .render(inner, frame.buffer_mut());
        } else {
            frame.render_stateful_widget(
                DataTable::default(),
                inner,
                &mut self.containers,
            );
        }

        frame.render_widget(self.status_line.clone(), footer_area);

        if let Some(dialog) = self.confirm_dialog.clone() {
            frame.render_widget(dialog, frame.area());
        }
    }

    fn render_logs(&mut self, frame: &mut Frame, name: String) {
        let layout = Layout::vertical([
            Constraint::Length(TOPBAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(frame.area());

        let top_area = layout.first().copied().unwrap_or_default();
        let content_area = layout.get(1).copied().unwrap_or_default();
        let footer_area = layout.get(2).copied().unwrap_or_default();

        let hotkeys = log_hotkeys();
        frame.render_widget(
            TopBarView {
                summary: &format!("Logs: {name}"),
                recent_hotkeys: &[],
                hotkeys: &hotkeys,
                global_hotkeys: &[],
                app_name: APP_NAME,
                build_info: None,
            },
            top_area,
        );

        let title = if self.log_follow {
            format!(" Logs: {name} (live) ")
        } else {
            format!(" Logs: {name} (scrolled, G to follow) ")
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(title)
            .title_alignment(Alignment::Center);
        let inner = block.inner(content_area);
        frame.render_widget(block, content_area);

        // log_scroll is the absolute index of the first visible line.
        let visible = inner.height as usize;
        self.log_viewport_height = visible;
        let max_start = self.log_lines.len().saturating_sub(visible);
        let start = if self.log_follow {
            max_start
        } else {
            self.log_scroll.min(max_start)
        };
        self.log_scroll = start;
        let end = (start + visible).min(self.log_lines.len());
        let text = self.log_lines.get(start..end).unwrap_or(&[]).join("\n");
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .render(inner, frame.buffer_mut());

        frame.render_widget(self.status_line.clone(), footer_area);
    }
}

fn centered(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(3)])
        .flex(layout::Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(60)])
        .flex(layout::Flex::Center)
        .split(vertical.first().copied().unwrap_or(area));
    horizontal.first().copied().unwrap_or(area)
}
