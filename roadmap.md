# vir-gtk Roadmap

- [x] Extract `theme.rs` and `color_scheme.rs` logic from the desktop apps.
- [x] Standardize `portal` logic to handle optional settings injection and fallbacks.
- [x] Support multiple injection patterns (Custom CSS properties & String token replacement).
- [ ] Explore native Wayland color-scheme read protocols when they land in `gio`.

## Phase 2: Component Consolidation & Ecosystem Parity
*Context: Centralizing duplicated base CSS and plain-GTK widget replacements (rows, dialogs, clamp) currently scattered across Atrium, Conservatory, Viaduct, Colophon, and Framework.*

- [x] **Centralized Base Widget Stylesheet:** Export `vir_gtk::theme::base_css()` covering standard widgets, typography (`.title-1`), and focus rings. Eliminates 800+ lines of duplicated CSS across downstream apps. *(2026-09-04 survey, totals re-measured 2026-09-05: a ~145-line base sheet is copied across Atrium (theme.rs, 272 lines), Conservatory (theme.rs, 235), and Viaduct (theme.rs, 224) — the shared fraction is roughly 53-65% of each, not the 72-76% first computed against stale totals; re-measure the base sheet itself before scoping. Atrium's data/style.css is 1,012 lines (951 at survey time), ~all app-specific. DESIGN BRIEF before any code: (1) radius and checked/selected semantics diverge deliberately — Atrium rounded + paint-accent, Conservatory/Viaduct square, Conservatory "lift not paint" — so base_css() needs radius/checked-style parameterization or per-app override guarantees; (2) Atrium's data/style.css consumes @define-color adwaita names defined only in Atrium's own sheet — that block must move into vir-gtk or stay app-side or 1,012 lines lose their colors; (3) token mechanism splits 1-of-3: only Viaduct consumes var(--c-*), Atrium/Conservatory use %TOKEN% hexes, and Conservatory pins gtk4 v4_14 partly to skip custom properties — pick tokens or require a feature bump; (4) install order is the only override mechanism today (USER+1 everywhere, later-wins) and Atrium depends on it; a shared sheet must install before app sheets and never re-install after them; (5) Viaduct hand-reimplements the provider-swap this crate now provides (1.0.3) — subsume it. Shipped 1.1.0 (2026-09-06), the four brief calls resolved as approved: (1) no parameters — the base ships the unanimous square/flat core (button-checked paint is the 2-of-3 majority; row selection differs in all three so it is excluded entirely) and app sheets override; (2) @define-color stays app-side; (3) tokens, not var() — Conservatory's v4_14 pin respected, Viaduct keeps var() in its app sheet over the properties block; (4) the ladder replaces timing — install_app_stylesheet (USER+2) outranks the crate tier (USER+1) by construction; (5) Viaduct's hand-rolled provider swap is subsumed by the crate tiers. Base is ~120 lines; consumer adoption landed in the 1.1.0 wave.)*
- [x] **Managed Stylesheet Lifecycle:** Create `StyleManager` to safely swap `GtkCssProvider` instances per `GdkDisplay` without memory leaks during live theme switches. *(Partially shipped in 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display, closing the leak. 1.1.0: the tracking is per (display, tier) and `install_app_stylesheet` adds the app tier. Shipped 1.2.0 (2026-09-10): the tracking becomes the named `vir_gtk::style::StyleManager` API. A handle is a key to one rung of the ladder, not an owner: `crate_tier()` (USER + 1), `app_tier()` (USER + 2), or `at_priority(prio)` for layers above the app sheet (Conservatory's runtime accent provider lives at USER + 3 and becomes formally manageable instead of hand-rolled state), each with `install` (replace), `remove` (explicit teardown, the one capability the tracking lacked), `is_installed`, and `priority`. Clones share the rung; dropping every handle uninstalls nothing. `install_stylesheet`/`install_app_stylesheet` stay in `theme` as one-line delegates, so no consumer import changes; the (display, tier) registry moved with the manager into `style`.)*
- [x] **Thread-Safe Portal State:** Replace `thread_local!` state with thread-safe atomics / `Arc<RwLock>` so background threads in Colophon and Conservatory can safely query `is_dark()`. *(Shipped in 1.0.3 as the honest shape: the boolean state is global atomics readable from any thread; the GTK-bound pieces (settings, connection, listeners) stay main-thread thread-locals because they are `!Send`. Phase 3's desync bug is the same fix.)*
- [ ] **Extended Palette Support:** Add Kanagawa Wave, secondary charting roles, 6-hue swatches, and a `ThemeRegistry` (Gruvbox, Nord) to allow Colophon to fully migrate to `vir-gtk`.
  *(DECIDED 2026-09-12 (Brandon): deferred until Colophon actually asks to migrate; the box stays open as on-demand.)*
- [x] **Cairo & GDK Color Helpers:** Add hex-to-RGBA/Cairo conversion tools (`to_gdk_rgba`, `to_cairo_rgba`) and automatic redraw queuing to clean up Conservatory's waveform/spectrum and Colophon's charts. *(Shipped 1.1.0 (2026-09-06): `vir_gtk::color` with strict-hex `to_gdk_rgba`/`to_cairo_rgba` and `redraw_on_theme_change`, a weakly-held redraw hook on portal dark/light flips. The 2026-09-04 gate dissolved in the design: the cairo values are plain f64 triples, so this crate needs no cairo dependency at all; `cairo-rs` stays transitive via gdk4 where consumers link it.)*
- [ ] **Shared Plain-GTK4 Component Kit:** Introduce `vir_gtk::widgets` containing shared implementations for `action_row`, `combo_row`, `Alert` dialogs, `StatusPage`, and `Clamp`. Eliminates duplicated widgets across Atrium, Conservatory, and Viaduct. *(2026-09-04 survey: ~1,300 lines of near-verbatim Rust duplication — row builders (Atrium rows.rs 302 / Conservatory 268 / Viaduct 212, `build_row` verbatim x3), StatusPage composites x3 (three shapes, one concept), alert dialogs x3 (Atrium/Conservatory APIs identical, 131-line diff is docs; Viaduct's is parent-bound), toast revealer + newest-wins timeout x3. Atrium-only: Clamp, Page/Bin. Drift to reconcile: entry_row signatures differ, group() returns struct vs tuple.)*
  *(DECIDED 2026-09-12 (Brandon): approved as a FIRST SLICE (the shared row builders plus alert dialogs, the largest verbatim mass), shipped as a crate release with the Atrium/Conservatory/Viaduct adoption wave; the rest of the kit re-scopes after the slice lands.)*
  *(SLICE SHIPPED 1.4.0 (2026-09-13): `vir_gtk::widgets` carries the row family, `Group`, `Alert`, and `close_on_escape`; see the 2026-09-12 findings box for the full ship note. The remainder of this box (StatusPage, Clamp, expander rows, toasts) stays open as the re-scoped follow-up.)*
- [ ] **C-ABI Integration for Framework:** Expose a `vir-gtk-capi` library and `vir-gtk.pc` pkg-config file so the C17 document viewer (Framework) can drop its manual D-Bus portal C port (`fw-theme.c`, 373 lines at the 2026-09-05 re-measure; 357 at survey time).
  *(DECIDED 2026-09-12 (Brandon): the capi is approved with the MIT-into-GPL attribution note, and Framework adopts in the same stage as its 1.0.0 tag (tag approved the same evening).)*
  *(Update 2026-09-13: Framework 1.0.0 shipped WITHOUT the capi, so its adoption moved to 1.0.1. This crate's side SHIPPED in 1.4.0 as the `capi/` member; the box stays open only for Framework's adoption and the palette_css half, both recorded in the 2026-09-12 findings box.)*
- [x] **Per-Window Style Overrides:** Add GSettings binding builders and allow independent style contexts so Conservatory's "Now Playing" full-screen can force dark mode regardless of the system theme. *(Shipped 1.2.0 (2026-09-10): `vir_gtk::style::{ThemeChoice, StyleScope}`. `ThemeChoice` parses the settings nicks (`system`/`default`/`dark`/`force-dark`/`light`/`force-light`; unknown nicks warn on the `vir-gtk` domain and stand on System, the portal's degrade-loudly rule). `StyleScope` roots at any widget subtree, not just windows, because the motivating full-screen is a stack page inside Conservatory's main window: while the choice forces a palette the target carries the `vir-style-scope` class and one display provider at USER + 4 (above every global rung, including runtime accent layers) serves `base_css` re-spliced with the forced palette plus the app's `extra_css` template plus a root-canvas rule, all scoped under the class by the public `scope_css` transform (descendant prefix plus class-appended root form; tooltips and popovers that do not descend from the target keep the global look, a documented edge). `bind_settings(&settings, key)` makes a writable string key the source of truth and `set_choice` writes back through it when bound; System tears the provider down and hands the subtree back to the global ladder. Widget-scoped `StyleContext` providers were the obvious mechanism and are unusable: deprecated since GTK 4.10 and scoped to the single widget anyway, so the shipped shape is the class-scoped display provider Conservatory's accent provider already validated.)*

## Phase 3: Hidden Bugs & Integrity (2026-08-23)
*Context: Found severe UI listener dropping, thread desyncs, and silent leaks.*

### Bugs to Fix
- [x] **Missing CSS Tokens:** Add substitution for `%BG%` and `%HEADING%` in `Palette::replace_tokens` to prevent raw placeholders from bleeding into legacy stylesheets. *(Shipped 1.0.3; the test pinning their absence flipped, and the every-token test covers all fifteen slots.)*
- [x] **Re-entrant Listener Drops:** Fix `broadcast()` zeroing the listener array during re-entrant state mutations, which currently causes silent UI update failures. *(Shipped 1.0.3: index iteration over a sampled length instead of `mem::take`. Corrected in 1.0.4: the 1.0.3 loop still held the `LISTENERS` `RefCell` borrow across each callback, so the re-entrancy it promised panicked with `BorrowMutError`; callbacks are now cloned out of the borrow and fired outside it, with two regression tests pinning the re-entrant broadcast and the mid-pass registration.)*
- [x] **Thread-Local State Desync:** Prevent background async workers from reading uninitialized `thread_local!` state and assuming light mode. *(Shipped 1.0.3: boolean state moved to global atomics; the direction note in the old text was wrong — the thread-local defaults were dark, not light. Phase 2's thread-safety item records the full shape.)*
- [x] **Synchronous D-Bus Blocking:** Avoid blocking the GTK main thread during synchronous `xdg-desktop-portal` reads on startup. *(Shipped 1.0.3: the portal read is async (`gio::bus_get` + `DBusConnection::call`), with the Read-fallback for older portals on the same path; `init()` returns without blocking.)*
- [x] **Theme Light Mode Override:** Fix `portal_scheme_is_dark(0)` overriding the `default_dark` configuration when the portal expresses no preference. *(Shipped 1.0.3: `portal_scheme_preference` maps 1→dark, 2→light, anything else to None, and None falls back to `default_dark`.)*
- [x] **GtkCssProvider Leak:** Properly remove old `GtkCssProvider` instances from the `GdkDisplay` before appending new ones during dynamic theme switches. *(Shipped 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display; API unchanged.)*

### Refactoring & Growth
- [x] **Clean Up D-Bus Connection:** Remove unused static `BUS` connection retention. *(Resolved in 1.0.3 by making retention load-bearing: the async redesign keeps the connection alive for the signal subscription, renamed `CONNECTION` with the invariant documented — the dead-write state no longer exists.)*
- [x] **Docs Sync:** Update `CLAUDE.md` to remove claims about single-writer mutexes, update `connect_changed` signatures, and align GTK version requirements. *(Shipped 1.0.3: CLAUDE.md documents the atomics + thread-locals shape; spec.md's `connect_changed` corrected to the real `connect_dark_changed(owner, f)` and the "stateless crate" wording dropped; CI comments now describe vir-gtk (they were copied from Atrium) and the test step dropped xvfb since no test initializes GTK.)*

### 2026-09-06 Recon Sweep
*Context: findings from the 2026-09-05 audit recon, recorded here so they exist in-repo before being fixed.*

- [x] **Dead `tracing` Dependency:** dropped from `Cargo.toml`; it was declared but never imported anywhere in the crate. *(Shipped 1.0.4.)*
- [x] **Silent `bus_get` Failure:** a failed session-bus connection emitted a bare `return` with no record; it now emits a `g_warning` on the crate's `vir-gtk` log domain before the default-theme fallback stands, per the crate's degrade-safely design. *(Shipped 1.0.4; the one path the suite cannot reach, since no test touches a bus.)*
- [x] **Spec API Coverage:** `system_is_dark()` and `resolve_now()` have been public since 1.0.3 but were missing from spec.md's API contract; both are now listed, and the fallback section records the g_warning behavior. *(Shipped 1.0.4.)*
- [x] **CI Package Residue:** the workflow installed `sqlite-devel` and `xorg-x11-server-Xvfb`, copy-paste residue from the Atrium workflow this one was built from; neither is used by this crate. *(Shipped 1.0.4.)*
- [ ] **Hermitage `theme.py` Parity:** decide whether Hermitage's Python portal port is a parity target for this crate or explicitly out of scope. *(Recon finding 2026-09-05, previously recorded only in the audit folder. Brandon-gated; no design work started.)*
  *(DECIDED 2026-09-12 (Brandon): out of scope, with a cross-reference from Hermitage's docs; Hermitage cannot consume a Rust crate.)*
- [x] **1.0.4 Consumer Wave (cascade closed per the shared-library rule):** Atrium, Conservatory, and Viaduct re-pinned this crate at 1.0.4 (`864fe82`) and every suite is green: Atrium `cargo test --workspace` 988 passed, Conservatory workspace 633 passed plus the music-only lane 60 passed, Viaduct workspace 222 passed. Adoption commits: Atrium `66faf50` (lock + `data/cargo-sources.json` regen + patchnotes line), Conservatory `43f8625` (lock + patchnotes line), Viaduct `491110e` (lock + patchnotes line, which also records its previously missing 1.0.3 adoption). No consumer code changes were required: the fixed API surface is unchanged. Tag `v1.0.4` cut verbatim on `864fe82` and pushed with the release. *(2026-09-06.)*
- [x] **1.1.0 Consumer Wave (base_css adoption, cascade closed):** Atrium, Conservatory, and Viaduct adopted `base_css()` and the install ladder at `65f4249`, each moving its deliberate divergences onto `install_app_stylesheet` (USER + 2): Atrium `6b0544d` (@define-color + rounded idiom, `data/style.css` joins USER + 2, cargo-sources regen; suite 988 green), Conservatory `97ebe41` (lifted selection, lit checked, square controls, accent ring to USER + 3; workspace 633 + music-only 60 green), Viaduct `19300e7` (var()-based overrides, hand-rolled provider swap subsumed; suite 222 green). Sheet content preserved rule-for-rule; the visual deltas are Viaduct's newly-covered widget families (switch/check/scale/disabled had no rules before), flagged for Brandon's next display pass. Tag `v1.1.0` cut verbatim on `65f4249` and pushed with the release. *(2026-09-06.)*
- [x] **1.2.0 Consumer Wave (StyleManager + per-window overrides, cascade closed):** Atrium, Conservatory, and Viaduct re-pinned this crate at `6ffa999`. Both new surfaces are opt-in additions, so no consumer code changes were required. Adoption commits: Atrium `c46cbae` (lock + `data/cargo-sources.json` regen + patchnotes line; workspace suite green, CI `34533472657` green), Conservatory `b2b6891` (lock + patchnotes line; workspace and music-only lanes green), Viaduct `a46ad4b` (lock + patchnotes line; workspace suite green). Tag `v1.2.0` cut verbatim on `6ffa999` and pushed with the release; crate CI `34532984462` green. *(2026-09-10, main-thread execution of the gated cascade tail.)*

## New findings 2026-09-12 (six-lens full audit; detail: audit/FULL-AUDIT-2026-09-12.md, Wave 10)

- [x] **Library-panic hardening: the SettingChanged callback can panic
      into a consumer's main loop** (portal.rs:172-179 child_value
      asserts on malformed bodies; the subscription is not signature-
      pinned). Use try_child_value with an early return. A library must
      not panic into host apps.
      *(Shipped 1.4.0: the signal body goes through
      `setting_changed_scheme`, every child access is `try_child_value`,
      and a malformed body degrades to "not our key". Re-verified against
      the 1.3.0 listener port first, as the reconciliation required: the
      panic path survived the port. Tests pin the malformed shapes that
      used to panic: empty tuple, short tuple, wrong child types, wrong
      value type.)*
- [x] **Portal robustness:** a stale ReadOne/Read reply can overwrite
      fresher SettingChanged state (generation counter); changes made
      while the portal is down are never picked up (watch
      NameOwnerChanged and re-read); total read failure is silent
      against the spec's own "degradation is not silent".
      *(Shipped 1.4.0: a generation counter stamps every read at issue;
      a live SettingChanged apply bumps it and a stale reply drops
      instead of overwriting. A `NameOwnerChanged` watcher on the session
      bus (arg0-pinned to the portal name) re-reads when the portal name
      is acquired, covering late portal start and restart. Total failure
      of both ReadOne and Read now warns on the `vir-gtk` domain with
      the underlying error before the default stands.)*
- [x] **StyleScope leak:** a forced scope's provider stays installed on
      the display when the target widget dies (connect_destroy cleanup
      in the forced branch); INSTALLED never prunes closed displays.
      *(Shipped 1.4.0: `StyleScope::build` connects the target's destroy
      to a teardown holding the provider slot weakly, so a target dying
      while forced leaves no provider on the display; a regression test
      forces a palette, drops the target, and asserts the slot empties.
      The (display, tier) registry prunes closed displays on every
      install/remove/query; that half has no direct test because the
      default display cannot be safely closed in-process, and the ship
      note is the honest record of that.)*
- [x] **Docs/API surface:** all five portal public functions lack
      rustdoc (port spec 2's contract lines onto them); Cargo.toml needs
      description/license/repository; add #![warn(missing_docs)] and
      crate-level docs; Palette's 15 pub fields undocumented; portal.rs
      writes gtk-application-prefer-dark-theme globally - document the
      side effect; the README init example teaches the broken pattern
      (no listener = never re-splices) - show the loop or ship
      theme::install_default(); the ladder is a tie-breaker not an
      override (document the specificity profile).
      *(Shipped 1.4.0: all five portal functions carry their contract
      rustdoc, `init` documents the prefer-dark side effect, the crate
      landing has real docs plus `#![warn(missing_docs)]`, every Palette
      field is documented, the style module and spec carry the
      specificity profile, and the README's standard init is now
      `theme::install_default()` with the manual loop as the documented
      alternative. `install_default` shipped as the fix, and doubles as
      the payload behind the capi's `theme_install`.)*
- [x] **CLAUDE/AGENTS/spec exclusion-list correction:** checked paint,
      selection tint, and typography utilities ARE in base_css by the
      recorded 1.1.0 majority decisions - the exclusion list forbids
      rules the sheet carries and would break the byte-stable contract.
      Reword to row-selection styling / font-family rules per the
      roadmap resolutions.
      *(Shipped 1.4.0: CLAUDE.md and spec 1.2 now list the exclusions as
      radius, row-selection styling, @define-color, font-family, toasts,
      OSD, and name the carried majority rules explicitly so no agent
      following the docs "removes" them.)*
- [x] **Widget-kit first slice (approved):** unified rows + Alert +
      close_on_escape; entry_row widened signature reconciles the
      recorded drift; group() returns the struct (2-of-3); per-consumer
      adoption boxes; xvfb returns to CI for the gtk tests.
      *(Shipped 1.4.0 as `vir_gtk::widgets` on the Atrium/Conservatory
      body with Viaduct's `button_row`: `row`, `action_row`, `switch_row`,
      `spin_row`, `combo_row`, `button_row`, `entry_row` (four-Option
      signature: title, subtitle, text, placeholder), `Group` (struct,
      2-of-3, with `clear`), `Appearance`, `Alert` (Atrium/Conservatory
      shape: 2-arg `add_response` + `set_response_appearance`,
      `present(Option<&impl IsA<Widget>>)` deriving transient-for from
      the root, close paths emit exactly one id), `close_on_escape`
      (capture phase, weakly-held window, the 2-of-3 shape), and nine
      `#[gtk::test]` widget tests ported from the consumers' suites.
      Expander, Page/Bin, StatusPage, toasts, Clamp stay app-side per
      the slice rule; Atrium's tokio-based `choose_future` stays
      app-side (the kit is sync-only, no async runtime in the crate) as
      a thin wrapper over `connect_response`. Xvfb returned to CI for
      the gtk tests (the 1.0.4 drop was for the old display-free
      suite). Adoption wave: Atrium, Conservatory, and Viaduct, one
      commit each.)*
- [x] **capi surface enumerated:** theme_install / is_dark /
      on_dark_changed / palette_css; the one reconciliation is the
      palette (Framework's backdrop/border/shade variants and drifting
      hexes vs the crate's 15 slots) - canonical block generator or
      Framework keeps its table. prefer_dark_chrome() folds the fourth
      copy of the nudge. icons module candidate (tray theme installer +
      search-path probe).
      *(Shipped 1.4.0, crate side, as the `capi/` workspace member:
      `vir-gtk-capi` (cdylib + staticlib) exporting
      `vir_gtk_theme_install(default_dark)` (one-call portal + re-splicing
      base sheet, the `fw_theme_install` replacement),
      `vir_gtk_is_dark()`, `vir_gtk_on_dark_changed(callback, user_data,
      destroy)` with exactly-once destroy semantics and
      `vir_gtk_disconnect_dark_changed(id)`. Handwritten `vir-gtk.h` and
      `vir-gtk.pc` checked in beside it; the pc Version moves with the
      release. THREE ITEMS STAY OPEN, recorded here so nothing is
      silently dropped: (1) `palette_css` is NOT exported because the
      palette reconciliation is unsettled - Framework's
      backdrop/border/shade variants and drifting hexes (card #201f1e,
      accent dragonBlue2, success #8a9a7b) vs the crate's 15 slots
      (card #1d1c19, accent #c4746e, ok #87a987); Framework keeps its
      own table until Brandon rules canonical-generator-vs-keep-table.
      (2) `prefer_dark_chrome()` is not exported; the nudge rides
      inside `theme_install` via the portal's documented global side
      effect, which covers the C case. (3) The icons module (tray theme
      installer + search-path probe) remains a recorded candidate,
      unstarted. Framework adopts at its 1.0.1 (decision 29); the
      MIT-into-GPL attribution note is carried in capi/README.md and the
      capi crate docs.)*
- [ ] **GitHub presentation (workspace batch):** description empty,
      topics null, zero Releases, README pins branch=main instead of the
      tag, no consumer list - proposals drafted in the ledger.
- [x] **gtk4 0.11 platform bump (1.3.0):** gtk4 0.9 → 0.11 (glib/gio →
      0.22); portal.rs ports `signal_subscribe` to `subscribe_to_signal`
      with the strong `SignalSubscription` held process-lifetime; the
      `v4_14` pin is unchanged and no API surface moves. Cascade:
      Atrium adopts in the same wave (decision 61, before its 1.0.0
      tag); Conservatory and Viaduct are WAIVED to adopt at their next
      releases - 1.3.0 changes no API they call, so both compile
      against 1.2.0 until then. *(Shipped 2026-09-13; suite 33 green,
      clippy clean.)*
