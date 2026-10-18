//! The drawing of the ProofReadMe icon.
//!
//! The drawing is original and produced by code: four corner brackets, like the brackets of the
//! Kairo panels, around a check mark. This crate has no dependency, so that it serves the
//! application (session icon), its build script (icon of the Windows executable) and `xtask`
//! (icon of the macOS bundle) alike.
//!
//! - [`glyph`]: the motif alone, in one colour, on a transparent background (session icon);
//! - [`application`]: the orange motif on a dark tile with rounded corners (application icon);
//! - [`ico`]: the application icon in the Windows `.ico` format.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

/// The Kairo command orange (`--k-orange-500`): the colour of the motif on the tile.
pub const ORANGE: [u8; 3] = [0xff, 0x6b, 0x00];
/// The background of the tile: the darkest Kairo ink (`--k-ink-950`).
pub const INK: [u8; 3] = [0x0b, 0x0c, 0x0f];

/// The number of points sampled per pixel, on each axis, to soften the edges.
const SUBSAMPLES: u32 = 4;

/// The largest side drawn, in pixels: that of the largest icon a system asks for. A larger side is
/// brought down to this one.
pub const MAX_SIDE: u32 = 1024;

/// The motif alone: a square image of `side` pixels ([`MAX_SIDE`] at most), in non-premultiplied
/// RGBA, in the given colour on a transparent background.
#[must_use]
pub fn glyph(side: u32, color: [u8; 3]) -> Vec<u8> {
    image(side, |x, y| is_in_glyph(x, y).then_some(color))
}

/// The application icon: a square image of `side` pixels ([`MAX_SIDE`] at most), in
/// non-premultiplied RGBA, where the orange motif sits on a dark tile with rounded corners.
///
/// `margin` is the share of the side left transparent on each side of the tile: macOS expects a
/// margin around its icons (about a tenth), Windows none.
#[must_use]
pub fn application(side: u32, margin: f32) -> Vec<u8> {
    /// The radius of the tile corners, as a share of its side.
    const RADIUS: f32 = 0.225;
    /// The share of the tile the motif takes up.
    const SCALE: f32 = 0.72;

    let margin = margin.clamp(0.0, 0.4);
    let tile = 1.0 - 2.0 * margin;
    image(side, |x, y| {
        // The point in the frame of the tile, then its distance to the centre (the tile is symmetric).
        let (u, v) = ((x - margin) / tile, (y - margin) / tile);
        let (du, dv) = ((u - 0.5).abs(), (v - 0.5).abs());
        if du > 0.5 || dv > 0.5 {
            return None;
        }
        // In the corner square, only the quarter disc belongs to the tile.
        let (out_u, out_v) = (du - (0.5 - RADIUS), dv - (0.5 - RADIUS));
        if out_u > 0.0 && out_v > 0.0 && out_u * out_u + out_v * out_v > RADIUS * RADIUS {
            return None;
        }
        let on_the_motif = is_in_glyph(0.5 + (u - 0.5) / SCALE, 0.5 + (v - 0.5) / SCALE);
        Some(if on_the_motif { ORANGE } else { INK })
    })
}

/// The application icon in the Windows `.ico` format, at the sizes Explorer, the taskbar and
/// Alt-Tab ask for: 16, 24, 32, 48, 64 and 256 pixels.
///
/// Each image is stored uncompressed (a 32-bit bitmap and its transparency mask): the form every
/// resource compiler accepts.
#[must_use]
pub fn ico() -> Vec<u8> {
    const SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
    /// The file header, then one directory entry per image.
    const HEADER: usize = 6;
    const ENTRY: usize = 16;

    let images: Vec<(u32, Vec<u8>)> = SIZES.iter().map(|&side| (side, icon_bitmap(side, &application(side, 0.0)))).collect();
    let mut file = Vec::new();
    file.extend_from_slice(&0_u16.to_le_bytes()); // reserved
    file.extend_from_slice(&1_u16.to_le_bytes()); // type: icon
    file.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut position = HEADER + ENTRY * images.len();
    for (side, image) in &images {
        // A side of 256 pixels is written 0.
        let declared_side = if *side >= 256 { 0 } else { *side as u8 };
        file.extend_from_slice(&[declared_side, declared_side, 0, 0]); // width, height, palette, reserved
        file.extend_from_slice(&1_u16.to_le_bytes()); // planes
        file.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
        file.extend_from_slice(&(image.len() as u32).to_le_bytes());
        file.extend_from_slice(&(position as u32).to_le_bytes());
        position += image.len();
    }
    for (_, image) in &images {
        file.extend_from_slice(image);
    }
    file
}

/// One image of an `.ico` file: the `BITMAPINFOHEADER` header, the pixels in BGRA from bottom to
/// top, then the mask at one bit per pixel (1 = transparent), also from bottom to top, each row
/// rounded up to 4 bytes.
fn icon_bitmap(side: u32, rgba: &[u8]) -> Vec<u8> {
    let width = side as usize;
    let mask_row = width.div_ceil(32) * 4;
    let mut image = Vec::with_capacity(40 + width * width * 4 + width * mask_row);
    image.extend_from_slice(&40_u32.to_le_bytes()); // header size
    image.extend_from_slice(&side.to_le_bytes()); // width
    image.extend_from_slice(&(2 * side).to_le_bytes()); // height: the image then its mask
    image.extend_from_slice(&1_u16.to_le_bytes()); // planes
    image.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
    image.extend_from_slice(&[0; 24]); // no compression; sizes, resolution and palette at 0
    let rows = || rgba.chunks_exact(width * 4).rev();
    for row in rows() {
        for &[red, green, blue, alpha] in row.as_chunks::<4>().0 {
            image.extend_from_slice(&[blue, green, red, alpha]);
        }
    }
    for row in rows() {
        let mut mask = vec![0_u8; mask_row];
        for (column, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] == 0 {
                mask[column / 8] |= 0b1000_0000 >> (column % 8);
            }
        }
        image.extend_from_slice(&mask);
    }
    image
}

/// A square image of `side` pixels in non-premultiplied RGBA, where `color_at` gives the colour at
/// each point of the unit square (`None`: transparent).
///
/// Each pixel is the average of 4 × 4 points.
fn image(side: u32, color_at: impl Fn(f32, f32) -> Option<[u8; 3]>) -> Vec<u8> {
    // Bounded, the side overflows none of the products that follow.
    let side = side.min(MAX_SIDE);
    let step = 1.0 / (side * SUBSAMPLES) as f32;
    let points = SUBSAMPLES * SUBSAMPLES;
    let mut pixels = Vec::with_capacity((side * side * 4) as usize);
    for row in 0..side {
        for column in 0..side {
            let mut covered = 0_u32;
            let mut sum = [0_u32; 3];
            for sub_row in 0..SUBSAMPLES {
                for sub_column in 0..SUBSAMPLES {
                    let x = ((column * SUBSAMPLES + sub_column) as f32 + 0.5) * step;
                    let y = ((row * SUBSAMPLES + sub_row) as f32 + 0.5) * step;
                    if let Some(color) = color_at(x, y) {
                        covered += 1;
                        for (total, component) in sum.iter_mut().zip(color) {
                            *total += u32::from(component);
                        }
                    }
                }
            }
            // The colour is the average of the covered points only (non-premultiplied).
            let average = |total: u32| total.checked_div(covered).unwrap_or(0).min(255) as u8;
            let alpha = (covered * 255 / points).min(255) as u8;
            pixels.extend_from_slice(&[average(sum[0]), average(sum[1]), average(sum[2]), alpha]);
        }
    }
    pixels
}

/// The glyph in the unit square: true if the point belongs to it.
fn is_in_glyph(x: f32, y: f32) -> bool {
    /// The margin left around the glyph.
    const MARGIN: f32 = 0.09;
    /// The stroke thickness of the brackets.
    const STROKE: f32 = 0.09;
    /// The length of each arm of a bracket.
    const ARM: f32 = 0.27;
    /// The three points of the check mark: the start of the short arm, the low point, the end of
    /// the long arm (y points down).
    const CHECK: [(f32, f32); 3] = [(0.30, 0.50), (0.44, 0.64), (0.70, 0.36)];
    /// The half thickness of the check mark.
    const HALF_CHECK: f32 = 0.075;

    // The brackets are symmetric about both axes: reason on the distance to the centre.
    let (dx, dy) = ((x - 0.5).abs(), (y - 0.5).abs());
    let edge = 0.5 - MARGIN;
    let in_the_frame = dx <= edge && dy <= edge;
    let on_the_stroke = dx >= edge - STROKE || dy >= edge - STROKE;
    let near_a_corner = dx >= edge - ARM && dy >= edge - ARM;
    let bracket = in_the_frame && on_the_stroke && near_a_corner;
    let check = CHECK.windows(2).any(|arm| match arm {
        [start, end] => distance_to_segment((x, y), *start, *end) <= HALF_CHECK,
        _ => false,
    });
    bracket || check
}

/// The distance from a point to the segment that joins `start` to `end`.
fn distance_to_segment(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let (along_x, along_y) = (end.0 - start.0, end.1 - start.1);
    let (to_x, to_y) = (point.0 - start.0, point.1 - start.1);
    let length_squared = along_x * along_x + along_y * along_y;
    let share = if length_squared > 0.0 { ((to_x * along_x + to_y * along_y) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    let (gap_x, gap_y) = (to_x - share * along_x, to_y - share * along_y);
    (gap_x * gap_x + gap_y * gap_y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pixel (x, y) of an image of the given side.
    fn pixel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        let start = ((y * side + x) * 4) as usize;
        [pixels[start], pixels[start + 1], pixels[start + 2], pixels[start + 3]]
    }

    fn alpha(pixels: &[u8], side: u32, x: u32, y: u32) -> u8 {
        pixel(pixels, side, x, y)[3]
    }

    #[test]
    fn the_glyph_is_an_rgba_square_in_the_requested_color_on_a_transparent_background() {
        let pixels = glyph(32, [0xff, 0x6a, 0x00]);

        assert_eq!(pixels.len(), 32 * 32 * 4);
        assert_eq!(alpha(&pixels, 32, 0, 0), 0, "the margin is transparent");
        assert_eq!(pixel(&pixels, 32, 16, 16), [0xff, 0x6a, 0x00, 255], "the check mark crosses the centre");
        assert_eq!(pixel(&pixels, 32, 14, 20), [0xff, 0x6a, 0x00, 255], "the low point of the check mark is solid");
        assert_eq!(pixel(&pixels, 32, 19, 14), [0xff, 0x6a, 0x00, 255], "the long arm of the check mark is solid");
        assert_eq!(alpha(&pixels, 32, 9, 22), 0, "below the short arm, nothing");
        assert_eq!(pixel(&pixels, 32, 4, 4), [0xff, 0x6a, 0x00, 255], "the corner of a bracket is solid");
        assert_eq!(alpha(&pixels, 32, 16, 4), 0, "the brackets do not meet");
    }

    #[test]
    fn an_oversized_side_is_brought_down_to_the_largest_side_drawn_without_crashing() {
        let expected = (MAX_SIDE * MAX_SIDE * 4) as usize;

        assert_eq!(glyph(u32::MAX, ORANGE).len(), expected);
        assert_eq!(application(70_000, 0.1).len(), expected);
        assert_eq!(glyph(0, ORANGE).len(), 0);
    }

    #[test]
    fn the_application_icon_carries_the_orange_glyph_on_a_dark_tile_with_rounded_corners() {
        let pixels = application(64, 0.0);

        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert_eq!(alpha(&pixels, 64, 0, 0), 0, "the corner of the tile is rounded");
        assert_eq!(pixel(&pixels, 64, 32, 2), [INK[0], INK[1], INK[2], 255], "the edge of the tile is dark and opaque");
        assert_eq!(pixel(&pixels, 64, 32, 32), [ORANGE[0], ORANGE[1], ORANGE[2], 255], "the check mark crosses the centre");
        assert_eq!(pixel(&pixels, 64, 29, 38), [ORANGE[0], ORANGE[1], ORANGE[2], 255], "the low point of the check mark is orange");
        assert_eq!(pixel(&pixels, 64, 32, 14), [INK[0], INK[1], INK[2], 255], "between the brackets, the tile");
        assert_eq!(pixel(&pixels, 64, 14, 14), [ORANGE[0], ORANGE[1], ORANGE[2], 255], "the corner of a bracket is orange");
    }

    #[test]
    fn the_requested_margin_stays_transparent_around_the_tile() {
        let pixels = application(100, 0.1);

        assert_eq!(alpha(&pixels, 100, 50, 5), 0, "the margin is transparent");
        assert_eq!(alpha(&pixels, 100, 50, 94), 0, "the margin is transparent");
        assert_eq!(pixel(&pixels, 100, 50, 12), [INK[0], INK[1], INK[2], 255], "the tile starts after the margin");
        assert_eq!(pixel(&pixels, 100, 50, 50), [ORANGE[0], ORANGE[1], ORANGE[2], 255]);
    }

    fn u16_at(bytes: &[u8], position: usize) -> u32 {
        u32::from(u16::from_le_bytes([bytes[position], bytes[position + 1]]))
    }

    fn u32_at(bytes: &[u8], position: usize) -> usize {
        u32::from_le_bytes([bytes[position], bytes[position + 1], bytes[position + 2], bytes[position + 3]]) as usize
    }

    #[test]
    fn the_ico_file_describes_one_image_per_size_and_lays_them_end_to_end() {
        let file = ico();

        // Header: reserved, type 1 (icon), number of images.
        assert_eq!(file[..6], [0, 0, 1, 0, 6, 0]);
        // (side, bytes of the image): a 40-byte header, 4 bytes per pixel, then the mask at one
        // bit per pixel, each row rounded up to 4 bytes.
        let expected = [
            (16, 40 + 16 * 16 * 4 + 16 * 4),
            (24, 40 + 24 * 24 * 4 + 24 * 4),
            (32, 40 + 32 * 32 * 4 + 32 * 4),
            (48, 40 + 48 * 48 * 4 + 48 * 8),
            (64, 40 + 64 * 64 * 4 + 64 * 8),
            (256, 40 + 256 * 256 * 4 + 256 * 32),
        ];
        let mut position = 6 + 16 * expected.len();
        for (rank, (side, size)) in expected.into_iter().enumerate() {
            let entry = 6 + 16 * rank;
            let declared_side = if side == 256 { 0 } else { side as u8 };
            assert_eq!(file[entry..entry + 4], [declared_side, declared_side, 0, 0], "side of image {side}");
            assert_eq!(u16_at(&file, entry + 4), 1, "planes");
            assert_eq!(u16_at(&file, entry + 6), 32, "bits per pixel");
            assert_eq!(u32_at(&file, entry + 8), size, "size of image {side}");
            assert_eq!(u32_at(&file, entry + 12), position, "position of image {side}");
            // The image header: 40 bytes, width, doubled height (image then mask).
            assert_eq!(u32_at(&file, position), 40);
            assert_eq!(u32_at(&file, position + 4), side);
            assert_eq!(u32_at(&file, position + 8), 2 * side);
            assert_eq!(u16_at(&file, position + 12), 1, "planes");
            assert_eq!(u16_at(&file, position + 14), 32, "bits per pixel");
            assert_eq!(u32_at(&file, position + 16), 0, "no compression");
            position += size;
        }
        assert_eq!(file.len(), position);
    }

    #[test]
    fn an_image_of_the_ico_file_is_in_bgra_from_bottom_to_top_with_its_transparency_mask() {
        let file = ico();

        // The first image (16 px) starts after the header and the six entries.
        let pixels = 6 + 16 * 6 + 40;
        let pixel = |x: usize, row_from_the_bottom: usize| &file[pixels + (row_from_the_bottom * 16 + x) * 4..][..4];
        assert_eq!(pixel(8, 7), [ORANGE[2], ORANGE[1], ORANGE[0], 255], "the check mark at the centre, in BGRA");
        assert_eq!(pixel(0, 0)[3], 0, "the rounded corner is transparent");
        // The mask follows the pixels: one bit per pixel, 1 = transparent, most significant bit on the left.
        let mask = pixels + 16 * 16 * 4;
        assert_eq!(file[mask] & 0b1000_0000, 0b1000_0000, "the bottom left corner is masked");
        assert_eq!(file[mask + 8 * 4 + 1] & 0b1000_0000, 0, "the centre is not");
    }

    #[test]
    #[ignore = "tool: writes the application icon (256 px, raw RGBA) to the file PROOFREADME_DRAWING, to look at it"]
    fn write_the_drawing() {
        let Ok(file) = std::env::var("PROOFREADME_DRAWING") else { return };
        std::fs::write(file, application(256, 0.1)).unwrap();
    }
}
