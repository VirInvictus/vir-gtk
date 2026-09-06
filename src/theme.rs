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
    /// The providers this crate last installed per (display, priority), so a
    /// re-install on a theme switch replaces its tier's provider instead of
    /// accumulating providers on the display. GTK objects are `!Send`; styling
    /// runs on the main thread.
    static INSTALLED: RefCell<Vec<(gtk::gdk::Display, u32, gtk::CssProvider)>> =
        const { RefCell::new(Vec::new()) };
}

fn install_at(css: &str, priority: u32) -> Option<gtk::CssProvider> {
    let display = gtk::gdk::Display::default()?;
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        if let Some(pos) = installed
            .iter()
            .position(|(d, p, _)| d == &display && p == &priority)
        {
            let (_, _, old) = installed.remove(pos);
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
        gtk::style_context_add_provider_for_display(&display, &provider, priority);
        installed.push((display, priority, provider.clone()));
    });
    Some(provider)
}

/// Replace the crate-tier stylesheet previously installed on the default
/// display (if any) with `css`, at `USER + 1`. Returns the new provider.
///
/// This tier carries the shared sheets this crate owns: [`base_css`], the
/// palette custom-properties block, and the dark/light re-splice on theme
/// switches. Application sheets belong on [`install_app_stylesheet`], which
/// sits one step higher so app rules always win regardless of install order.
pub fn install_stylesheet(css: &str) -> Option<gtk::CssProvider> {
    install_at(css, gtk::STYLE_PROVIDER_PRIORITY_USER + 1)
}

/// Replace the app-tier stylesheet previously installed on the default
/// display (if any) with `css`, at `USER + 2`. Returns the new provider.
///
/// The ladder is the override mechanism: the system `gtk.css` sits below
/// `USER`, the crate's sheets install at `USER + 1`, and the application's
/// own sheet installs here, so app-specific rules beat the shared base by
/// construction instead of by install timing. Call it after (and on every
/// theme switch alongside) [`install_stylesheet`]; each tier replaces only
/// its own previous provider.
pub fn install_app_stylesheet(css: &str) -> Option<gtk::CssProvider> {
    install_at(css, gtk::STYLE_PROVIDER_PRIORITY_USER + 2)
}

/// The shared flat, square base widget sheet: window chrome, headerbar,
/// lists and rows, the button family, entries, popovers, tooltips,
/// scrollbars, the Adwaita utility classes, and the scoped focus ring,
/// spliced with `palette`'s hexes so no `%TOKEN%` survives.
///
/// This is the unanimous core distilled from the consumer sheets (the
/// square, hard-1px-border idiom); deliberate per-app divergences (Atrium's
/// rounding, Conservatory's lifted selection, Viaduct's transparent lists)
/// are intentionally absent and belong in the application's own sheet,
/// installed via [`install_app_stylesheet`] so they override by priority.
/// The sheet carries no `font-family` rules and no `%TOKEN%` remains after
/// substitution, so literal CSS braces in the template are safe.
pub fn base_css(palette: &Palette) -> String {
    palette.replace_tokens(BASE_CSS_TEMPLATE)
}

const BASE_CSS_TEMPLATE: &str = "\
/* vir-gtk base sheet: the shared flat/square widget core, Kanagawa-spliced.
   Install at the crate tier (install_stylesheet); app sheets override via
   install_app_stylesheet (USER + 2). */
window, .background { background-color: %BG_WINDOW%; color: %FG%; }
window.csd, decoration { border-radius: 0; box-shadow: none; }
headerbar {
  background-color: %BG_HEADER%;
  background-image: none;
  color: %FG%;
  box-shadow: none;
  border-bottom: 1px solid %GRID%;
  min-height: 34px;
  padding: 0 4px;
}
headerbar button { min-height: 24px; }
paned > separator {
  background-color: %GRID%;
  background-image: none;
  min-width: 1px;
  min-height: 1px;
}
listview, list, columnview { background-color: %BG_VIEW%; color: %FG%; }
row { border-radius: 0; }
row.activatable:hover { background-color: alpha(currentColor, 0.06); }
.card, list.boxed-list {
  background-color: %BG_CARD%;
  color: %FG%;
  border: 1px solid %GRID%;
  border-radius: 0;
  box-shadow: none;
}
list.boxed-list > row { border-bottom: 1px solid %GRID%; }
list.boxed-list > row:last-child { border-bottom: none; }
button {
  background-color: %BG_CARD%;
  background-image: none;
  color: %FG%;
  border: 1px solid %GRID%;
  border-radius: 0;
  box-shadow: none;
  min-height: 24px;
  padding: 2px 10px;
}
button:hover { background-color: %GRID%; }
button:active, button:checked { background-color: %ACCENT%; color: %ON_ACCENT%; border-color: %ACCENT%; }
/* Insensitive controls must read as such: without this the flat sheet leaves
   a disabled button visually identical to a live one. */
button:disabled { color: %FG_DIM%; border-color: alpha(%GRID%, 0.5); background-color: transparent; }
button.flat, button.circular { background-color: transparent; border-color: transparent; box-shadow: none; }
button.flat:hover, button.circular:hover { background-color: %GRID%; }
button.suggested-action { background-color: %ACCENT%; color: %ON_ACCENT%; border-color: %ACCENT%; }
button.destructive-action { background-color: %ERR%; color: %ON_ACCENT%; border-color: %ERR%; }
.linked > button:not(:first-child) { border-left-width: 0; }
.toolbar { padding: 4px 6px; }
entry, spinbutton {
  background-color: %BG_VIEW%;
  color: %FG%;
  border: 1px solid %GRID%;
  border-radius: 0;
  box-shadow: none;
}
entry:focus-within, spinbutton:focus-within { border-color: %ACCENT%; }
spinbutton > button { border-width: 0; background-color: transparent; }
spinbutton > button:hover { background-color: %GRID%; }
dropdown > button { background-color: %BG_CARD%; }
popover > arrow { background-color: %BG_CARD%; }
popover > contents {
  background-color: %BG_CARD%;
  color: %FG%;
  border: 1px solid %GRID%;
  border-radius: 0;
  box-shadow: none;
  padding: 4px;
}
popover.menu modelbutton { border-radius: 0; padding: 5px 8px; }
modelbutton:hover { background-color: %ACCENT%; color: %ON_ACCENT%; }
popover.menu separator { background-color: %GRID%; min-height: 1px; margin: 4px 0; }
tooltip, tooltip.background {
  background-color: %BG_HEADER%;
  color: %FG%;
  border: 1px solid %GRID%;
  border-radius: 0;
  box-shadow: none;
  padding: 4px 8px;
}
scrollbar { background-color: transparent; }
scrollbar slider { background-color: %GRID%; border-radius: 0; min-width: 6px; min-height: 6px; }
scrollbar slider:hover { background-color: %FG_DIM%; }
selection { background-color: alpha(%ACCENT%, 0.35); color: %FG%; }
/* Utility classes the Adwaita stylesheet used to provide (weight / size /
   colour only; no font-family rules by design). */
.title-1 { font-weight: 800; font-size: 170%; }
.title-2 { font-weight: 800; font-size: 140%; }
.title-3 { font-weight: 700; font-size: 120%; }
.title-4 { font-weight: 700; font-size: 105%; }
.large-title { font-weight: 300; font-size: 200%; }
.heading { font-weight: 700; }
.caption { font-size: 82%; }
.caption-heading { font-weight: 700; font-size: 82%; }
.dim-label { color: %FG_DIM%; }
.success { color: %OK%; }
.warning { color: %WARN%; }
.error { color: %ERR%; }
.accent { color: %ACCENT%; }
.numeric { font-feature-settings: 'tnum'; }
/* Keyboard-focus ring, scoped to discrete interactive controls, NOT a
   universal `*`: pressing a bare modifier (a tiling workspace-switch chord)
   flips GTK into keyboard-focus-visible mode, and a `*` rule then outlines
   every widget in the focus chain at once, flashing the accent across the
   whole window. Rows show position via the selection background, so they
   need no outline. */
button:focus-visible,
entry:focus-visible,
spinbutton:focus-visible,
switch:focus-visible,
checkbutton:focus-visible,
check:focus-visible,
dropdown:focus-visible,
scale:focus-visible { outline: 1px solid %ACCENT%; outline-offset: -1px; }
";

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

    #[test]
    fn base_css_leaves_no_token_behind() {
        // Literal percent signs are legitimate CSS (font-size: 82%); what
        // must never survive is a %UPPERCASE_TOKEN% span.
        let has_raw_token = |css: &str| {
            css.char_indices().any(|(i, c)| {
                if c != '%' {
                    return false;
                }
                match css[i + 1..].find('%') {
                    Some(end) => {
                        let inner = &css[i + 1..i + 1 + end];
                        !inner.is_empty()
                            && inner.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                    }
                    None => false,
                }
            })
        };
        for palette in [Palette::dragon(), Palette::lotus()] {
            let css = super::base_css(&palette);
            assert!(
                !has_raw_token(&css),
                "raw token survived base_css substitution"
            );
        }
    }

    #[test]
    fn base_css_splices_the_given_palette() {
        let dark = super::base_css(&Palette::dragon());
        let light = super::base_css(&Palette::lotus());
        assert!(dark.contains("#181616"));
        assert!(light.contains("#e7dba0"));
        assert_ne!(dark, light);
    }

    #[test]
    fn base_css_carries_the_unanimous_core_rules() {
        // The contract consumers rely on when their app sheet overrides:
        // these rules are byte-stable, and removing one is a breaking change
        // for every app sheet built against them.
        let css = super::base_css(&Palette::dragon());
        for rule in [
            "headerbar {\n  background-color: #1d1c19;",
            ".linked > button:not(:first-child) { border-left-width: 0; }",
            "list.boxed-list > row:last-child { border-bottom: none; }",
            "button:active, button:checked { background-color: #c4746e; color: #12120f; border-color: #c4746e; }",
            "dropdown:focus-visible,\nscale:focus-visible { outline: 1px solid #c4746e; outline-offset: -1px; }",
            ".numeric { font-feature-settings: 'tnum'; }",
        ] {
            assert!(css.contains(rule), "base_css lost a core rule: {rule}");
        }
    }
}
