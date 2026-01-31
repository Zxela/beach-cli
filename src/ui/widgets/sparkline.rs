//! Tide sparkline widget for inline visualization

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Block characters for different tide heights (8 levels)
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A sparkline widget showing tide heights over time
pub struct TideSparkline<'a> {
    /// Tide heights for each time slot
    heights: &'a [f64],
    /// Maximum tide height for normalization
    max_height: f64,
    /// Current position marker (index into heights)
    current_position: Option<usize>,
    /// Style for the sparkline
    style: Style,
    /// Style for the current position marker
    marker_style: Style,
}

impl<'a> TideSparkline<'a> {
    pub fn new(heights: &'a [f64], max_height: f64) -> Self {
        Self {
            heights,
            max_height,
            current_position: None,
            style: Style::default().fg(Color::Cyan),
            marker_style: Style::default().fg(Color::Yellow),
        }
    }

    pub fn current_position(mut self, pos: usize) -> Self {
        self.current_position = Some(pos);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    fn height_to_block(&self, height: f64) -> char {
        let normalized = (height / self.max_height).clamp(0.0, 1.0);
        let index = ((normalized * 7.0).round() as usize).min(7);
        BLOCKS[index]
    }
}

impl<'a> Widget for TideSparkline<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let width = area.width as usize;

        for (i, height) in self.heights.iter().take(width).enumerate() {
            let block = self.height_to_block(*height);
            let x = area.x + i as u16;
            let y = area.y;

            let style = if self.current_position == Some(i) {
                self.marker_style
            } else {
                self.style
            };

            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(block).set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_height_to_block_minimum() {
        let sparkline = TideSparkline::new(&[], 4.8);
        assert_eq!(sparkline.height_to_block(0.0), '▁');
    }

    #[test]
    fn test_height_to_block_maximum() {
        let sparkline = TideSparkline::new(&[], 4.8);
        assert_eq!(sparkline.height_to_block(4.8), '█');
    }

    #[test]
    fn test_height_to_block_mid() {
        let sparkline = TideSparkline::new(&[], 4.8);
        let block = sparkline.height_to_block(2.4); // 50%
        assert!(BLOCKS.contains(&block));
    }

    #[test]
    fn test_sparkline_creation() {
        let heights = vec![1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0];
        let sparkline = TideSparkline::new(&heights, 4.8)
            .current_position(3)
            .style(Style::default().fg(Color::Blue));

        assert_eq!(sparkline.heights.len(), 7);
        assert_eq!(sparkline.current_position, Some(3));
    }

    #[test]
    fn test_height_to_block_above_max_clamps() {
        let sparkline = TideSparkline::new(&[], 4.8);
        // Above max should clamp to maximum block
        assert_eq!(sparkline.height_to_block(10.0), '█');
    }

    #[test]
    fn test_height_to_block_negative_clamps() {
        let sparkline = TideSparkline::new(&[], 4.8);
        // Negative should clamp to minimum block
        assert_eq!(sparkline.height_to_block(-1.0), '▁');
    }
}
