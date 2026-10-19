//! The Kairo effects, approximated with what egui offers: scanlines, glows, hazard stripes,
//! stepped blinking, stretched mincho.
//!
//! Nothing here is identical to the CSS rendering (ADR 0001): the differences are described on
//! each function.

use std::sync::Arc;
use std::time::Duration;

use egui::epaint::{ClippedShape, Mesh, Primitive, Shadow};
use egui::{Color32, CornerRadius, Galley, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::fonts::Voice;
use crate::tokens::{self, Glow};

/// The glow of a box (`box-shadow: 0 0 <blur> <color>`), to be painted before the box itself.
///
/// Difference: egui tessellates a rectangle with a softened edge, not a Gaussian blur; the halo is
/// a little sharper than in CSS.
pub fn glow(painter: &Painter, rect: Rect, glow: Glow) {
    let blur = glow.blur.round().clamp(0.0, 255.0) as u8;
    let shadow = Shadow { offset: [0, 0], blur, spread: 0, color: glow.color };
    painter.add(shadow.as_shape(rect, CornerRadius::ZERO));
}

/// The glow of a text (`text-shadow: 0 0 <blur> <color>`), to be painted before the text.
///
/// Difference: egui cannot blur a text. The halo is approximated by offset, very transparent
/// copies of the text, on two rings; it is shorter and less diffuse than in CSS.
pub fn text_glow(painter: &Painter, position: Pos2, galley: &Arc<Galley>, glow: Glow) {
    const DIRECTIONS: [(f32, f32); 8] =
        [(1.0, 0.0), (0.707, 0.707), (0.0, 1.0), (-0.707, 0.707), (-1.0, 0.0), (-0.707, -0.707), (0.0, -1.0), (0.707, -0.707)];
    for (radius, opacity) in [(glow.blur * 0.10, 0.22), (glow.blur * 0.22, 0.10)] {
        let color = glow.color.gamma_multiply(opacity);
        for (dx, dy) in DIRECTIONS {
            painter.galley_with_override_text_color(position + vec2(dx, dy) * radius, Arc::clone(galley), color);
        }
    }
}

/// The scanlines (`--pattern-scanlines`) over a rectangle: a white line at 3.5% every 3.
///
/// Difference: the lines are laid out in logical points; on a high-density screen they span
/// several physical pixels, as in CSS.
pub fn scanlines(painter: &Painter, rect: Rect) {
    let color = Color32::WHITE.gamma_multiply(tokens::SCANLINE_ALPHA);
    let mut mesh = Mesh::default();
    for y in repetitions(rect.top(), rect.bottom(), tokens::SCANLINE_PERIOD) {
        let line = Rect::from_min_max(pos2(rect.left(), y), pos2(rect.right(), (y + tokens::SCANLINE_THICKNESS).min(rect.bottom())));
        mesh.add_colored_rect(line, color);
    }
    painter.add(Shape::mesh(mesh));
}

/// The largest number of repetitions of a pattern in an area: far more than any screen shows.
const MAX_REPETITIONS: f32 = 10_000.0;

/// The positions of a pattern repeated from `start` to `end`, one per period. Their number is
/// bounded, and zero if the area is not finite (an infinite rectangle, or an undefined one):
/// drawing a pattern never loops forever.
fn repetitions(start: f32, end: f32, period: f32) -> impl Iterator<Item = f32> {
    let count = ((end - start) / period).ceil();
    let count = if start.is_finite() && count.is_finite() && count > 0.0 { count.min(MAX_REPETITIONS) as u16 } else { 0 };
    (0..count).map(move |rank| start + f32::from(rank) * period)
}

/// The scanlines overlaid on the whole screen, above everything else. To be called once per frame.
pub fn fullscreen_scanlines(ctx: &egui::Context) {
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("kairo-scanlines"));
    let rect = ctx.content_rect();
    scanlines(&ctx.layer_painter(layer), rect);
}

/// The measuring grid (`--pattern-grid`): `K_INK_800` rules every 24.
pub fn grid(painter: &Painter, rect: Rect) {
    let stroke = Stroke::new(tokens::BORDER_HAIRLINE, tokens::K_INK_800);
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    for x in repetitions(rect.left(), rect.right(), tokens::GRID_STEP) {
        painter.vline(x + 0.5, rect.y_range(), stroke);
    }
    for y in repetitions(rect.top(), rect.bottom(), tokens::GRID_STEP) {
        painter.hline(rect.x_range(), y + 0.5, stroke);
    }
}

/// The hazard stripes (`--pattern-hazard*`): diagonals at -45°, 12 of colour then 12 of void.
pub fn hazard_stripes(painter: &Painter, rect: Rect, color: Color32) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    painter.rect_filled(rect, CornerRadius::ZERO, tokens::K_VOID);
    // A stripe of 12 measured perpendicularly takes up 12·√2 horizontally.
    let width = tokens::HAZARD_STRIPE * std::f32::consts::SQRT_2;
    let period = width * 2.0;
    let height = rect.height();
    for x in repetitions(rect.left() - height - period, rect.right(), period) {
        let stripe = vec![pos2(x, rect.bottom()), pos2(x + width, rect.bottom()), pos2(x + width + height, rect.top()), pos2(x + height, rect.top())];
        painter.add(Shape::convex_polygon(stripe, color, Stroke::NONE));
    }
}

/// The corner brackets: four brackets of 10, 2 thick, at the corners of an important area.
pub fn brackets(painter: &Painter, rect: Rect, color: Color32) {
    let (t, e) = (tokens::BRACKET_SIZE, tokens::BORDER_STRONG);
    for (corner, sx, sy) in
        [(rect.left_top(), 1.0, 1.0), (rect.right_top(), -1.0, 1.0), (rect.left_bottom(), 1.0, -1.0), (rect.right_bottom(), -1.0, -1.0)]
    {
        painter.rect_filled(Rect::from_two_pos(corner, corner + vec2(sx * t, sy * e)), CornerRadius::ZERO, color);
        painter.rect_filled(Rect::from_two_pos(corner, corner + vec2(sx * e, sy * t)), CornerRadius::ZERO, color);
    }
}

/// The focus ring: a cyan rule of 1, offset by 2 (`outline: 1px solid; outline-offset: 2px`).
pub fn focus_ring(painter: &Painter, rect: Rect) {
    painter.rect_stroke(rect.expand(2.0), CornerRadius::ZERO, Stroke::new(tokens::BORDER_HAIRLINE, tokens::FOCUS_RING), StrokeKind::Outside);
}

/// True during the first half of each blink period (`k-blink`, `steps(1)`): on, then off, with no
/// fade.
pub fn blink_is_on(time: f64) -> bool {
    let period = f64::from(tokens::BLINK_RATE);
    time.rem_euclid(period) < period / 2.0
}

/// The current state of the blink, and the repaint request for the next flip.
pub fn blink(ui: &Ui) -> bool {
    let time = ui.input(|input| input.time);
    let half = f64::from(tokens::BLINK_RATE) / 2.0;
    let remaining = half - time.rem_euclid(half);
    ui.ctx().request_repaint_after(Duration::from_secs_f64(remaining.max(0.01)));
    blink_is_on(time)
}

/// The height taken up by a line of stretched mincho of the given size: `--leading-tight` line
/// height, stretched by `--display-stretch`.
pub fn display_line_height(size: f32) -> f32 {
    size * tokens::LEADING_TIGHT * tokens::DISPLAY_STRETCH
}

/// Paints a line of heavy mincho stretched vertically (`--display-stretch: scaleY(1.2)`), the
/// bottom of the line resting on `bottom_left`. Returns the covered rectangle.
///
/// Difference: egui has no non-uniform transform. The text is tessellated at its nominal size then
/// its vertices are stretched by 1.2 vertically; the glyphs are therefore resampled by the GPU, a
/// little less sharp than a true `scaleY`.
pub fn display(painter: &Painter, bottom_left: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), Voice::Display.font(size), color);
    let height = display_line_height(size);
    let covered = Rect::from_min_max(pos2(bottom_left.x, bottom_left.y - height), pos2(bottom_left.x + galley.size().x, bottom_left.y));
    if galley.is_empty() {
        return covered;
    }
    // The text is centred in its nominal line, then the whole line is stretched from its bottom.
    let nominal = size * tokens::LEADING_TIGHT;
    let origin = pos2(bottom_left.x, bottom_left.y - nominal + (nominal - galley.size().y) / 2.0);
    let shape = ClippedShape { clip_rect: Rect::EVERYTHING, shape: Shape::galley(origin, galley, color) };
    for primitive in painter.ctx().tessellate(vec![shape], painter.pixels_per_point()) {
        if let Primitive::Mesh(mut mesh) = primitive.primitive {
            for vertex in &mut mesh.vertices {
                vertex.pos.y = bottom_left.y - (bottom_left.y - vertex.pos.y) * tokens::DISPLAY_STRETCH;
            }
            painter.add(Shape::mesh(mesh));
        }
    }
    covered
}

/// The width of a line of mincho of the given size, to align it before painting it.
pub fn display_width(painter: &Painter, text: &str, size: f32) -> f32 {
    painter.layout_no_wrap(text.to_owned(), Voice::Display.font(size), Color32::PLACEHOLDER).size().x
}

#[cfg(test)]
mod tests {
    use super::blink_is_on;

    #[test]
    fn the_blink_is_on_for_the_first_half_second_then_off() {
        assert!(blink_is_on(0.0));
        assert!(blink_is_on(0.49));
        assert!(!blink_is_on(0.5));
        assert!(!blink_is_on(0.99));
        assert!(blink_is_on(1.0));
        assert!(blink_is_on(7.25));
        assert!(!blink_is_on(7.75));
    }
}
