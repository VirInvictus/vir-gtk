# vir-gtk Patch Notes

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
