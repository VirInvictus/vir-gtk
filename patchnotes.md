# vir-gtk Patch Notes

## v1.1.0 (2026-09-06)

**Phase 2 opens: the shared base stylesheet, the override ladder, and the
chart color helpers.**

*   **`theme::base_css(palette)` ships the shared base widget sheet.** The
    unanimous flat/square core distilled from Atrium's, Conservatory's,
    and Viaduct's copied sheets: window chrome, headerbar, lists and rows,
    the button family, entries, popovers, tooltips, scrollbars, the
    Adwaita utility classes, and the scoped focus ring, with the palette's
    hexes spliced so no `%TOKEN%` survives. Per the approved design brief
    it carries none of the deliberate divergences: no radius/checked
    parameters, no `font-family`, no `@define-color` block, no
    row-selection rule, no toast. Atrium's rounding, Conservatory's lifted
    selection and lit checked buttons, and Viaduct's transparent lists
    stay app-side, where they belong.
*   **`install_app_stylesheet(css)` completes the override ladder.** The
    system `gtk.css` sits below `USER`, the crate's sheets at `USER + 1`,
    and the application's sheet at `USER + 2`, each tier tracked per
    display and replaced on re-install. App rules now beat the shared base
    by priority instead of by install timing, which was the one
    load-bearing ordering Atrium depended on.
*   **`vir_gtk::color` for the consumers' custom-drawn charts.**
    `to_gdk_rgba` and `to_cairo_rgba` parse strict CSS hex (malformed
    input is `None`, not a panic) into a `gdk::RGBA` and the 0.0-1.0
    triple cairo's source functions take, and `redraw_on_theme_change`
    re-queues a widget's draw on portal dark/light flips through a weak
    reference. The cairo dependency the 2026-09-04 gate worried about
    never materializes: the cairo values are plain `f64` triples, and
    `cairo-rs` is already transitive in every consumer via `gdk4`.
*   **Docs:** spec.md gains the base-sheet, priority-ladder, and
    color-helper contracts; README describes the third module; CLAUDE.md
    records the ladder rule; the roadmap's base_css box records how each
    of the four design-brief calls resolved.

Suite: 20 green (8 theme + 6 color + 6 portal), clippy `-D warnings`
clean, `cargo fmt --check` clean.

## v1.0.4 (2026-09-06)

**The broadcast panic repair, plus the recon's small findings.**

*   **Re-entrant broadcasts work, for real this time.** 1.0.3's
    `broadcast()` kept its index iteration but still held the `LISTENERS`
    `RefCell` borrow across every callback, so a listener that re-entered
    the portal mid-broadcast (a `resolve_now()` after a state change, or a
    `connect_dark_changed()` from inside a callback) panicked with
    `BorrowMutError`, contradicting the 1.0.3 notes, the roadmap, and the
    doc comment. Each callback is now cloned out of the borrow and fired
    outside it: a nested broadcast runs to completion, a mid-pass
    registration neither panics nor disturbs the running pass, and two
    regression tests pin both paths.
*   **A failed session-bus connection is no longer silent.** When
    `bus_get` cannot reach the session bus (headless machines, a broken
    portal daemon), the portal degraded to the application default as
    designed but left no trace. It now emits a `g_warning` on the crate's
    `vir-gtk` log domain; the fallback behavior itself is unchanged. This
    is the one path the test suite cannot reach (no bus in the tests).
*   **Dead `tracing` dependency dropped.** It was declared in
    `Cargo.toml` and never imported anywhere in the crate.
*   **CI installs only what the build links.** The workflow still carried
    `sqlite-devel` and `xorg-x11-server-Xvfb` from the Atrium workflow it
    was copied from; both are gone.
*   **Docs:** spec.md's API contract now lists `system_is_dark()` and
    `resolve_now()`, public since 1.0.3, and its fallback section records
    the new warning behavior. The roadmap records the Hermitage
    `theme.py` parity question as an open box instead of leaving it only
    in the audit folder.

Suite: 11 green (5 theme + 6 portal), clippy `-D warnings` clean,
`cargo fmt --check` clean.

## v1.0.3 (2026-09-04)

**Phase 3: the six hidden bugs and both refactors.** Every fix verified
against the source before landing:

*   **`init()` no longer blocks the main thread.** The portal read ran
    `bus_get_sync` + a synchronous D-Bus call (up to a 1000 ms timeout) on
    the startup path, before the first frame. `init()` now sends the read
    asynchronously (`gio::bus_get` + `DBusConnection::call`); the answer
    lands through the main loop and re-resolves state, so startup renders on
    the application default immediately. The `Read`-fallback for older
    portals rides the same async path. `CONNECTION` (the old dead `BUS`)
    is now load-bearing: it keeps the signal subscription alive for the
    process lifetime.
*   **The portal's "no preference" no longer forces light.**
    `portal_scheme_is_dark(0)` resolved to `false`, so a desktop reporting
    scheme 0 ("no preference") overrode the caller's `default_dark`. The
    mapping is now `portal_scheme_preference`: `1` → dark, `2` → light,
    anything else → `None`, and the application default stands.
*   **Background threads read real state.** The boolean state
    (`INITIALIZED`/`SYSTEM_DARK`/`RESOLVED_DARK`, plus a new
    `DEFAULT_DARK`) lives in global atomics, so `is_dark()` and
    `system_is_dark()` are safe from any thread. The GTK-bound pieces
    (settings, connection, listener registry) stay in main-thread
    thread-locals: they are `!Send` by nature. This is also roadmap Phase
    2's thread-safe-portal item in its honest shape.
*   **Re-entrant broadcasts work.** `broadcast()` used `mem::take` on the
    listener vec, so a callback that triggered a nested state change fired
    nothing. It now iterates by index over a length sampled up front:
    nested broadcasts reach every listener, and a callback that registers
    a new listener neither panics the `RefCell` nor gets skipped.
*   **`%BG%` and `%HEADING%` tokens exist.** The palette had the fields but
    no token mapping, so legacy stylesheets leaked raw placeholders. The
    test that pinned their absence flipped with the fix, and the
    every-token test now covers all fifteen slots.
*   **`install_stylesheet` replaces instead of accumulating.** Nothing ever
    removed a provider from the display, so live theme switches stacked
    them. The crate now tracks the provider it last installed per display
    and removes it before adding the new one. The API is unchanged.
*   **Docs sync:** CLAUDE.md's "single-writer mutexes" claim replaced with
    the real state shape; spec.md's phantom `connect_changed` signature
    corrected to `connect_dark_changed(owner, f)`, the "stateless crate"
    wording dropped, and the async/threading semantics documented; the
    README's consumer list no longer claims Colophon (it does not depend
    on this crate); CI comments describe vir-gtk instead of copied Atrium
    notes, and the test step drops xvfb now that no test initializes GTK.

Suite: 9 green (5 theme + 4 portal), clippy `-D warnings` clean,
`cargo fmt --check` clean.

## v1.0.2 (2026-08-25)

- **Tests:** Added the crate's first unit tests (8): `Palette::dragon()` pinned to the Kanagawa Dragon reference hexes, Dragon/Lotus distinctness, full `%TOKEN%` substitution coverage (including the documented `%BG%`/`%HEADING%` gap from roadmap Phase 3), CSS custom-property generation, and the portal `color-scheme` value mapping plus theme-nick resolution helpers.

## v1.0.1 (2026-08-23)

- **Build:** add GitHub Actions CI workflow

## v1.0.0 (2026-08-22)

**Initial Extraction and Release**
`vir-gtk` has been extracted from Atrium, Conservatory, Viaduct, and Colophon into a standalone shared library. This centralizes the VirInvictus design idiom into a single repository and permanently removes the need for individual applications to duplicate D-Bus portal listening code or hardcode hex values.

*   **Portal Module**: Introduced the `portal` module to handle `org.freedesktop.portal.Settings` resolution. The module automatically syncs with the desktop environment's `color-scheme` broadcast, dropping dead weak references safely to prevent memory leaks in GTK's main loop.
*   **Theme Module**: Added the `theme` module containing the authoritative Kanagawa Dragon and Lotus color palettes. It supports CSS generation and string token replacement to securely inject themes into GTK4 `CssProvider` instances.
*   **Priority CSS**: The library strictly enforces CSS injection at `STYLE_PROVIDER_PRIORITY_USER + 1`, ensuring that `vir-gtk` styling always overrides standard desktop stylesheets while preserving the application's ability to selectively override properties.
