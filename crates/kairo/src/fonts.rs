//! The four typographic voices of Kairo and their fonts, embedded in the binary.
//!
//! The files come from `google/fonts` (`ofl/` folders), under the OFL 1.1 licence: see
//! `assets/ATTRIBUTION.md` at the root of the repository. Nothing is loaded at runtime.

use std::sync::Arc;

use egui::epaint::text::{FontData, FontDefinitions};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId};

use crate::tokens;

const SHIPPORI_MINCHO_B1_800: &[u8] = include_bytes!("../assets/fonts/shipporiminchob1/ShipporiMinchoB1-ExtraBold.ttf");
const BARLOW_CONDENSED_600: &[u8] = include_bytes!("../assets/fonts/barlowcondensed/BarlowCondensed-SemiBold.ttf");
const BARLOW_CONDENSED_700: &[u8] = include_bytes!("../assets/fonts/barlowcondensed/BarlowCondensed-Bold.ttf");
const SHARE_TECH_MONO: &[u8] = include_bytes!("../assets/fonts/sharetechmono/ShareTechMono-Regular.ttf");
const BARLOW_400: &[u8] = include_bytes!("../assets/fonts/barlow/Barlow-Regular.ttf");
const ZEN_KAKU_GOTHIC_NEW_400: &[u8] = include_bytes!("../assets/fonts/zenkakugothicnew/ZenKakuGothicNew-Regular.ttf");

const KEY_MINCHO: &str = "kairo-shippori-mincho-b1-800";
const KEY_CONDENSED_600: &str = "kairo-barlow-condensed-600";
const KEY_CONDENSED_700: &str = "kairo-barlow-condensed-700";
const KEY_MONO: &str = "kairo-share-tech-mono";
const KEY_BARLOW: &str = "kairo-barlow-400";
const KEY_ZEN: &str = "kairo-zen-kaku-gothic-new-400";

/// A typographic voice of Kairo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voice {
    /// `--font-display`: heavy mincho (Shippori Mincho B1 800), for dramatic titles.
    Display,
    /// `--font-label` at weight 600: condensed (Barlow Condensed), for labels.
    Label,
    /// `--font-label` at weight 700: Panel titles, hazard stripes.
    LabelStrong,
    /// `--font-mono`: Share Tech Mono, for figures, codes and logs.
    Mono,
    /// `--font-body`: Barlow, for prose.
    Body,
    /// `--font-jp-sans`: Zen Kaku Gothic New, for the Japanese dressing.
    Jp,
}

impl Voice {
    const ALL: [Self; 6] = [Self::Display, Self::Label, Self::LabelStrong, Self::Mono, Self::Body, Self::Jp];

    fn name(self) -> &'static str {
        match self {
            Self::Display => "kairo-display",
            Self::Label => "kairo-label",
            Self::LabelStrong => "kairo-label-strong",
            Self::Mono => "kairo-mono",
            Self::Body => "kairo-body",
            Self::Jp => "kairo-jp",
        }
    }

    /// The fonts of the voice, in fallback order. Japanese always falls back to Zen Kaku Gothic
    /// New, as in the Kairo CSS stacks.
    fn stack(self) -> &'static [&'static str] {
        match self {
            Self::Display => &[KEY_MINCHO, KEY_ZEN],
            Self::Label => &[KEY_CONDENSED_600, KEY_ZEN],
            Self::LabelStrong => &[KEY_CONDENSED_700, KEY_ZEN],
            Self::Mono => &[KEY_MONO, KEY_ZEN],
            Self::Body => &[KEY_BARLOW, KEY_ZEN],
            Self::Jp => &[KEY_ZEN, KEY_BARLOW],
        }
    }

    /// The egui family of the voice. It only exists after [`install`].
    pub fn family(self) -> FontFamily {
        FontFamily::Name(Arc::from(self.name()))
    }

    /// The font of the voice at the given size, in points.
    pub fn font(self, size: f32) -> FontId {
        FontId::new(size, self.family())
    }
}

/// The Kairo font definitions: the six named families, and egui's `Proportional` and `Monospace`
/// families wired to the body and the mono.
pub fn definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::empty();
    for (key, bytes) in [
        (KEY_MINCHO, SHIPPORI_MINCHO_B1_800),
        (KEY_CONDENSED_600, BARLOW_CONDENSED_600),
        (KEY_CONDENSED_700, BARLOW_CONDENSED_700),
        (KEY_MONO, SHARE_TECH_MONO),
        (KEY_BARLOW, BARLOW_400),
        (KEY_ZEN, ZEN_KAKU_GOTHIC_NEW_400),
    ] {
        definitions.font_data.insert(key.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let stack = |voice: Voice| voice.stack().iter().map(|key| (*key).to_owned()).collect::<Vec<_>>();
    for voice in Voice::ALL {
        definitions.families.insert(voice.family(), stack(voice));
    }
    definitions.families.insert(FontFamily::Proportional, stack(Voice::Body));
    definitions.families.insert(FontFamily::Monospace, stack(Voice::Mono));
    definitions
}

/// Installs the Kairo fonts in the egui context. To be called once, at startup.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

/// A text in a single voice, with its letter spacing in `em` (see `tokens::TRACKING_*`).
pub fn text(content: impl Into<String>, voice: Voice, size: f32, letter_spacing_em: f32, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    append(&mut job, content, voice, size, letter_spacing_em, color);
    job
}

/// Appends a section to a text, to mix voices on one line.
pub fn append(job: &mut LayoutJob, content: impl Into<String>, voice: Voice, size: f32, letter_spacing_em: f32, color: Color32) {
    append_after(job, 0.0, content, voice, size, letter_spacing_em, color);
}

/// Like [`append`], after a gap of the given width.
pub fn append_after(job: &mut LayoutJob, gap: f32, content: impl Into<String>, voice: Voice, size: f32, letter_spacing_em: f32, color: Color32) {
    let format = TextFormat {
        font_id: voice.font(size),
        extra_letter_spacing: letter_spacing_em * size,
        color,
        valign: egui::Align::BOTTOM,
        ..Default::default()
    };
    job.append(&content.into(), gap.max(0.0), format);
}

/// A label: condensed 600, capitals, `--tracking-label` letter spacing.
pub fn label(content: &str, size: f32, color: Color32) -> LayoutJob {
    text(content.to_uppercase(), Voice::Label, size, tokens::TRACKING_LABEL, color)
}

/// A mono text, for figures and codes.
pub fn mono(content: impl Into<String>, size: f32, color: Color32) -> LayoutJob {
    text(content, Voice::Mono, size, 0.0, color)
}

/// A body text, in normal case.
pub fn body(content: impl Into<String>, size: f32, color: Color32) -> LayoutJob {
    text(content, Voice::Body, size, 0.0, color)
}

/// A Japanese dressing: Zen Kaku Gothic New, `--tracking-jp` letter spacing.
pub fn jp(content: impl Into<String>, size: f32, color: Color32) -> LayoutJob {
    text(content, Voice::Jp, size, tokens::TRACKING_JP, color)
}

/// The usual Kairo pair: a label in capitals followed by its Japanese dressing, smaller and paler
/// (`CORE SYNC 同期率`).
pub fn bilingual_label(content: &str, dressing: Option<&str>) -> LayoutJob {
    let mut job = label(content, 12.0, tokens::FG_2);
    if let Some(dressing) = dressing {
        // The gap of 8 between the two, minus the letter spacing that already follows the last capital.
        let gap = 8.0 - tokens::TRACKING_LABEL * 12.0;
        append_after(&mut job, gap, dressing, Voice::Jp, tokens::TEXT_XS, tokens::TRACKING_JP, tokens::FG_3);
    }
    job
}
