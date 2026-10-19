//! The clipboard: read with `arboard`, written with `clipboard-win` in a single opening, so that
//! the text, the image (PNG and DIB) and the exclusion formats of a temporary write reach the clipboard
//! histories as one entry they ignore.

use clipboard_win::options::NoClear;
use clipboard_win::{Clipboard, raw};
use proofreadme_core::{ClipboardContent, Image, Mark, OsError};

/// How many times the clipboard is tried while another program holds it open.
const ATTEMPTS: usize = 10;

/// `CF_DIBV5`: a device-independent bitmap with an alpha channel.
const CF_DIBV5: u32 = 17;

/// The formats that keep a write out of the clipboard history and the cloud clipboard, each set to
/// a DWORD 0.
const EXCLUSIONS: [&str; 3] = ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];

fn failed(cause: impl std::fmt::Display) -> OsError {
    OsError::Clipboard(cause.to_string())
}

/// The text and the image of the clipboard. An image in a form that cannot be decoded counts as
/// another format (ADR 0002: only text and image are kept).
pub(crate) fn read() -> Result<ClipboardContent, OsError> {
    let mut clipboard = arboard::Clipboard::new().map_err(failed)?;
    let text = match clipboard.get_text() {
        Ok(text) => Some(text),
        Err(arboard::Error::ContentNotAvailable) => None,
        Err(error) => return Err(failed(error)),
    };
    let image = match clipboard.get_image() {
        Ok(image) => Some(Image { width: image.width, height: image.height, rgba: image.bytes.into_owned() }),
        Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => None,
        Err(error) => return Err(failed(error)),
    };
    Ok(ClipboardContent { text, image })
}

/// Empties the clipboard and puts the content on it, excluded from the histories if transient.
pub(crate) fn write(content: &ClipboardContent, mark: Mark) -> Result<(), OsError> {
    let image = content.image.as_ref().map(|image| Ok::<_, OsError>((png(image)?, dib_v5(image)?))).transpose()?;
    let png_format = clipboard_win::register_format("PNG").ok_or_else(|| failed("format PNG unavailable"))?;
    let _open = Clipboard::new_attempts(ATTEMPTS).map_err(failed)?;
    raw::empty().map_err(failed)?;
    if let Some((png, dib)) = image {
        raw::set_without_clear(png_format.get(), &png).map_err(failed)?;
        raw::set_without_clear(CF_DIBV5, &dib).map_err(failed)?;
    }
    if let Some(text) = &content.text {
        raw::set_string_with(text, NoClear).map_err(failed)?;
    }
    if mark == Mark::Transient {
        for name in EXCLUSIONS {
            let format = clipboard_win::register_format(name).ok_or_else(|| failed(format!("format {name} unavailable")))?;
            raw::set_without_clear(format.get(), &0u32.to_ne_bytes()).map_err(failed)?;
        }
    }
    Ok(())
}

/// The sequence number of the clipboard, which Windows increments at each change.
pub(crate) fn sequence_number() -> u64 {
    raw::seq_num().map_or(0, |number| u64::from(number.get()))
}

/// The image as a PNG file.
fn png(image: &Image) -> Result<Vec<u8>, OsError> {
    let width = u32::try_from(image.width).map_err(failed)?;
    let height = u32::try_from(image.height).map_err(failed)?;
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(&image.rgba).map_err(failed)?;
    writer.finish().map_err(failed)?;
    Ok(file)
}

/// The image as a `CF_DIBV5`: a BITMAPV5HEADER (32 bits, BGRA bit fields, sRGB) then the rows,
/// bottom-up.
fn dib_v5(image: &Image) -> Result<Vec<u8>, OsError> {
    const HEADER: u32 = 124;
    let row = image.width.checked_mul(4).ok_or_else(|| failed("image too wide"))?;
    let size = row.checked_mul(image.height).filter(|&size| size == image.rgba.len()).ok_or_else(|| failed("image of inconsistent size"))?;
    let width = i32::try_from(image.width).map_err(failed)?;
    let height = i32::try_from(image.height).map_err(failed)?;
    let mut dib = Vec::with_capacity(size.saturating_add(124));
    dib.extend_from_slice(&HEADER.to_le_bytes());
    dib.extend_from_slice(&width.to_le_bytes());
    dib.extend_from_slice(&height.to_le_bytes()); // positive: bottom-up
    dib.extend_from_slice(&1u16.to_le_bytes()); // planes
    dib.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    dib.extend_from_slice(&3u32.to_le_bytes()); // BI_BITFIELDS
    dib.extend_from_slice(&u32::try_from(size).map_err(failed)?.to_le_bytes());
    dib.extend_from_slice(&[0; 16]); // resolution, palette
    for mask in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000] {
        dib.extend_from_slice(&mask.to_le_bytes());
    }
    dib.extend_from_slice(&0x7352_4742u32.to_le_bytes()); // LCS_sRGB
    dib.extend_from_slice(&[0; 48]); // end points, gamma
    dib.extend_from_slice(&4u32.to_le_bytes()); // LCS_GM_IMAGES
    dib.extend_from_slice(&[0; 12]); // profile, reserved
    for line in image.rgba.chunks_exact(row.max(1)).rev() {
        for pixel in line.chunks_exact(4) {
            if let [red, green, blue, alpha] = *pixel {
                dib.extend_from_slice(&[blue, green, red, alpha]);
            }
        }
    }
    Ok(dib)
}
