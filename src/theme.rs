// SPDX-License-Identifier: MIT
//! Shared Kanagawa themes and CSS utilities.

use gtk4 as gtk;

use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: &'static str,
    pub bg_window: &'static str,
    pub bg_view: &'static str,
    pub bg_header: &'static str,
    pub bg_card: &'static str,
    pub fg: &'static str,
    pub fg_dim: &'static str,
    pub heading: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
    pub grid: &'static str,
    pub warn: &'static str,
    pub err: &'static str,
    pub ok: &'static str,
    pub bg_raised: &'static str,
}

impl Palette {
    /// Kanagawa Dragon (dark).
    pub const fn dragon() -> Self {
        Self {
            bg: "#12120f",
            bg_window: "#181616",
            bg_view: "#12120f",
            bg_header: "#1d1c19",
            bg_card: "#1d1c19",
            fg: "#c5c9c5",
            fg_dim: "#a6a69c",
            heading: "#c8c093",
            accent: "#c4746e",
            on_accent: "#12120f",
            grid: "#393836",
            warn: "#c4b28a",
            err: "#c4746e",
            ok: "#87a987",
            bg_raised: "#282727",
        }
    }

    /// Kanagawa Lotus (light).
    pub const fn lotus() -> Self {
        Self {
            bg: "#f2ecbc",
            bg_window: "#e7dba0",
            bg_view: "#f2ecbc",
            bg_header: "#e5ddb0",
            bg_card: "#e5ddb0",
            fg: "#545464",
            fg_dim: "#8a8980",
            heading: "#43436c",
            accent: "#4d699b",
            on_accent: "#f2ecbc",
            grid: "#d5cea3",
            warn: "#836f4a",
            err: "#c84053",
            ok: "#6f894e",
            bg_raised: "#e0d6a0",
        }
    }

    /// Substitute tokens (e.g., `%BG_WINDOW%`) in a template string with hex colors.
    pub fn replace_tokens(&self, template: &str) -> String {
        template
            .replace("%BG_WINDOW%", self.bg_window)
            .replace("%BG_VIEW%", self.bg_view)
            .replace("%BG_HEADER%", self.bg_header)
            .replace("%BG_CARD%", self.bg_card)
            .replace("%BG_RAISED%", self.bg_raised)
            .replace("%BG%", self.bg)
            .replace("%FG_DIM%", self.fg_dim)
            .replace("%FG%", self.fg)
            .replace("%HEADING%", self.heading)
            .replace("%GRID%", self.grid)
            .replace("%ACCENT%", self.accent)
            .replace("%ON_ACCENT%", self.on_accent)
            .replace("%WARN%", self.warn)
            .replace("%ERR%", self.err)
            .replace("%OK%", self.ok)
    }

    /// Create CSS custom properties block (e.g. `--c-bg-window: #181616;`).
    pub fn to_css_custom_properties(&self) -> String {
        format!(
            ":root {{\n  --c-bg: {bg};\n  --c-bg-window: {bg_window};\n  --c-bg-view: {bg_view};\n  --c-bg-header: {bg_header};\n  \
             --c-bg-card: {bg_card};\n  --c-fg: {fg};\n  --c-fg-dim: {fg_dim};\n  --c-heading: {heading};\n  \
             --c-accent: {accent};\n  --c-on-accent: {on_accent};\n  --c-grid: {grid};\n  \
             --c-warn: {warn};\n  --c-err: {err};\n  --c-ok: {ok};\n  --c-bg-raised: {bg_raised};\n}}\n",
            bg = self.bg,
            bg_window = self.bg_window,
            bg_view = self.bg_view,
            bg_header = self.bg_header,
            bg_card = self.bg_card,
            fg = self.fg,
            fg_dim = self.fg_dim,
            heading = self.heading,
            accent = self.accent,
            on_accent = self.on_accent,
            grid = self.grid,
            warn = self.warn,
            err = self.err,
            ok = self.ok,
            bg_raised = self.bg_raised,
        )
    }
}

thread_local! {
    /// The provider this crate last installed per display, so a re-install on
    /// a theme switch replaces it instead of accumulating providers on the
    /// display. GTK objects are `!Send`; styling runs on the main thread.
    static INSTALLED: RefCell<Vec<(gtk::gdk::Display, gtk::CssProvider)>> =
        const { RefCell::new(Vec::new()) };
}

/// Replace the stylesheet this crate previously installed on the default
/// display (if any) with `css`, at `USER + 1`. Returns the new provider.
pub fn install_stylesheet(css: &str) -> Option<gtk::CssProvider> {
    let display = gtk::gdk::Display::default()?;
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        if let Some(pos) = installed.iter().position(|(d, _)| d == &display) {
            let (_, old) = installed.remove(pos);
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
        installed.push((display, provider.clone()));
    });
    Some(provider)
}

#[cfg(test)]
mod tests {
    use super::Palette;

    #[test]
    fn dragon_matches_kanagawa_dragon_reference_hexes() {
        let p = Palette::dragon();
        assert_eq!(p.bg_window, "#181616");
        assert_eq!(p.bg_raised, "#282727");
        assert_eq!(p.fg, "#c5c9c5");
        assert_eq!(p.heading, "#c8c093");
        assert_eq!(p.accent, "#c4746e");
        assert_eq!(p.warn, "#c4b28a");
        assert_eq!(p.err, "#c4746e");
        assert_eq!(p.ok, "#87a987");
    }

    #[test]
    fn lotus_is_light_and_distinct_from_dragon() {
        let d = Palette::dragon();
        let l = Palette::lotus();
        assert_ne!(l.bg_window, d.bg_window);
        assert_ne!(l.fg, d.fg);
        assert_ne!(l.accent, d.accent);
        // On-accent text tracks each palette's own background tone.
        assert_eq!(d.on_accent, d.bg_view);
        assert_eq!(l.on_accent, l.bg);
    }

    #[test]
    fn replace_tokens_substitutes_every_documented_token() {
        let out = Palette::dragon().replace_tokens(
            "%BG%|%BG_WINDOW%|%BG_VIEW%|%BG_HEADER%|%BG_CARD%|%FG_DIM%|%FG%|%HEADING%|%GRID%|%BG_RAISED%|%ACCENT%|%ON_ACCENT%|%WARN%|%ERR%|%OK%",
        );
        assert_eq!(
            out,
            "#12120f|#181616|#12120f|#1d1c19|#1d1c19|#a6a69c|#c5c9c5|#c8c093|#393836|#282727|#c4746e|#12120f|#c4b28a|#c4746e|#87a987"
        );
    }

    #[test]
    fn replace_tokens_covers_bg_and_heading() {
        // %BG% and %HEADING% had palette fields but no token mapping until
        // 1.0.3; the test that pinned their absence flipped with the fix.
        let out = Palette::dragon().replace_tokens("%BG% %HEADING% %FG%");
        assert_eq!(out, "#12120f #c8c093 #c5c9c5");
    }

    #[test]
    fn css_custom_properties_lists_every_palette_slot() {
        let css = Palette::dragon().to_css_custom_properties();
        assert!(css.starts_with(":root {"));
        assert!(css.ends_with("}\n"));
        for key in [
            "--c-bg:",
            "--c-bg-window:",
            "--c-bg-view:",
            "--c-bg-header:",
            "--c-bg-card:",
            "--c-fg:",
            "--c-fg-dim:",
            "--c-heading:",
            "--c-accent:",
            "--c-on-accent:",
            "--c-grid:",
            "--c-warn:",
            "--c-err:",
            "--c-ok:",
            "--c-bg-raised:",
        ] {
            assert!(css.contains(key), "missing {key} in generated CSS");
        }
        assert!(css.contains("--c-bg-window: #181616;"));
    }
}
