# vir-gtk Patch Notes

## v1.4.1 (2026-09-15)

**The base sheet owns plain label text, and the provider-ladder rustdoc
states what priority actually buys.**

*   **`label { color: %FG% }` (+ `label:disabled` dim) in the base
    template.** The failure Viaduct's force-light QA found: a desktop
    whose `gtk-theme-name` points at a dark third-party GTK theme writes
    explicit label colors at theme priority, and explicit rules beat
    inheritance everywhere the higher tiers don't restate a color, so
    light Kanagawa backgrounds rendered with the dark theme's label text.
    Proven by running the same binary with `GTK_THEME=Adwaita:light`:
    identical app sheets, clean render. Consumers: Viaduct takes this
    release with the fix verified live in both modes; Atrium and
    Conservatory are the routine lock bump.
*   **Documented the ladder's real reach** (final-audit MED): "app rules
    always win" was overstated at three rustdoc sites. Priority buys
    same-specificity wins only; a base rule on a selector the app sheet
    never restates stands, which is exactly how the label gap above
    survived two tiers of app-owned stylesheets.

## v1.4.0 (2026-09-13)

**The widget-kit first slice, the portal hardening from the six-lens
audit, the capi surface, and the docs debt, in one cascade release.**

*   **`vir_gtk::widgets`, the approved first slice.** The shared
    plain-GTK row family (`row`, `action_row`, `switch_row`, `spin_row`,
    `combo_row`, `button_row`, `entry_row`), `Group`, `Alert`, and
    `close_on_escape` move into the crate from the near-verbatim copies
    Atrium, Conservatory, and Viaduct each carried. `entry_row` takes
    the widened four-`Option` signature (title, subtitle, text,
    placeholder) that reconciles the three historical shapes.
    `Alert` ships on the Atrium/Conservatory shape: named responses with
    per-response appearance, a default response for Enter, and exactly
    one response id per presentation (dismissal without a button emits
    the close response). `close_on_escape` uses the capture-phase,
    weakly-held window shape. Expander rows, StatusPage, toasts, Clamp,
    and Page/Bin stay application-side by the slice rule, as does
    Atrium's tokio-based `choose_future` (the kit is sync-only; a thin
    wrapper over `connect_response` covers it app-side).
*   **`theme::install_default()`** starts the portal and keeps the base
    sheet re-spliced on the crate tier for the process lifetime. This is
    also the fix for the README's old init example, which taught
    a pattern that never re-spliced after startup; the manual loop now
    appears in the README as the alternative, not the default.
*   **Library-panic hardening.** The `SettingChanged` handler reads the
    signal body through `try_child_value` with early returns, so a
    malformed D-Bus body degrades to "not our key" instead of panicking
    into a consumer's main loop. Regression tests pin the shapes that
    used to panic.
*   **Portal robustness.** A generation counter stamps every portal read
    at issue; a live signal apply bumps it and a stale `ReadOne`/`Read`
    reply drops instead of overwriting fresher state. A
    `NameOwnerChanged` watcher re-reads when the portal name is
    acquired, so a portal that starts late or restarts has its missed
    changes picked up. Total failure of both read shapes now warns on
    the `vir-gtk` log domain instead of standing silently.
*   **StyleScope leak closed.** A scope whose target dies while a
    palette is forced now tears its display provider down on the
    target's destroy; before, the provider stayed installed forever.
    The (display, tier) registry also prunes closed displays.
*   **`capi/` member: the C-ABI surface** (`vir-gtk-capi` cdylib +
    staticlib, handwritten `vir-gtk.h`, `vir-gtk.pc`) built from
    Framework's `fw-theme.c` call surface: `vir_gtk_theme_install`,
    `vir_gtk_is_dark`, `vir_gtk_on_dark_changed`,
    `vir_gtk_disconnect_dark_changed`. `palette_css` is deliberately not
    exported yet: the palette reconciliation against Framework's
    backdrop/border/shade table is unsettled and recorded open in the
    roadmap. Framework adopts at its 1.0.1 (decision 29).
*   **Docs surface.** Cargo.toml gains description/license/repository;
    the crate has a real docs landing plus `#![warn(missing_docs)]`; all
    five portal functions carry their contract rustdoc; every Palette
    field is documented; the `gtk-application-prefer-dark-theme` global
    side effect is documented where it happens; the style docs and spec
    record that the ladder is a tie-breaker, not a trump (GTK CSS
    specificity beats provider priority). The CLAUDE/AGENTS/spec
    exclusion list is corrected to what base_css actually carries (the
    1.1.0 majority rules: checked paint, text selection tint,
    typography utilities are IN the base; row-selection styling,
    radius, `@define-color`, and font-family rules stay app-side).
*   **Xvfb returned to CI** for the new `#[gtk::test]` widget, scope,
    and install_default tests (the 1.0.3 drop was for the old
    display-free suite). Suite: 51 tests + doc-test, clippy `-D
    warnings` clean, fmt clean, now spanning the crate and the capi
    member.

**Cascade (consumers):** one adoption commit per consumer, each with
the lock bump and a green suite: Atrium (deletes the shared subset of
`rows.rs` and `dialogs.rs`' Alert, keeps `Page`/`Bin` and the tokio
`choose_future` wrapper, `entry_row` call sites adapt to the widened
signature), Conservatory (likewise, keeps `Expander`), Viaduct (deletes
`rows.rs` and `alert.rs`, adapts its tuple-group and `ResponseStyle`
call sites). Wave detail and commit hashes in the roadmap's 1.4.0 wave
record.

## v1.3.0 (2026-09-13)

**The gtk4 0.11 platform bump, so consumers can take the same step
ahead of their 1.0 freezes (Atrium decision 61: the gtk4 0.11 crate
lands before its v1.0.0 tag).**

*   **gtk4 dependency moves 0.9 to 0.11** (gtk-rs-core follows: glib
    and gio land on 0.22). The `v4_14` feature pin is unchanged; no
    API surface is added or removed.
*   **portal.rs ports the `SettingChanged` listener to gio 0.22's
    `subscribe_to_signal`** (the old `signal_subscribe` is deprecated
    and CI runs clippy `-D warnings`). One behavioral detail carried
    deliberately: the new API returns a strong `SignalSubscription`
    whose drop unsubscribes, so the handle now lives in a process-
    lifetime thread-local next to `CONNECTION` instead of being
    discarded. Listener lifetime is unchanged in practice; the
    teardown path is now explicit rather than incidental.
*   Suite green (33 tests), clippy clean on the new stack.

**Cascade (consumers):** Atrium adopts in the same wave (decision 61).
Conservatory and Viaduct adopt at their next releases; this roadmap
records that waiver so the wave counts as closed; both consumers
compile against 1.2.0 until then, and vir-gtk 1.3.0 changes none of
the API they call (the portal port is internal).

## v1.2.0 (2026-09-10)

**Phase 2 continues: the managed stylesheet lifecycle and the
forced-palette subtree override.**

*   **`vir_gtk::style::StyleManager` is the named lifecycle API.** The
    (display, tier) provider registry that 1.0.3 introduced and 1.1.0
    split into tiers is now a public type: a `StyleManager` handle is a
    key to one rung of the ladder, `crate_tier()` (USER + 1),
    `app_tier()` (USER + 2), or `at_priority(prio)` for layers above the
    app sheet (Conservatory's runtime accent provider sits at USER + 3
    and becomes formally manageable instead of hand-rolled state). Each
    rung gets `install` (replace), `remove` (explicit teardown, the one
    capability the tracking lacked), `is_installed`, and `priority`.
    Handles are keys, not owners: sheets are display-global state,
    dropping every handle uninstalls nothing, and clones share the rung.
    `install_stylesheet`/`install_app_stylesheet` stay in `theme` as
    one-line delegates, so no consumer import changes.
*   **`vir_gtk::style::{ThemeChoice, StyleScope}` ship the per-window
    style overrides.** A `StyleScope` roots at any widget subtree, not
    just windows, because the motivating case, Conservatory's Now
    Playing full-screen, is a stack page inside the main window: while
    the choice forces a palette, the target carries the
    `vir-style-scope` class and one display provider at USER + 4 (above
    every global rung, including runtime accent layers) serves
    `base_css` re-spliced with the forced palette, the app's own
    `extra_css` template, and a root-canvas rule, all scoped under the
    class by the new public `scope_css` transform. `ThemeChoice`
    (`system`/`dark`/`light`) parses the nick forms the portal resolver
    already accepts; unknown nicks warn on the `vir-gtk` log domain and
    stand on System. `bind_settings(&settings, key)` makes a writable
    string key the source of truth and `set_choice` writes back through
    it when bound; System tears the provider down and hands the subtree
    back to the global ladder. Widget-scoped `StyleContext` providers
    were the obvious mechanism and are unusable: deprecated since GTK
    4.10 and scoped to the single widget anyway, so the shipped shape is
    the class-scoped display provider Conservatory's accent provider
    already validated.
*   **`scope_css(css, class)` is public.** The scoping transform rewrites
    a flat stylesheet so its rules only match inside a subtree carrying
    the class: every selector is emitted in descendant form and with the
    class appended to its final compound, so scope-root subjects still
    match. Comments and declarations pass through verbatim; tooltips and
    popovers that do not descend from the scope root keep the global
    look (a documented edge of the class-scoping mechanism).
*   **Docs:** spec.md gains a Stylesheet Lifecycle section (§1.4) and the
    API contract entries; README describes the fourth module with a
    usage snippet; CLAUDE.md records the managed-lifecycle rule; the
    roadmap's lifecycle and per-window boxes carry the ship notes.

Suite: 33 green (8 theme + 6 color + 6 portal + 13 style), clippy
`-D warnings` clean, `cargo fmt --check` clean.

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
    2's thread-safe-portal item in its working shape.
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
`vir-gtk` has been extracted from Atrium, Conservatory, Viaduct, and Colophon into a standalone shared library. This centralizes the VirInvictus design idiom into a single repository and removes the need for individual applications to duplicate D-Bus portal listening code or hardcode hex values.

*   **Portal Module**: Introduced the `portal` module to handle `org.freedesktop.portal.Settings` resolution. The module syncs with the desktop environment's `color-scheme` broadcast, dropping dead weak references so dead UI objects do not linger in GTK's main loop.
*   **Theme Module**: Added the `theme` module containing the authoritative Kanagawa Dragon and Lotus color palettes. It supports CSS generation and string token replacement to inject themes into GTK4 `CssProvider` instances.
*   **Priority CSS**: The library installs CSS at `STYLE_PROVIDER_PRIORITY_USER + 1`, so `vir-gtk` styling overrides standard desktop stylesheets while preserving the application's ability to selectively override properties.
