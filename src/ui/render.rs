use super::app::App;
use crate::state::ControlId;
use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialize the terminal
pub fn init() -> Result<Tui> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore() -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    Ok(())
}

/// Render the TUI
pub fn render(terminal: &mut Tui, app: &App) -> Result<()> {
    terminal.draw(|f| {
        let chunks = create_layout(f.area());

        // Render status bar
        render_status_bar(f, app, chunks[0]);

        // Render spectrum
        render_spectrum_placeholder(f, app, chunks[1]);

        // Render waterfall
        render_waterfall_placeholder(f, app, chunks[2]);

        // Split bottom area into controls and decoder output
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[3]);

        // Render controls
        render_controls(f, app, bottom_chunks[0]);

        // Render decoder output placeholder
        render_decoder_placeholder(f, bottom_chunks[1]);
    })?;
    Ok(())
}

/// Create the main layout
fn create_layout(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Percentage(30),  // Spectrum
            Constraint::Percentage(30),  // Waterfall
            Constraint::Percentage(40),  // Bottom (controls + decoder)
        ])
        .split(area)
}

/// Render the status bar
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let freq = app.get_frequency();
    let is_recording = app.is_recording();
    let status = app.get_status();

    let title = if is_recording {
        format!("[RECORDING] RTL-SDR TUI - {} MHz", freq as f64 / 1_000_000.0)
    } else {
        format!("RTL-SDR TUI - {:.3} MHz", freq as f64 / 1_000_000.0)
    };

    let status_text = vec![
        Line::from(vec![
            Span::styled(
                title,
                Style::default()
                    .fg(if is_recording { Color::Red } else { Color::Cyan })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(status, Style::default().fg(Color::Yellow)),
        ]),
    ];

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

/// Render spectrum analyzer
fn render_spectrum_placeholder(f: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read();
    let freq = state.sdr.frequency;
    let sample_rate = state.sdr.sample_rate;

    let block = Block::default()
        .title("Spectrum Analyzer")
        .borders(Borders::ALL);

    // Get FFT data from state
    let fft_data = &state.spectrum.fft_data;

    if fft_data.is_empty() {
        // Show placeholder if no data
        let text = Paragraph::new("Waiting for signal data...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(text, area);
    } else {
        // Render actual spectrum
        let widget = super::widgets::SpectrumWidget::new(fft_data, freq, sample_rate)
            .block(block)
            .db_range(-100.0, 0.0);
        f.render_widget(widget, area);
    }
}

/// Render waterfall display
fn render_waterfall_placeholder(f: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read();

    let block = Block::default()
        .title("Waterfall Display")
        .borders(Borders::ALL);

    // Get waterfall data from state
    let waterfall_data = state.spectrum.get_waterfall_display();

    if waterfall_data.is_empty() {
        // Show placeholder if no data
        let text = Paragraph::new("Waiting for signal data...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(text, area);
    } else {
        // Render actual waterfall
        let widget = super::widgets::WaterfallWidget::new(waterfall_data)
            .block(block)
            .db_range(-100.0, 0.0);
        f.render_widget(widget, area);
    }
}

/// Render controls panel
fn render_controls(f: &mut Frame, app: &App, area: Rect) {
    let selected = app.state.read().ui.selected_control;
    let freq = app.get_frequency();
    let mode = app.get_mode();
    let gain = app.get_gain();
    let sample_rate = app.get_sample_rate();
    let is_recording = app.is_recording();

    let gain_str = if gain == -1 {
        "Auto".to_string()
    } else {
        format!("{}.{} dB", gain / 10, gain % 10)
    };

    let controls_text = vec![
        create_control_line(
            "Frequency:",
            format!("{:.3} MHz", freq as f64 / 1_000_000.0),
            selected == ControlId::Frequency,
        ),
        create_control_line(
            "Mode:",
            mode.name(),
            selected == ControlId::Mode,
        ),
        create_control_line(
            "Gain:",
            gain_str,
            selected == ControlId::Gain,
        ),
        create_control_line(
            "Sample Rate:",
            format!("{:.3} MHz", sample_rate as f64 / 1_000_000.0),
            selected == ControlId::SampleRate,
        ),
        Line::from(""),
        create_control_line(
            "Record:",
            if is_recording { "[ACTIVE]" } else { "[Press R]" },
            selected == ControlId::Record,
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("Controls:", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Green)),
            Span::raw(" - Next control  "),
        ]),
        Line::from(vec![
            Span::styled("↑↓←→/hjkl", Style::default().fg(Color::Green)),
            Span::raw(" - Adjust value"),
        ]),
        Line::from(vec![
            Span::styled("1-9,0", Style::default().fg(Color::Green)),
            Span::raw(" - Freq presets"),
        ]),
        Line::from(vec![
            Span::styled("Q", Style::default().fg(Color::Green)),
            Span::raw(" - Quit  "),
            Span::styled("R", Style::default().fg(Color::Green)),
            Span::raw(" - Record"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Presets:", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("1", Style::default().fg(Color::Cyan)),
            Span::raw(" APRS-NA  "),
            Span::styled("2", Style::default().fg(Color::Cyan)),
            Span::raw(" APRS-EU"),
        ]),
        Line::from(vec![
            Span::styled("3-9", Style::default().fg(Color::Cyan)),
            Span::raw(" NOAA 162.4-162.55 MHz"),
        ]),
        Line::from(vec![
            Span::styled("0", Style::default().fg(Color::Cyan)),
            Span::raw(" ADS-B (1090 MHz)"),
        ]),
    ];

    let paragraph = Paragraph::new(controls_text)
        .block(Block::default().title("Controls").borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

/// Create a control line with optional highlighting
fn create_control_line(label: impl Into<String>, value: impl Into<String>, selected: bool) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Line::from(vec![
        Span::styled(
            if selected { "> " } else { "  " }.to_string(),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(format!("{:15}", label.into()), style),
        Span::styled(value.into(), style.fg(Color::Cyan)),
    ])
}

/// Render decoder output placeholder
fn render_decoder_placeholder(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title("Decoder Output")
        .borders(Borders::ALL);

    let text = Paragraph::new("Decoded messages (APRS, ADS-B, etc.) will appear here")
        .block(block)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(text, area);
}
