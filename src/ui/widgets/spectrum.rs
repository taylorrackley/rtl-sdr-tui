use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

/// Spectrum analyzer widget that displays FFT data as a line chart
pub struct SpectrumWidget<'a> {
    /// FFT magnitude data in dB
    data: &'a [f32],
    /// Center frequency in Hz
    center_freq: u32,
    /// Sample rate in Hz
    sample_rate: u32,
    /// Block to wrap the widget
    block: Option<Block<'a>>,
    /// Minimum dB value for display
    min_db: f32,
    /// Maximum dB value for display
    max_db: f32,
}

impl<'a> SpectrumWidget<'a> {
    /// Create a new spectrum widget
    pub fn new(data: &'a [f32], center_freq: u32, sample_rate: u32) -> Self {
        Self {
            data,
            center_freq,
            sample_rate,
            block: None,
            min_db: -100.0,
            max_db: 0.0,
        }
    }

    /// Set the block for the widget
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the dB range for display
    pub fn db_range(mut self, min: f32, max: f32) -> Self {
        self.min_db = min;
        self.max_db = max;
        self
    }
}

impl Widget for SpectrumWidget<'_> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if area.width < 2 || area.height < 2 {
            return;
        }

        // If no data, show placeholder
        if self.data.is_empty() {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;

        // Downsample or interpolate data to fit width
        let displayed_data = resample_data(self.data, width);

        // Convert dB values to pixel heights
        let pixel_heights: Vec<usize> = displayed_data
            .iter()
            .map(|&db| {
                let normalized = ((db - self.min_db) / (self.max_db - self.min_db))
                    .max(0.0)
                    .min(1.0);
                ((height - 1) as f32 * normalized) as usize
            })
            .collect();

        // Draw the spectrum using vertical bars
        for (x, &pixel_height) in pixel_heights.iter().enumerate() {
            if x >= width {
                break;
            }

            // Determine color based on signal strength
            let color = get_signal_color(pixel_height, height);

            // Draw vertical line from bottom to pixel_height
            for y_offset in 0..=pixel_height.min(height - 1) {
                let y = area.bottom() - 1 - y_offset as u16;
                if y >= area.top() && y < area.bottom() {
                    let x_pos = area.left() + x as u16;
                    buf.get_mut(x_pos, y)
                        .set_char('▁')
                        .set_fg(color);
                }
            }
        }

        // Draw frequency labels (if space allows)
        if area.height > 3 {
            draw_frequency_labels(
                buf,
                area,
                self.center_freq,
                self.sample_rate,
            );
        }
    }
}

/// Resample data to fit the target width
fn resample_data(data: &[f32], target_width: usize) -> Vec<f32> {
    if data.is_empty() {
        return vec![];
    }

    if data.len() == target_width {
        return data.to_vec();
    }

    let mut result = Vec::with_capacity(target_width);
    let ratio = data.len() as f32 / target_width as f32;

    for i in 0..target_width {
        let src_pos = i as f32 * ratio;
        let src_idx = src_pos as usize;

        if src_idx >= data.len() {
            result.push(data[data.len() - 1]);
        } else if src_idx + 1 >= data.len() {
            result.push(data[src_idx]);
        } else {
            // Linear interpolation
            let frac = src_pos - src_idx as f32;
            let value = data[src_idx] * (1.0 - frac) + data[src_idx + 1] * frac;
            result.push(value);
        }
    }

    result
}

/// Get color based on signal strength
fn get_signal_color(pixel_height: usize, max_height: usize) -> Color {
    let ratio = pixel_height as f32 / max_height as f32;

    if ratio > 0.8 {
        Color::Red
    } else if ratio > 0.6 {
        Color::Yellow
    } else if ratio > 0.4 {
        Color::Green
    } else if ratio > 0.2 {
        Color::Cyan
    } else {
        Color::Blue
    }
}

/// Draw frequency labels at the bottom
fn draw_frequency_labels(buf: &mut Buffer, area: Rect, center_freq: u32, sample_rate: u32) {
    if area.width < 20 {
        return; // Not enough space for labels
    }

    let y = area.bottom() - 1;

    // Calculate frequency range
    let bandwidth = sample_rate as f32;
    let start_freq = center_freq as f32 - bandwidth / 2.0;
    let end_freq = center_freq as f32 + bandwidth / 2.0;

    // Draw labels at left, center, and right
    let positions = [
        (area.left(), start_freq),
        (area.left() + area.width / 2, center_freq as f32),
        (area.right() - 10, end_freq),
    ];

    for (x, freq_hz) in positions {
        let freq_mhz = freq_hz / 1_000_000.0;
        let label = format!("{:.2}", freq_mhz);

        for (i, ch) in label.chars().enumerate() {
            let x_pos = x + i as u16;
            if x_pos < area.right() {
                buf.get_mut(x_pos, y)
                    .set_char(ch)
                    .set_fg(Color::Gray);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_data() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Downsample
        let resampled = resample_data(&data, 3);
        assert_eq!(resampled.len(), 3);

        // Upsample
        let resampled = resample_data(&data, 10);
        assert_eq!(resampled.len(), 10);

        // Same size
        let resampled = resample_data(&data, 5);
        assert_eq!(resampled, data);
    }

    #[test]
    fn test_get_signal_color() {
        assert_eq!(get_signal_color(90, 100), Color::Red);
        assert_eq!(get_signal_color(70, 100), Color::Yellow);
        assert_eq!(get_signal_color(50, 100), Color::Green);
        assert_eq!(get_signal_color(30, 100), Color::Cyan);
        assert_eq!(get_signal_color(10, 100), Color::Blue);
    }
}
