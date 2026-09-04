# vir-gtk Roadmap

- [x] Extract `theme.rs` and `color_scheme.rs` logic from the desktop apps.
- [x] Standardize `portal` logic to handle optional settings injection and fallbacks.
- [x] Support multiple injection patterns (Custom CSS properties & String token replacement).
- [ ] Explore native Wayland color-scheme read protocols when they land in `gio`.

## Phase 2: Component Consolidation & Ecosystem Parity
*Context: Centralizing duplicated base CSS and plain-GTK widget replacements (rows, dialogs, clamp) currently scattered across Atrium, Conservatory, Viaduct, Colophon, and Framework.*

- [ ] **Centralized Base Widget Stylesheet:** Export `vir_gtk::theme::base_css()` covering standard widgets, typography (`.title-1`), and focus rings. Eliminates 800+ lines of duplicated CSS across downstream apps.
- [ ] **Managed Stylesheet Lifecycle:** Create `StyleManager` to safely swap `GtkCssProvider` instances per `GdkDisplay` without memory leaks during live theme switches. *(Partially shipped in 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display, closing the leak. The fuller named `StyleManager` API remains open.)*
- [x] **Thread-Safe Portal State:** Replace `thread_local!` state with thread-safe atomics / `Arc<RwLock>` so background threads in Colophon and Conservatory can safely query `is_dark()`. *(Shipped in 1.0.3 as the honest shape: the boolean state is global atomics readable from any thread; the GTK-bound pieces (settings, connection, listeners) stay main-thread thread-locals because they are `!Send`. Phase 3's desync bug is the same fix.)*
- [ ] **Extended Palette Support:** Add Kanagawa Wave, secondary charting roles, 6-hue swatches, and a `ThemeRegistry` (Gruvbox, Nord) to allow Colophon to fully migrate to `vir-gtk`.
- [ ] **Cairo & GDK Color Helpers:** Add hex-to-RGBA/Cairo conversion tools (`to_gdk_rgba`, `to_cairo_rgba`) and automatic redraw queuing to clean up Conservatory's waveform/spectrum and Colophon's charts.
- [ ] **Shared Plain-GTK4 Component Kit:** Introduce `vir_gtk::widgets` containing shared implementations for `action_row`, `combo_row`, `Alert` dialogs, `StatusPage`, and `Clamp`. Eliminates duplicated widgets across Atrium, Conservatory, and Viaduct.
- [ ] **C-ABI Integration for Framework:** Expose a `vir-gtk-capi` library and `vir-gtk.pc` pkg-config file so the C17 document viewer (Framework) can drop its 358-line manual D-Bus portal C port.
- [ ] **Per-Window Style Overrides:** Add GSettings binding builders and allow independent style contexts so Conservatory's "Now Playing" full-screen can force dark mode regardless of the system theme.

## Phase 3: Hidden Bugs & Integrity (2026-08-23)
*Context: Found severe UI listener dropping, thread desyncs, and silent leaks.*

### Bugs to Fix
- [x] **Missing CSS Tokens:** Add substitution for `%BG%` and `%HEADING%` in `Palette::replace_tokens` to prevent raw placeholders from bleeding into legacy stylesheets. *(Shipped 1.0.3; the test pinning their absence flipped, and the every-token test covers all fifteen slots.)*
- [x] **Re-entrant Listener Drops:** Fix `broadcast()` zeroing the listener array during re-entrant state mutations, which currently causes silent UI update failures. *(Shipped 1.0.3: index iteration over a sampled length instead of `mem::take`; nested broadcasts reach everyone, mid-broadcast registrations neither panic nor get skipped.)*
- [x] **Thread-Local State Desync:** Prevent background async workers from reading uninitialized `thread_local!` state and assuming light mode. *(Shipped 1.0.3: boolean state moved to global atomics; the direction note in the old text was wrong — the thread-local defaults were dark, not light. Phase 2's thread-safety item records the full shape.)*
- [x] **Synchronous D-Bus Blocking:** Avoid blocking the GTK main thread during synchronous `xdg-desktop-portal` reads on startup. *(Shipped 1.0.3: the portal read is async (`gio::bus_get` + `DBusConnection::call`), with the Read-fallback for older portals on the same path; `init()` returns without blocking.)*
- [x] **Theme Light Mode Override:** Fix `portal_scheme_is_dark(0)` overriding the `default_dark` configuration when the portal expresses no preference. *(Shipped 1.0.3: `portal_scheme_preference` maps 1→dark, 2→light, anything else to None, and None falls back to `default_dark`.)*
- [x] **GtkCssProvider Leak:** Properly remove old `GtkCssProvider` instances from the `GdkDisplay` before appending new ones during dynamic theme switches. *(Shipped 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display; API unchanged.)*

### Refactoring & Growth
- [x] **Clean Up D-Bus Connection:** Remove unused static `BUS` connection retention. *(Resolved in 1.0.3 by making retention load-bearing: the async redesign keeps the connection alive for the signal subscription, renamed `CONNECTION` with the invariant documented — the dead-write state no longer exists.)*
- [x] **Docs Sync:** Update `CLAUDE.md` to remove claims about single-writer mutexes, update `connect_changed` signatures, and align GTK version requirements. *(Shipped 1.0.3: CLAUDE.md documents the atomics + thread-locals shape; spec.md's `connect_changed` corrected to the real `connect_dark_changed(owner, f)` and the "stateless crate" wording dropped; CI comments now describe vir-gtk (they were copied from Atrium) and the test step dropped xvfb since no test initializes GTK.)*
