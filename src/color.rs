// SPDX-License-Identifier: MIT
//! Color conversion and redraw helpers for custom drawing (charts,
//! waveforms, spectrums): hex palette slots into GDK/cairo values, and a
//! palette-change redraw hook.

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

/// Parse a CSS hex color (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`,
/// case-insensitive) into 0.0-1.0 floats. Strict: only these forms, so chart
/// code gets deterministic values from the palette's hex strings.
fn hex_to_rgba01(hex: &str) -> Option<(f64, f64, f64, f64)> {
    let hex = hex.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let expand = |c: u8| -> Option<f64> {
        let v = (char::from(c).to_digit(16)? as f64) / 15.0;
        Some(v)
    };
    let pair = |c: u8, d: u8| -> Option<f64> {
        let v = char::from(c).to_digit(16)? * 16 + char::from(d).to_digit(16)?;
        Some(v as f64 / 255.0)
    };
    let bytes = hex.as_bytes();
    match bytes.len() {
        3 => Some((expand(bytes[0])?, expand(bytes[1])?, expand(bytes[2])?, 1.0)),
        4 => Some((
            expand(bytes[0])?,
            expand(bytes[1])?,
            expand(bytes[2])?,
            expand(bytes[3])?,
        )),
        6 => Some((
            pair(bytes[0], bytes[1])?,
            pair(bytes[2], bytes[3])?,
            pair(bytes[4], bytes[5])?,
            1.0,
        )),
        8 => Some((
            pair(bytes[0], bytes[1])?,
            pair(bytes[2], bytes[3])?,
            pair(bytes[4], bytes[5])?,
            pair(bytes[6], bytes[7])?,
        )),
        _ => None,
    }
}

/// Convert a hex color string (`#rrggbb`, or the palette's `&'static str`
/// slots) into a [`gdk::RGBA`] for GDK-side drawing and styling.
pub fn to_gdk_rgba(hex: &str) -> Option<gdk::RGBA> {
    let (r, g, b, a) = hex_to_rgba01(hex)?;
    Some(gdk::RGBA::new(r as f32, g as f32, b as f32, a as f32))
}

/// Convert a hex color string into the 0.0-1.0 RGB triple cairo's
/// `set_source_rgb` takes. Pair with `Context::set_source_rgba(r, g, b,
/// alpha)` when the chart needs translucency on top of a palette slot.
pub fn to_cairo_rgba(hex: &str) -> Option<(f64, f64, f64)> {
    let (r, g, b, _) = hex_to_rgba01(hex)?;
    Some((r, g, b))
}

/// Queue a redraw of `widget` whenever the portal's composed dark/light
/// state changes. The chart redraws through its own draw function, which
/// should read the current palette at draw time. The owner reference is
/// weak, so the hook dies with the widget.
pub fn redraw_on_theme_change(widget: &impl IsA<gtk4::Widget>) {
    let owned = widget.upcast_ref::<gtk4::Widget>();
    let weak = {
        let slot = glib::WeakRef::<gtk4::Widget>::new();
        slot.set(Some(owned));
        slot
    };
    crate::portal::connect_dark_changed(owned.upcast_ref::<glib::Object>(), move |_| {
        if let Some(w) = weak.upgrade() {
            w.queue_draw();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Palette;

    fn close(a: f64, b: f64) -> bool {
        // gdk::RGBA carries f32, so round-tripping through it costs about
        // 1e-8 relative; 1e-6 leaves that room without hiding real errors.
        (a - b).abs() < 1e-6
    }

    #[test]
    fn six_digit_hex_parses_exact() {
        let (r, g, b) = to_cairo_rgba("#c4746e").unwrap();
        // #c4746e = dragonRed
        assert!(close(r, 196.0 / 255.0));
        assert!(close(g, 116.0 / 255.0));
        assert!(close(b, 110.0 / 255.0));
    }

    #[test]
    fn three_digit_hex_expands() {
        let (r, g, b) = to_cairo_rgba("#F0a").unwrap();
        assert!(close(r, 1.0));
        assert!(close(g, 0.0));
        assert!(close(b, 170.0 / 255.0));
    }

    #[test]
    fn eight_digit_hex_carries_alpha_into_gdk() {
        let rgba = to_gdk_rgba("#18161680").unwrap();
        assert!(close(rgba.red() as f64, 24.0 / 255.0));
        assert!(close(rgba.green() as f64, 22.0 / 255.0));
        assert!(close(rgba.blue() as f64, 22.0 / 255.0));
        assert!(close(rgba.alpha() as f64, 128.0 / 255.0));
    }

    #[test]
    fn opaque_hex_gives_full_alpha() {
        let rgba = to_gdk_rgba("#393836").unwrap();
        assert!(close(rgba.alpha() as f64, 1.0));
    }

    #[test]
    fn malformed_hex_is_none_not_a_panic() {
        for bad in [
            "", "#", "#12", "#12345", "#1234567", "181616", "#hhhhhh", "red",
        ] {
            assert!(to_gdk_rgba(bad).is_none(), "{bad} must not parse");
        }
    }

    #[test]
    fn palette_slots_round_trip_through_both_conversions() {
        // Every palette hex is a strict #rrggbb string; both converters must
        // accept all of them (this is the contract the charts lean on).
        for palette in [Palette::dragon(), Palette::lotus()] {
            for hex in [
                palette.bg,
                palette.bg_window,
                palette.bg_view,
                palette.bg_header,
                palette.bg_card,
                palette.fg,
                palette.fg_dim,
                palette.heading,
                palette.accent,
                palette.on_accent,
                palette.grid,
                palette.warn,
                palette.err,
                palette.ok,
                palette.bg_raised,
            ] {
                assert!(to_gdk_rgba(hex).is_some(), "{hex} must parse to gdk");
                assert!(to_cairo_rgba(hex).is_some(), "{hex} must parse to cairo");
            }
        }
    }
}
