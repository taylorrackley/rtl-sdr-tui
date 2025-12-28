use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

/// Waterfall display widget that shows spectrum history over time
pub struct WaterfallWidget<'a> {
    /// Waterfall history data (oldest to newest)
    data: Vec<&'a Vec<f32>>,
    /// Block to wrap the widget
    block: Option<Block<'a>>,
    /// Minimum dB value for color mapping
    min_db: f32,
    /// Maximum dB value for color mapping
    max_db: f32,
}

impl<'a> WaterfallWidget<'a> {
    /// Create a new waterfall widget
    pub fn new(data: Vec<&'a Vec<f32>>) -> Self {
        Self {
            data,
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

    /// Set the dB range for color mapping
    pub fn db_range(mut self, min: f32, max: f32) -> Self {
        self.min_db = min;
        self.max_db = max;
        self
    }
}

impl Widget for WaterfallWidget<'_> {
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

        // If no data, return early
        if self.data.is_empty() {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;

        // Determine how many rows of history to display
        let rows_to_display = height.min(self.data.len());

        // Get the most recent rows
        let start_idx = if self.data.len() > rows_to_display {
            self.data.len() - rows_to_display
        } else {
            0
        };

        // Render each row of the waterfall (newest at bottom)
        for (row_idx, fft_data) in self.data[start_idx..].iter().enumerate() {
            let y = area.top() + row_idx as u16;

            // Resample FFT data to fit width
            let row_data = resample_waterfall_row(fft_data, width);

            // Draw each pixel in the row
            for (x, &db_value) in row_data.iter().enumerate() {
                if x >= width {
                    break;
                }

                let color = db_to_color(db_value, self.min_db, self.max_db);
                let x_pos = area.left() + x as u16;

                buf.get_mut(x_pos, y)
                    .set_char(' ')
                    .set_bg(color);
            }
        }
    }
}

/// Resample a single waterfall row to fit the target width
fn resample_waterfall_row(data: &[f32], target_width: usize) -> Vec<f32> {
    if data.is_empty() {
        return vec![-100.0; target_width];
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

/// Convert dB value to color (blue = weak, red = strong)
fn db_to_color(db: f32, min_db: f32, max_db: f32) -> Color {
    // Normalize to 0.0-1.0
    let normalized = ((db - min_db) / (max_db - min_db))
        .max(0.0)
        .min(1.0);

    // Map to color gradient: blue -> cyan -> green -> yellow -> red
    if normalized < 0.2 {
        // Very weak signal: dark blue
        Color::Rgb(0, 0, (normalized * 5.0 * 128.0) as u8 + 32)
    } else if normalized < 0.4 {
        // Weak signal: blue to cyan
        let t = (normalized - 0.2) * 5.0;
        Color::Rgb(
            0,
            (t * 128.0) as u8,
            128 + (t * 127.0) as u8,
        )
    } else if normalized < 0.6 {
        // Medium signal: cyan to green
        let t = (normalized - 0.4) * 5.0;
        Color::Rgb(
            (t * 64.0) as u8,
            128 + (t * 127.0) as u8,
            255 - (t * 255.0) as u8,
        )
    } else if normalized < 0.8 {
        // Strong signal: green to yellow
        let t = (normalized - 0.6) * 5.0;
        Color::Rgb(
            64 + (t * 191.0) as u8,
            255,
            0,
        )
    } else {
        // Very strong signal: yellow to red
        let t = (normalized - 0.8) * 5.0;
        Color::Rgb(
            255,
            255 - (t * 255.0) as u8,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_waterfall_row() {
        let data = vec![-80.0, -60.0, -40.0, -20.0];

        // Downsample
        let resampled = resample_waterfall_row(&data, 2);
        assert_eq!(resampled.len(), 2);

        // Upsample
        let resampled = resample_waterfall_row(&data, 8);
        assert_eq!(resampled.len(), 8);

        // Empty data
        let resampled = resample_waterfall_row(&[], 5);
        assert_eq!(resampled.len(), 5);
        assert_eq!(resampled[0], -100.0);
    }

    #[test]
    fn test_db_to_color() {
        // Test weak signal (blue-ish)
        let color = db_to_color(-100.0, -100.0, 0.0);
        match color {
            Color::Rgb(r, g, b) => {
                assert!(b > 0); // Should have blue component
            }
            _ => panic!("Expected RGB color"),
        }

        // Test strong signal (red-ish)
        let color = db_to_color(0.0, -100.0, 0.0);
        match color {
            Color::Rgb(r, g, b) => {
                assert!(r > 200); // Should be mostly red
            }
            _ => panic!("Expected RGB color"),
        }
    }
}
