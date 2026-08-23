// SPDX-License-Identifier: MIT
//! Shared Kanagawa themes and CSS utilities.

use gtk4 as gtk;

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
            .replace("%FG_DIM%", self.fg_dim)
            .replace("%FG%", self.fg)
            .replace("%GRID%", self.grid)
            .replace("%BG_RAISED%", self.bg_raised)
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

/// Helper to add a string as a CSS provider at `USER + 1`.
pub fn install_stylesheet(css: &str) -> Option<gtk::CssProvider> {
    let display = gtk::gdk::Display::default()?;
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
    Some(provider)
}
