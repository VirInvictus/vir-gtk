# vir-gtk Roadmap

- [x] Extract `theme.rs` and `color_scheme.rs` logic from the desktop apps.
- [x] Standardize `portal` logic to handle optional settings injection and fallbacks.
- [x] Support multiple injection patterns (Custom CSS properties & String token replacement).
- [ ] Explore native Wayland color-scheme read protocols when they land in `gio`.

## Phase 2: Component Consolidation & Ecosystem Parity
*Context: Centralizing duplicated base CSS and plain-GTK widget replacements (rows, dialogs, clamp) currently scattered across Atrium, Conservatory, Viaduct, Colophon, and Framework.*

- [x] **Centralized Base Widget Stylesheet:** Export `vir_gtk::theme::base_css()` covering standard widgets, typography (`.title-1`), and focus rings. Eliminates 800+ lines of duplicated CSS across downstream apps. *(2026-09-04 survey, totals re-measured 2026-09-05: a ~145-line base sheet is copied across Atrium (theme.rs, 272 lines), Conservatory (theme.rs, 235), and Viaduct (theme.rs, 224); the shared fraction is roughly 53-65% of each, not the 72-76% first computed against stale totals; re-measure the base sheet itself before scoping. Atrium's data/style.css is 1,012 lines (951 at survey time), ~all app-specific. DESIGN BRIEF before any code: (1) radius and checked/selected semantics diverge deliberately (Atrium rounded + paint-accent, Conservatory/Viaduct square, Conservatory "lift not paint"), so base_css() needs radius/checked-style parameterization or per-app override guarantees; (2) Atrium's data/style.css consumes @define-color adwaita names defined only in Atrium's own sheet: that block must move into vir-gtk or stay app-side or 1,012 lines lose their colors; (3) token mechanism splits 1-of-3: only Viaduct consumes var(--c-*), Atrium/Conservatory use %TOKEN% hexes, and Conservatory pins gtk4 v4_14 partly to skip custom properties: pick tokens or require a feature bump; (4) install order is the only override mechanism today (USER+1 everywhere, later-wins) and Atrium depends on it; a shared sheet must install before app sheets and never re-install after them; (5) Viaduct hand-reimplements the provider-swap this crate now provides (1.0.3): subsume it. Shipped 1.1.0 (2026-09-06), the four brief calls resolved as approved: (1) no parameters: the base ships the unanimous square/flat core (button-checked paint is the 2-of-3 majority; row selection differs in all three so it is excluded entirely) and app sheets override; (2) @define-color stays app-side; (3) tokens, not var(); Conservatory's v4_14 pin respected, Viaduct keeps var() in its app sheet over the properties block; (4) the ladder replaces timing: install_app_stylesheet (USER+2) outranks the crate tier (USER+1) by construction; (5) Viaduct's hand-rolled provider swap is subsumed by the crate tiers. Base is ~120 lines; consumer adoption landed in the 1.1.0 wave.)*
- [x] **Managed Stylesheet Lifecycle:** Create `StyleManager` to safely swap `GtkCssProvider` instances per `GdkDisplay` without memory leaks during live theme switches. *(Partially shipped in 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display, closing the leak. 1.1.0: the tracking is per (display, tier) and `install_app_stylesheet` adds the app tier. Shipped 1.2.0 (2026-09-10): the tracking becomes the named `vir_gtk::style::StyleManager` API. A handle is a key to one rung of the ladder, not an owner: `crate_tier()` (USER + 1), `app_tier()` (USER + 2), or `at_priority(prio)` for layers above the app sheet (Conservatory's runtime accent provider lives at USER + 3 and becomes formally manageable instead of hand-rolled state), each with `install` (replace), `remove` (explicit teardown, the one capability the tracking lacked), `is_installed`, and `priority`. Clones share the rung; dropping every handle uninstalls nothing. `install_stylesheet`/`install_app_stylesheet` stay in `theme` as one-line delegates, so no consumer import changes; the (display, tier) registry moved with the manager into `style`.)*
- [x] **Thread-Safe Portal State:** Replace `thread_local!` state with thread-safe atomics / `Arc<RwLock>` so background threads in Colophon and Conservatory can safely query `is_dark()`. *(Shipped in 1.0.3 as the shape the constraints allow: the boolean state is global atomics readable from any thread; the GTK-bound pieces (settings, connection, listeners) stay main-thread thread-locals because they are `!Send`. Phase 3's desync bug is the same fix.)*
- [ ] **Extended Palette Support:** Add Kanagawa Wave, secondary charting roles, 6-hue swatches, and a `ThemeRegistry` (Gruvbox, Nord) to allow Colophon to fully migrate to `vir-gtk`.
  *(DECIDED 2026-09-12 (Brandon): deferred until Colophon actually asks to migrate; the box stays open as on-demand.)*
- [x] **Cairo & GDK Color Helpers:** Add hex-to-RGBA/Cairo conversion tools (`to_gdk_rgba`, `to_cairo_rgba`) and automatic redraw queuing to clean up Conservatory's waveform/spectrum and Colophon's charts. *(Shipped 1.1.0 (2026-09-06): `vir_gtk::color` with strict-hex `to_gdk_rgba`/`to_cairo_rgba` and `redraw_on_theme_change`, a weakly-held redraw hook on portal dark/light flips. The 2026-09-04 gate dissolved in the design: the cairo values are plain f64 triples, so this crate needs no cairo dependency at all; `cairo-rs` stays transitive via gdk4 where consumers link it.)*
- [ ] **Shared Plain-GTK4 Component Kit:** Introduce `vir_gtk::widgets` containing shared implementations for `action_row`, `combo_row`, `Alert` dialogs, `StatusPage`, and `Clamp`. Eliminates duplicated widgets across Atrium, Conservatory, and Viaduct. *(2026-09-04 survey: ~1,300 lines of near-verbatim Rust duplication: row builders (Atrium rows.rs 302 / Conservatory 268 / Viaduct 212, `build_row` verbatim x3), StatusPage composites x3 (three shapes, one concept), alert dialogs x3 (Atrium/Conservatory APIs identical, 131-line diff is docs; Viaduct's is parent-bound), toast revealer + newest-wins timeout x3. Atrium-only: Clamp, Page/Bin. Drift to reconcile: entry_row signatures differ, group() returns struct vs tuple.)*
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
- [x] **Thread-Local State Desync:** Prevent background async workers from reading uninitialized `thread_local!` state and assuming light mode. *(Shipped 1.0.3: boolean state moved to global atomics; the direction note in the old text was wrong: the thread-local defaults were dark, not light. Phase 2's thread-safety item records the full shape.)*
- [x] **Synchronous D-Bus Blocking:** Avoid blocking the GTK main thread during synchronous `xdg-desktop-portal` reads on startup. *(Shipped 1.0.3: the portal read is async (`gio::bus_get` + `DBusConnection::call`), with the Read-fallback for older portals on the same path; `init()` returns without blocking.)*
- [x] **Theme Light Mode Override:** Fix `portal_scheme_is_dark(0)` overriding the `default_dark` configuration when the portal expresses no preference. *(Shipped 1.0.3: `portal_scheme_preference` maps 1→dark, 2→light, anything else to None, and None falls back to `default_dark`.)*
- [x] **GtkCssProvider Leak:** Properly remove old `GtkCssProvider` instances from the `GdkDisplay` before appending new ones during dynamic theme switches. *(Shipped 1.0.3: `install_stylesheet` tracks and replaces the crate's last provider per display; API unchanged.)*

### Refactoring & Growth
- [x] **Clean Up D-Bus Connection:** Remove unused static `BUS` connection retention. *(Resolved in 1.0.3 by making retention load-bearing: the async redesign keeps the connection alive for the signal subscription, renamed `CONNECTION` with the invariant documented; the dead-write state no longer exists.)*
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
- [x] **1.4.0 Consumer Wave (widget-kit first slice + the deferred gtk4 0.11 platform waiver, cascade closed):** release `6deeec9`, tag `v1.4.0` cut verbatim and pushed, crate CI `34780806721` green (fmt + clippy + xvfb test, now spanning the `capi` member). One adoption commit per consumer, each with the lock bump, the code adaptation, a patchnotes line, and a green local suite: Viaduct `ffcd2d6` (deletes `rows.rs` + `alert.rs` entirely, kit re-exported as `ui::rows`; rides gtk4 0.9→0.11 + webkit6 0.4→0.6 per the 1.3.0 waiver; `ViaductWindow`'s wrapper! lists every interface per the Atrium precedent; suite 220 green), Conservatory `aa8cb95` (deletes `dialogs.rs` + the shared subset of `rows.rs`, keeps `Expander`; rides gtk4 0.9→0.11 per the same waiver; workspace 657 + music-only 69 green), Atrium `11a777a` (deletes the shared subset of `rows.rs` + the hand-rolled `Alert`; `dialogs.rs` becomes the kit re-export shim carrying the app-side tokio `AlertChoose::choose_future`; `data/cargo-sources.json` regen; suite 995 + `scripts/regression.sh` PASS). Consumer pushes ride their own repos' batch gates, after this crate's push. The GitHub presentation pass (decision 60) applied the same day; see its box.

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
      note records that gap as-is.)*
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
      (capture phase, weakly-held window, the 2-of-3 shape), and eight
      `#[gtk::test]` widget tests plus one plain test ported from the
      consumers' suites.
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
      release. THREE ITEMS RESOLVED OR RECORDED (updated 2026-09-13,
      evening): (1) `palette_css` is NOT exported and the question is
      now SETTLED (Brandon): Framework KEEPS ITS OWN TABLE, no canonical
      block generator - the capi never grows palette_css unless
      Framework's roadmap reopens it. (2) `prefer_dark_chrome()` is not
      exported; the nudge rides inside `theme_install` via the portal's
      documented global side effect, which covers the C case. (3) The
      icons module (tray theme installer + search-path probe) remains a
      recorded candidate, unstarted. Framework adopts at its 1.0.1
      (decision 29); the MIT-into-GPL attribution note is carried in
      capi/README.md and the capi crate docs.)*
- [x] **GitHub presentation (workspace batch):** description empty,
      topics null, zero Releases, README pins branch=main instead of the
      tag, no consumer list - proposals drafted in the ledger.
      *(Applied 2026-09-13 (decision 60): description drafted in the
      Atrium house voice; ten topics (gtk4, gtk, rust, kanagawa, theme,
      dark-mode, dbus, linux-desktop, local-first, widget-kit); Releases
      created for all six tags on the remote (v1.0.3 through v1.4.0),
      each body the verbatim patchnotes entry with the v1.4.0-shaped
      title `vX.Y.Z (date)`, v1.4.0 Latest; wiki off; discussions on.
      v1.0.0-v1.0.2 predate the repo's tags and stay release-less.
      (DECIDED 2026-09-13 (Brandon): the backfill is REJECTED; the three
      stay untagged and release-less under the forward-only policy, per
      the Viaduct v3.2.1/v3.3.1 precedent. The releases page tells the
      true tagged history from v1.0.3 onward.) The README branch=main
      pin question is settled 2026-09-15 the documented-keep way:
      Installation explains the branch-tracking + lock-rev consumption
      model (upgrades are explicit cargo update events) and names the
      tag-pinned variant, instead of repointing the example at a tag.)*
- [x] **gtk4 0.11 platform bump (1.3.0):** gtk4 0.9 → 0.11 (glib/gio →
      0.22); portal.rs ports `signal_subscribe` to `subscribe_to_signal`
      with the strong `SignalSubscription` held process-lifetime; the
      `v4_14` pin is unchanged and no API surface moves. Cascade:
      Atrium adopts in the same wave (decision 61, before its 1.0.0
      tag); Conservatory and Viaduct are WAIVED to adopt at their next
      releases - 1.3.0 changes no API they call, so both compile
      against 1.2.0 until then. *(Shipped 2026-09-13; suite 33 green,
      clippy clean.)*

### Final audit 2026-09-13 (THE FINAL AUDIT: NEW findings, one line each; full detail in audit-final/vir-gtk/FINAL-REPORT.md)
- [x] MED — close_on_escape never sets a propagation phase: runs BUBBLE while rustdoc/spec/patchnotes/roadmap all claim CAPTURE, and Viaduct's adoption silently dropped the capture behavior it originally added for a recorded real failure (widgets.rs:36-54; verified against Viaduct ffcd2d6~1). Set PropagationPhase::Capture + a pinning test. *(Executed 1.4.2: the controller sets Capture and a test inspects the installed phase.)*
- [x] MED — Alert responded-latch never reset: a second present() emits nothing against the documented "exactly one response id per presentation" (widgets.rs:330, 519-544). Reset in present() (cascade-free today) or fix the three doc sites. *(Executed 1.4.2, the code fix: present() resets the latch; a test pins once-per-presentation and answers-again.)*
- [x] MED — CONFIRMED still open from Wave 10, never recorded fixed: scope_css splits selector lists on bare commas (style.rs:258-263, corrupts `:is(a, b)`); at_priority accepts USER+4 colliding with a live StyleScope provider (style.rs:134-136,170). *(Executed 1.4.2: scope_css splits at parentheses depth zero (tests pin the :is case and a mixed list); at_priority warns on the vir-gtk domain at USER + 4, documented scope-owned.)*
- [x] MED — bind_settings builds a reference cycle (scope ↔ settings ↔ closure): any bound scope leaks for process lifetime (style.rs:351-358); doc at :349-350 claims "its own lifetime". Break the cycle or reword. *(Executed 1.4.2: the scope holds the settings weakly (the cycle is broken; the changed handler still keeps the scope's teardown state alive until the target's destroy); the doc states the real shape. No direct regression test: constructing a gio::Settings needs an installed schema the test suite does not have; the fix is structural and the audit itself verified the cycle by code-reading.)*
- [x] MED — Ladder-overpromise rustdoc survives in three sites (theme.rs:139-141, :151-153, style.rs:121-122): "app rules always win by construction" refuted by the base template's own window.csd rule; port the specificity caveat the 1.4.0 note claims is recorded. *(Executed 1.4.1, alongside the label-coverage fix below whose discovery re-proved the point live.)*
- [x] MED — install_default rustdoc omits the display precondition both C twins document (theme.rs:169-187 vs capi lib.rs:59, vir-gtk.h): pre-display calls silently no-op until the first portal flip. *(Executed 1.4.2: the rustdoc states the precondition and names the C twins.)*
- [x] MED — Add a contract test pinning capi/vir-gtk.pc's Version to CARGO_PKG_VERSION (the one deliberate second version carrier, hand-synced since 1.4.0, unpinned). *(Executed 1.4.2, and the pin proved its worth immediately: the .pc had drifted to 1.4.0 at the 1.4.1 release; the test landed with the file corrected and it moved with the 1.4.2 bump.)*
- [x] MED — GitHub: no branch/tag rulesets while three consumers track main by rev (force-push would orphan them; block force-push + deletion only, no required checks); README Installation documents no versioning story (add lock-rev note + tag-pin variant, or record branch=main as deliberate); CI lacks a concurrency group and fedora:latest floats. *(Executed 2026-09-15 per Brandon's gate answers: a ruleset on main (non-fast-forward + deletion blocked, bypass Brandon always, no required checks, ruleset 23470630) is ACTIVE; the tag half hit a platform wall, recorded in its own box below; the README versioning story landed (documented-keep: lock-rev consumption model + tag-pin variant named); CI gained the concurrency group and the fedora:latest float is recorded as deliberate in the workflow.)*
- [x] MED — roadmap.md:173-179 GitHub ship-note tail garbled (stray `*`, "The README now pins the Consumers section added this release"), leaving the branch-pin proposal dispositionless; rewrite. Roadmap em-dashes survive at :11 (nine), :17, :31, :37. *(Executed 1.4.2: the tail is rewritten with the README disposition recorded; all twelve em-dashes on those four lines are recast.)*
- [x] LOW — Comment truth batch: stale "No test initializes GTK" premise (style.rs:495-497); portal.rs:80 copy-residue parenthetical, :147 "Safe from the main thread" ambiguity, :332 bump doc, :36 SETTINGS retention comment; widgets.rs:363-365/:591 garbled sentences; capi SLOTS doc wrong reason + broadcast test order dependence; color.rs public docs list one hex form where tests pin four. *(Executed 1.4.2, every site. The broadcast-order item was fixed harder than noted: the race fired live during the blitz (the test lost its name-sort luck once in five runs), so the capi test now starts from a pristine portal state via a test-only reset instead of relying on sort order.)*
- [x] LOW — Docs smalls: spec.md:36 stranded Priority Injection bullet (Wave-10 residual); README/CLAUDE.md state Framework capi adoption as current fact (pending); CLAUDE.md:26 "CI runs all four"; lib.rs "four things" vs five modules; "radius" exclusion-list wording could strip the square-zeroing base rules; roadmap "nine #[gtk::test]" is eight+one; spec.md:51 blanket non-activatable claim vs button_row; patchnotes unsettled-note rider (frozen record, note settlement next entry). *(Executed 1.4.2: bullet deleted, both adoption mentions read as pending, CI line names three commands, lib.rs ships five things, the exclusion list says radius divergences and names plain label text, the count is corrected, rows say non-activatable except button_row, and the 1.4.2 entry carries the settlement rider.)*
- [x] LOW — Housekeeping: .gitignore lacks editor/OS patterns; no rust-version (MSRV) field; revisit widgets/ split when kit slice 2 is scoped; portal silent wrong-typed-inner-value drop deserves a warn (degrade-loudly only covers total failure); capi NULL-callback guard; has_raw_token test helper duplication; set_choice double-apply churn. *(Executed 1.4.2: .gitignore aligned, rust-version 1.92 declared, the portal reply-drop warns, the capi rejects NULL, and the raw-token helper is shared. set_choice double-apply is RECORDED NOT FIXED: the churn is two provider installs per bound set_choice, invisible and audit-called harmless, and the zero-risk dedup depends on gio signal synchronicity, a userspace risk with no consumer on the API yet. The widgets/ split stays gated on slice 2 being scoped, which the blitz ruling defers.)*
- [x] LOW — Prose: README:5 benefit paragraph ("pixel-perfect", "instantly", "bespoke" + dangling modifier) and :22 "ensuring no memory leaks" guarantee contradicted by the StyleScope leak finding; v1.0.0 patchnotes adverb cluster ("securely", "automatically … safely"); "honest" self-descriptor tic; thrice-verbatim module-coverage parenthetical in README:9-11. *(Executed 1.4.2: all four; the v1.0.0 entry is safe to recast because the backfill ruling left it untagged, so no frozen record is touched.)*
- [x] Feature candidates logged (FINAL-REPORT L4, ranked): kit slice 2 (StatusPage + Toast; carries the MED fixes as ride-alongs so consumers pay one wave); base_css switch/check/scale family (Viaduct renders system chrome today); install_default_with(palette_fn) variant; icons module (thin); high-contrast (L, Brandon-gated broadcast-API question). *(Ruled 2026-09-15 (Brandon, blitz gates): kit slice 2 DEFERRED, recorded as this repo's reopen condition in project.done, not built this blitz (the MED fixes shipped as 1.4.2 instead); the Viaduct StatusPage registrable-class-vs-shim ruling therefore stays open with it. base_css switch/check/scale and install_default_with stay slot-as-boxes ungreen-lit; icons stays a recorded candidate; high-contrast stays parked.)*

- [x] MED — (found 2026-09-15 in Viaduct's force-light QA) base_css never pins plain `label` text, so a dark third-party `gtk-theme-name` supplies explicit label colors at theme priority that beat inheritance: light Kanagawa backgrounds rendered with dark-theme label text, washed to unreadable. Fix: `label { color: %FG% }` + disabled dim in the base template; proven with a `GTK_THEME=Adwaita:light` A/B. *(Executed 1.4.1; Viaduct verified live both modes.)*

**CONFIRMED-prior (final-audit verification):** Priority Injection bullet, scope_css comma split, at_priority collision, ladder overpromise (partially fixed), README branch-pin residual. SUPERSEDED (verified shipped): all Wave-10 fixes the ship notes claimed (rustdoc debt, metadata, missing_docs, portal rustdoc, Palette docs, GitHub pass, CI residue, tracing dep). Audit-side corrections for the manager: the vir-gtk sheet's suite/CI/version-sync lines are stale against 1.4.0 (51 tests, xvfb back, .pc carrier); the audit full-roadmap duplicates the verification section and names a phantom "spec §1.6". Slop-reader verdict: overwhelmingly human; three localized spots to pass.


## Consumer waves & cascade records (final blitz, 2026-09-15)

- [x] **v1.4.1 wave correction:** the manager's dispatch listed Atrium as
      not yet adopted for 1.4.1; verification found Atrium's own lane had
      landed it hours after the snapshot (commit `94c1e2b`, lock at
      `cc51428`, suite 1000 green, CI green, pushed 2026-09-14 21:34). So
      the whole 1.4.1 wave was already closed: Conservatory `6245c85` +
      `b352c16`, Viaduct lock at `cc51428` (v4.0.1, fix verified live),
      Atrium `94c1e2b`. Nothing to redo; recorded here and in the
      completion report as a ledger correction.
      *(Verified 2026-09-15.)*
- [x] **1.4.2 consumer wave (cascade closed):** release `21300ae`, tag
      `v1.4.2` cut verbatim and pushed, crate CI `34998293718` green, the
      GitHub Release live (plus the missing v1.4.1 Release backfilled
      from its tag message, same shape). One adoption commit per
      consumer, each with the lock bump, the cargo-sources regen, a
      patchnotes line, and a green suite: Atrium `b14a500` (suite 1000,
      CI green; the regen also fixed the 1.4.1 wave's skipped regen, so
      the manifest moved 1.4.0 pin → 21300ae in one commit),
      Conservatory `4741c2d` (48 binaries green, CI green, regen),
      Viaduct `b57e67a` (suite 227 green, CI green, regen). Cross-repo
      touches logged here and in each consumer's patchnotes per grant
      #117; Atrium's push rides its standing stage-close grant #86 and
      the blitz standing push grant covers the rest.
      *(2026-09-15.)*
- [x] **Framework capi disposition (cascade law, in writing):** Framework
      1.0.1 shipped WITHOUT the capi adoption (verified 2026-09-15: its
      tree has no `vir_gtk_*` references; only its roadmap mentions the
      rider). Framework's own roadmap keeps the adoption as a
      dependency-ordered open rider (its 1.0.1 fallback missed; it moves
      to a future Framework release). Ruled WAIVED/PENDING, owned by
      Framework's lane: this repo's capi surface is shipped, documented,
      and released; no Framework-side C code is built in this lane; the
      cascade closes here with the adoption pending on the consumer side
      and reopens if Framework's roadmap revives the rider.
      *(Recorded 2026-09-15.)*
- [x] **Blitz gates (Brandon, 2026-09-15, AskUserQuestion):** (1) PUSH:
      standing blitz grant through 2026-09-20 for this lane's pushes and
      consumer wave commits. (2) RULESETS: lightweight, main + v* tags,
      bypass Brandon, no required checks - main ruleset ACTIVE
      (non-fast-forward + deletion blocked, ruleset 23470630); the v*
      tag half is NOT POSSIBLE on this repo (see its box). (3) KIT SLICE
      2: fixes-only 1.4.2; slice 2 is the recorded reopen condition.
      (4) RELEASE AUTOMATION: keep manual, recorded; the verbatim rule
      makes a tag-triggered creator either a thin wrapper or a wrong
      substitute. (5) META: SECURITY.md only (landed); FUNDING.yml
      declined, the README Support section already carries it.
      *(All five asked and answered before execution.)*
- [x] **Tag-deletion protection: not possible on this repo, recorded.**
      GitHub push-target rulesets accept neither ref-name conditions nor
      the deletion rule (API 422s verified against the docs), and the
      classic protected-tags feature is Enterprise-only (endpoint 404 on
      this free-plan org repo). The catastrophic half of the finding
      (force-push orphaning three consumers' pinned revs) IS closed by
      the active main ruleset; the tag-deletion half is closed as
      platform-limited, revisit only if the org moves plans.
      *(Recorded 2026-09-15.)*
- [ ] **base_css switch/check/scale family + `install_default_with`
      variant:** stay slot-as-boxes (Brandon did not green-light them in
      the blitz gates). Natural first content for a post-blitz 1.5.0
      alongside kit slice 2.
- [ ] **Reopen conditions (recorded in project.done):** kit slice 2
      (StatusPage + Toast, carrying the Viaduct registrable-class
      ruling); high-contrast support (L-sized, broadcast-API design
      ruling needed); Framework capi adoption (Framework's rider);
      extended Palette/ThemeRegistry (waits on Colophon asking); icons
      module (waits on a third consumer).