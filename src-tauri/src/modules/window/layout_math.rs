use serde::{Deserialize, Serialize};

/// Represents display scaling factor (e.g. 1.0 for 100%, 1.25 for 125%, 1.5 for 150%, 2.0 for 200%).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DpiScale(pub f32);

impl Default for DpiScale {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Padding caused by Windows 10/11 invisible drop-shadow borders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InvisibleBorderPadding {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// Monitor work area rectangle excluding taskbars or docked app bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
}

/// A dimension specified either as a percentage of the work area or in explicit pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "unit", content = "value", rename_all = "snake_case")]
pub enum Dimension {
    Percent(f32),
    Pixels(i32),
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Percent(50.0)
    }
}

/// Presets for standard tiling layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PositionPreset {
    #[default]
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    LeftTwoThirds,
    RightOneThird,
    LeftOneThird,
    RightTwoThirds,
    Custom,
}

/// Complete placement specification for a window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowPlacement {
    pub monitor_index: Option<usize>,
    pub preset: PositionPreset,
    #[serde(default)]
    pub x: Dimension,
    #[serde(default)]
    pub y: Dimension,
    #[serde(default)]
    pub width: Dimension,
    #[serde(default)]
    pub height: Dimension,
    #[serde(default)]
    pub is_maximized: bool,
    #[serde(default)]
    pub is_fullscreen: bool,
}

impl Default for WindowPlacement {
    fn default() -> Self {
        Self {
            monitor_index: None,
            preset: PositionPreset::LeftHalf,
            x: Dimension::Percent(0.0),
            y: Dimension::Percent(0.0),
            width: Dimension::Percent(50.0),
            height: Dimension::Percent(100.0),
            is_maximized: false,
            is_fullscreen: false,
        }
    }
}

impl WindowPlacement {
    pub fn preset(preset: PositionPreset) -> Self {
        let (x, y, w, h) = match preset {
            PositionPreset::LeftHalf => (0.0, 0.0, 50.0, 100.0),
            PositionPreset::RightHalf => (50.0, 0.0, 50.0, 100.0),
            PositionPreset::TopHalf => (0.0, 0.0, 100.0, 50.0),
            PositionPreset::BottomHalf => (0.0, 50.0, 100.0, 50.0),
            PositionPreset::LeftTwoThirds => (0.0, 0.0, 66.66667, 100.0),
            PositionPreset::RightOneThird => (66.66667, 0.0, 33.33333, 100.0),
            PositionPreset::LeftOneThird => (0.0, 0.0, 33.33333, 100.0),
            PositionPreset::RightTwoThirds => (33.33333, 0.0, 66.66667, 100.0),
            PositionPreset::Custom => (0.0, 0.0, 50.0, 100.0),
        };

        Self {
            monitor_index: None,
            preset,
            x: Dimension::Percent(x),
            y: Dimension::Percent(y),
            width: Dimension::Percent(w),
            height: Dimension::Percent(h),
            is_maximized: false,
            is_fullscreen: false,
        }
    }
}

/// The calculated physical Win32 bounding box for `SetWindowPos`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculatedRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Calculates the target window rectangle based on work area, placement, scaling, and DWM padding.
pub fn calculate_window_rect(
    work_area: &MonitorWorkArea,
    placement: &WindowPlacement,
    _dpi: DpiScale,
    border_padding: &InvisibleBorderPadding,
) -> CalculatedRect {
    let (norm_x, norm_y, norm_w, norm_h) = if placement.preset != PositionPreset::Custom {
        match placement.preset {
            PositionPreset::LeftHalf => (0.0, 0.0, 0.5, 1.0),
            PositionPreset::RightHalf => (0.5, 0.0, 0.5, 1.0),
            PositionPreset::TopHalf => (0.0, 0.0, 1.0, 0.5),
            PositionPreset::BottomHalf => (0.0, 0.5, 1.0, 0.5),
            PositionPreset::LeftTwoThirds => (0.0, 0.0, 0.65, 1.0),
            PositionPreset::RightOneThird => (0.65, 0.0, 0.35, 1.0),
            PositionPreset::LeftOneThird => (0.0, 0.0, 0.35, 1.0),
            PositionPreset::RightTwoThirds => (0.35, 0.0, 0.65, 1.0),
            PositionPreset::Custom => (0.0, 0.0, 0.5, 1.0),
        }
    } else {
        (
            resolve_dim_fraction(placement.x, work_area.width as f32),
            resolve_dim_fraction(placement.y, work_area.height as f32),
            resolve_dim_fraction(placement.width, work_area.width as f32),
            resolve_dim_fraction(placement.height, work_area.height as f32),
        )
    };

    // Calculate unadjusted visible bounds in monitor space
    let visible_w = (norm_w * work_area.width as f32).round() as i32;
    let visible_h = (norm_h * work_area.height as f32).round() as i32;
    let visible_x = work_area.x + (norm_x * work_area.width as f32).round() as i32;
    let visible_y = work_area.y + (norm_y * work_area.height as f32).round() as i32;

    // Compensate for invisible drop-shadow borders:
    // Window bounding box must be expanded by border padding so that the VISIBLE frame
    // exactly hits the target rect!
    let win_x = visible_x - border_padding.left;
    let win_y = visible_y - border_padding.top;
    let win_w = visible_w + border_padding.left + border_padding.right;
    let win_h = visible_h + border_padding.top + border_padding.bottom;

    CalculatedRect {
        x: win_x,
        y: win_y,
        width: win_w,
        height: win_h,
    }
}

fn resolve_dim_fraction(dim: Dimension, total_pixels: f32) -> f32 {
    match dim {
        Dimension::Percent(pct) => (pct / 100.0).clamp(0.0, 1.0),
        Dimension::Pixels(px) => {
            if total_pixels <= 0.0 {
                0.0
            } else {
                (px as f32 / total_pixels).clamp(0.0, 1.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_50_50_split_no_overlap_no_gap() {
        let work = MonitorWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040, // 1080 minus 40px taskbar
            is_primary: true,
        };
        let padding = InvisibleBorderPadding::default();
        let dpi = DpiScale(1.0);

        let left_rect = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::LeftHalf),
            dpi,
            &padding,
        );
        let right_rect = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::RightHalf),
            dpi,
            &padding,
        );

        assert_eq!(left_rect.x, 0);
        assert_eq!(left_rect.width, 960);
        assert_eq!(right_rect.x, 960);
        assert_eq!(right_rect.width, 960);

        // Ensure left + right exactly equals total width (no gap, no overlap)
        assert_eq!(left_rect.width + right_rect.width, work.width);
        assert_eq!(left_rect.x + left_rect.width, right_rect.x);
    }

    #[test]
    fn test_65_35_split_no_overlap_no_gap() {
        let work = MonitorWorkArea {
            x: 0,
            y: 0,
            width: 2560,
            height: 1400,
            is_primary: true,
        };
        let padding = InvisibleBorderPadding::default();
        let dpi = DpiScale(1.25); // 125% DPI

        let left = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::LeftTwoThirds),
            dpi,
            &padding,
        );
        let right = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::RightOneThird),
            dpi,
            &padding,
        );

        // Left 65% of 2560 = 1664
        // Right 35% of 2560 = 896
        assert_eq!(left.width, 1664);
        assert_eq!(right.width, 896);
        assert_eq!(left.width + right.width, 2560);
        assert_eq!(left.x + left.width, right.x);
    }

    #[test]
    fn test_border_compensation() {
        let work = MonitorWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
        };
        // Typical Windows 11 drop-shadow padding: 7px left/right/bottom, 0 top
        let padding = InvisibleBorderPadding {
            left: 7,
            right: 7,
            top: 0,
            bottom: 7,
        };
        let dpi = DpiScale(1.0);

        let rect = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::LeftHalf),
            dpi,
            &padding,
        );

        // Visible x=0 -> Win32 x = -7
        assert_eq!(rect.x, -7);
        // Visible w=960 -> Win32 w = 960 + 7 + 7 = 974
        assert_eq!(rect.width, 974);
        // Visible h=1080 -> Win32 h = 1080 + 0 + 7 = 1087
        assert_eq!(rect.height, 1087);
    }

    #[test]
    fn test_work_area_offset_secondary_monitor() {
        // Secondary monitor placed to the left or right of primary
        let work = MonitorWorkArea {
            x: 1920,
            y: 50, // Docked top bar
            width: 1920,
            height: 1030,
            is_primary: false,
        };
        let padding = InvisibleBorderPadding::default();
        let dpi = DpiScale(1.5); // 150% DPI

        let rect = calculate_window_rect(
            &work,
            &WindowPlacement::preset(PositionPreset::LeftHalf),
            dpi,
            &padding,
        );

        assert_eq!(rect.x, 1920);
        assert_eq!(rect.y, 50);
        assert_eq!(rect.width, 960);
        assert_eq!(rect.height, 1030);
    }

    #[test]
    fn test_scaling_factors_clamping() {
        let scales = [1.0, 1.25, 1.5, 2.0];
        let work = MonitorWorkArea {
            x: 0,
            y: 0,
            width: 3840,
            height: 2160,
            is_primary: true,
        };
        let padding = InvisibleBorderPadding::default();

        for s in scales {
            let dpi = DpiScale(s);
            let left = calculate_window_rect(
                &work,
                &WindowPlacement::preset(PositionPreset::LeftHalf),
                dpi,
                &padding,
            );
            let right = calculate_window_rect(
                &work,
                &WindowPlacement::preset(PositionPreset::RightHalf),
                dpi,
                &padding,
            );

            assert_eq!(left.width + right.width, 3840);
            assert_eq!(left.x + left.width, right.x);
        }
    }
}

