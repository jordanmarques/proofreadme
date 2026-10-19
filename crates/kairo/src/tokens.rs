//! The Kairo tokens, transcribed as they are from `tokens/*.css` of the design system.
//!
//! No value is invented here: each constant carries the name of its CSS variable. Lengths are in
//! egui points, which equal CSS pixels.

use egui::Color32;

// --- Colours (`tokens/colors.css`) ----------------------------------------------------------

// Neutrals: cold cathode-ray-tube blacks, warm paper white.
pub const K_VOID: Color32 = Color32::from_rgb(0x05, 0x05, 0x06);
pub const K_INK_950: Color32 = Color32::from_rgb(0x0b, 0x0c, 0x0f);
pub const K_INK_900: Color32 = Color32::from_rgb(0x11, 0x13, 0x18);
pub const K_INK_800: Color32 = Color32::from_rgb(0x19, 0x1c, 0x22);
pub const K_INK_700: Color32 = Color32::from_rgb(0x24, 0x28, 0x30);
pub const K_INK_600: Color32 = Color32::from_rgb(0x34, 0x3a, 0x45);
pub const K_INK_500: Color32 = Color32::from_rgb(0x4c, 0x54, 0x62);
pub const K_INK_400: Color32 = Color32::from_rgb(0x7d, 0x85, 0x94);
pub const K_INK_300: Color32 = Color32::from_rgb(0xa0, 0xa6, 0xb0);
pub const K_PAPER_DIM: Color32 = Color32::from_rgb(0xcd, 0xc8, 0xba);
pub const K_PAPER: Color32 = Color32::from_rgb(0xef, 0xeb, 0xe0);

// Signal: command and alert.
pub const K_ORANGE_900: Color32 = Color32::from_rgb(0x3d, 0x1c, 0x00);
pub const K_ORANGE_700: Color32 = Color32::from_rgb(0xb8, 0x4d, 0x00);
pub const K_ORANGE_500: Color32 = Color32::from_rgb(0xff, 0x6b, 0x00);
pub const K_ORANGE_400: Color32 = Color32::from_rgb(0xff, 0x8f, 0x33);
pub const K_RED_900: Color32 = Color32::from_rgb(0x3a, 0x0a, 0x10);
pub const K_RED_700: Color32 = Color32::from_rgb(0xa3, 0x10, 0x1f);
pub const K_RED_500: Color32 = Color32::from_rgb(0xec, 0x1c, 0x2e);
pub const K_AMBER_900: Color32 = Color32::from_rgb(0x3d, 0x2a, 0x00);
pub const K_AMBER_500: Color32 = Color32::from_rgb(0xff, 0xb8, 0x1c);

// Data: phosphor and terminal.
pub const K_PHOSPHOR_900: Color32 = Color32::from_rgb(0x06, 0x30, 0x1a);
pub const K_PHOSPHOR_700: Color32 = Color32::from_rgb(0x1e, 0xa8, 0x5a);
pub const K_PHOSPHOR_500: Color32 = Color32::from_rgb(0x44, 0xff, 0x94);
pub const K_CYAN_900: Color32 = Color32::from_rgb(0x06, 0x2f, 0x38);
pub const K_CYAN_500: Color32 = Color32::from_rgb(0x33, 0xe4, 0xff);
pub const K_VIOLET_900: Color32 = Color32::from_rgb(0x1e, 0x14, 0x38);
pub const K_VIOLET_500: Color32 = Color32::from_rgb(0x8a, 0x5c, 0xf0);
pub const K_LIME_500: Color32 = Color32::from_rgb(0xb2, 0xf2, 0x3f);

// Semantic aliases.
pub const BG_BASE: Color32 = K_VOID;
pub const BG_SUNKEN: Color32 = K_INK_950;
pub const BG_SURFACE: Color32 = K_INK_900;
pub const BG_RAISED: Color32 = K_INK_800;
pub const FG_1: Color32 = K_PAPER;
pub const FG_2: Color32 = K_INK_300;
pub const FG_3: Color32 = K_INK_400;
pub const FG_INVERSE: Color32 = K_VOID;
pub const LINE_1: Color32 = K_INK_700;
pub const LINE_2: Color32 = K_INK_600;
pub const LINE_STRONG: Color32 = K_INK_400;
pub const ACCENT: Color32 = K_ORANGE_500;
pub const ACCENT_HOVER: Color32 = K_ORANGE_400;
pub const ACCENT_MUTED: Color32 = K_ORANGE_900;
pub const ON_ACCENT: Color32 = K_VOID;
pub const STATUS_OK: Color32 = K_PHOSPHOR_500;
pub const STATUS_WARN: Color32 = K_AMBER_500;
pub const STATUS_ALERT: Color32 = K_RED_500;
pub const STATUS_INFO: Color32 = K_CYAN_500;
pub const STATUS_UNIT: Color32 = K_VIOLET_500;
pub const FOCUS_RING: Color32 = K_CYAN_500;

// --- Spacing (`tokens/spacing.css`) ---------------------------------------------------------

pub const SPACE_0: f32 = 0.0;
pub const SPACE_1: f32 = 2.0;
pub const SPACE_2: f32 = 4.0;
pub const SPACE_3: f32 = 6.0;
pub const SPACE_4: f32 = 8.0;
pub const SPACE_5: f32 = 12.0;
pub const SPACE_6: f32 = 16.0;
pub const SPACE_7: f32 = 24.0;
pub const SPACE_8: f32 = 32.0;
pub const SPACE_9: f32 = 48.0;
pub const SPACE_10: f32 = 64.0;
pub const SPACE_11: f32 = 96.0;
/// No rounded corner, ever.
pub const RADIUS_0: f32 = 0.0;
pub const BORDER_HAIRLINE: f32 = 1.0;
pub const BORDER_STRONG: f32 = 2.0;
pub const BRACKET_SIZE: f32 = 10.0;
pub const CONTROL_H_SM: f32 = 28.0;
pub const CONTROL_H_MD: f32 = 36.0;
pub const CONTROL_H_LG: f32 = 48.0;
pub const BAR_H: f32 = 48.0;

// --- Typography (`tokens/typography.css`) ---------------------------------------------------

pub const TEXT_MICRO: f32 = 10.0;
pub const TEXT_XS: f32 = 11.0;
pub const TEXT_SM: f32 = 13.0;
pub const TEXT_MD: f32 = 15.0;
pub const TEXT_LG: f32 = 18.0;
pub const TEXT_XL: f32 = 24.0;
pub const TEXT_2XL: f32 = 32.0;
pub const TEXT_3XL: f32 = 48.0;
pub const TEXT_4XL: f32 = 72.0;
pub const TEXT_5XL: f32 = 112.0;
pub const TEXT_6XL: f32 = 168.0;

pub const LEADING_TIGHT: f32 = 0.9;
pub const LEADING_SNUG: f32 = 1.2;
pub const LEADING_BODY: f32 = 1.55;

/// Letter spacings are in `em`: to be multiplied by the text size.
pub const TRACKING_LABEL: f32 = 0.14;
pub const TRACKING_WIDE: f32 = 0.3;
pub const TRACKING_ULTRA: f32 = 0.5;
pub const TRACKING_JP: f32 = 0.1;

/// `--display-stretch: scaleY(1.2)`: the title mincho is always stretched vertically.
pub const DISPLAY_STRETCH: f32 = 1.2;

// --- Effects (`tokens/effects.css`) ---------------------------------------------------------

/// A cathode-ray-tube glow: `0 0 <blur>px <color>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    /// The blur radius, in points.
    pub blur: f32,
    pub color: Color32,
}

/// `--glow-orange: 0 0 16px rgba(255,107,0,.45)`.
pub const GLOW_ORANGE: Glow = Glow { blur: 16.0, color: Color32::from_rgba_unmultiplied_const(255, 107, 0, 115) };
/// `--glow-red: 0 0 18px rgba(236,28,46,.5)`.
pub const GLOW_RED: Glow = Glow { blur: 18.0, color: Color32::from_rgba_unmultiplied_const(236, 28, 46, 128) };
/// `--glow-green: 0 0 14px rgba(68,255,148,.4)`.
pub const GLOW_GREEN: Glow = Glow { blur: 14.0, color: Color32::from_rgba_unmultiplied_const(68, 255, 148, 102) };

/// `--pattern-scanlines`: a white line at 3.5% opacity, 1 thick, every 3.
pub const SCANLINE_ALPHA: f32 = 0.035;
pub const SCANLINE_THICKNESS: f32 = 1.0;
pub const SCANLINE_PERIOD: f32 = 3.0;
/// `--pattern-grid`: measuring grid, `K_INK_800` rules every 24.
pub const GRID_STEP: f32 = 24.0;
/// `--pattern-hazard*`: stripes at -45°, 12 of colour then 12 of void.
pub const HAZARD_STRIPE: f32 = 12.0;

/// Durations, in seconds.
pub const DUR_FAST: f32 = 0.080;
pub const DUR_BASE: f32 = 0.160;
pub const DUR_SLOW: f32 = 0.320;
pub const BLINK_RATE: f32 = 1.0;
/// `--ease-snap: steps(3,end)`: transitions advance in three steps.
pub const EASE_SNAP_STEPS: u32 = 3;
