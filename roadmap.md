# vir-gtk Roadmap

- [x] Extract `theme.rs` and `color_scheme.rs` logic from the desktop apps.
- [x] Standardize `portal` logic to handle optional settings injection and fallbacks.
- [x] Support multiple injection patterns (Custom CSS properties & String token replacement).
- [ ] Explore native Wayland color-scheme read protocols when they land in `gio`.

## Phase 2: Component Consolidation & Ecosystem Parity
*Context: Centralizing duplicated base CSS and plain-GTK widget replacements (rows, dialogs, clamp) currently scattered across Atrium, Conservatory, Viaduct, Colophon, and Framework.*

- [ ] **Centralized Base Widget Stylesheet:** Export `vir_gtk::theme::base_css()` covering standard widgets, typography (`.title-1`), and focus rings. Eliminates 800+ lines of duplicated CSS across downstream apps.
- [ ] **Managed Stylesheet Lifecycle:** Create `StyleManager` to safely swap `GtkCssProvider` instances per `GdkDisplay` without memory leaks during live theme switches.
- [ ] **Thread-Safe Portal State:** Replace `thread_local!` state with thread-safe atomics / `Arc<RwLock>` so background threads in Colophon and Conservatory can safely query `is_dark()`.
- [ ] **Extended Palette Support:** Add Kanagawa Wave, secondary charting roles, 6-hue swatches, and a `ThemeRegistry` (Gruvbox, Nord) to allow Colophon to fully migrate to `vir-gtk`.
- [ ] **Cairo & GDK Color Helpers:** Add hex-to-RGBA/Cairo conversion tools (`to_gdk_rgba`, `to_cairo_rgba`) and automatic redraw queuing to clean up Conservatory's waveform/spectrum and Colophon's charts.
- [ ] **Shared Plain-GTK4 Component Kit:** Introduce `vir_gtk::widgets` containing shared implementations for `action_row`, `combo_row`, `Alert` dialogs, `StatusPage`, and `Clamp`. Eliminates duplicated widgets across Atrium, Conservatory, and Viaduct.
- [ ] **C-ABI Integration for Framework:** Expose a `vir-gtk-capi` library and `vir-gtk.pc` pkg-config file so the C17 document viewer (Framework) can drop its 358-line manual D-Bus portal C port.
- [ ] **Per-Window Style Overrides:** Add GSettings binding builders and allow independent style contexts so Conservatory's "Now Playing" full-screen can force dark mode regardless of the system theme.

## Phase 3: Hidden Bugs & Integrity (2026-08-23)
*Context: Found severe UI listener dropping, thread desyncs, and silent leaks.*

### Bugs to Fix
- [ ] **Missing CSS Tokens:** Add substitution for `%BG%` and `%HEADING%` in `Palette::replace_tokens` to prevent raw placeholders from bleeding into legacy stylesheets.
- [ ] **Re-entrant Listener Drops:** Fix `broadcast()` zeroing the listener array during re-entrant state mutations, which currently causes silent UI update failures.
- [ ] **Thread-Local State Desync:** Prevent background async workers from reading uninitialized `thread_local!` state and assuming light mode.
- [ ] **Synchronous D-Bus Blocking:** Avoid blocking the GTK main thread during synchronous `xdg-desktop-portal` reads on startup.
- [ ] **Theme Light Mode Override:** Fix `portal_scheme_is_dark(0)` overriding the `default_dark` configuration when the portal expresses no preference.
- [ ] **GtkCssProvider Leak:** Properly remove old `GtkCssProvider` instances from the `GdkDisplay` before appending new ones during dynamic theme switches.

### Refactoring & Growth
- [ ] **Clean Up D-Bus Connection:** Remove unused static `BUS` connection retention.
- [ ] **Docs Sync:** Update `CLAUDE.md` to remove claims about single-writer mutexes, update `connect_changed` signatures, and align GTK version requirements.
