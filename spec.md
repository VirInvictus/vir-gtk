# vir-gtk Specification

**Domain**: GUI framework and system integration.
**Language**: Rust (edition 2021).
**Frameworks**: GTK4 (`gtk4`), `gio`, `glib`.

## 1. Architecture and Scope

`vir-gtk` is a small dependency crate managing GTK4 visual rendering and D-Bus signaling for the VirInvictus Rust applications. It exists to guarantee cross-app visual parity after the intentional removal of `libadwaita` from the ecosystem. It provides no widgets itself. Instead, it provides the backend engines required to style standard GTK widgets properly.

### 1.1 Portal Resolution (`vir_gtk::portal`)

The `portal` module handles `org.freedesktop.portal.Settings` over an asynchronous `gio::bus_get`: `init()` never blocks the calling (main) thread, and the portal's answer arrives through the main loop.
- **System Parity**: It listens for the `color-scheme` key (0 = no preference, 1 = prefer dark, 2 = prefer light) broadcast by the desktop environment. A real preference (1 or 2) overrides the application default; "no preference" and unknown values do not — the caller's `default_dark` stands.
- **Fallback Constraints**: The portal degrades safely to a user-defined default if the D-Bus connection fails. This ensures applications do not crash in headless environments or non-standard Wayland sessions. The degradation is not silent: a failed session-bus connection emits a `g_warning` on the crate's `vir-gtk` log domain before the default stands.
- **Composition Engine**: Applications can pass a `gio::Settings` handle and a specific key (e.g., `"theme"`). The module composes this against the system preference so that explicit `force-dark` or `force-light` overrides take precedence over the system broadcast.
- **Threading**: The boolean state is global atomics, readable from any thread; the GTK-bound state (settings, connection, listeners) is main-thread thread-locals. Listener callbacks fire on the thread that runs the main loop, iterated by index so re-entrant broadcasts and mid-broadcast registrations are safe.

### 1.2 Themes (`vir_gtk::theme`)

Contains the definitive **Kanagawa Dragon** (dark) and **Kanagawa Lotus** (light) hex values.
- **CSS Variable Generation**: Generates generic custom properties (`to_css_custom_properties`) conforming to standard CSS variable notation (`--c-*`) to be parsed natively by GTK 4.16+.
- **Token Replacement**: Provides `replace_tokens` to inject colors into raw CSS strings for legacy applications or highly specific override blocks that do not use CSS variables. Every palette slot has a token, including `%BG%` and `%HEADING%`.
- **Shared Base Sheet**: `base_css(palette)` emits the shared flat, square widget core (window chrome, headerbar, lists and rows, the button family, entries, popovers, tooltips, scrollbars, the Adwaita utility classes, and the scoped focus ring) with the palette's hexes spliced in. It deliberately excludes the per-app divergences: radius, selection and checked semantics, `@define-color` blocks, `font-family` rules, toasts, and OSD surfaces all stay in the application's own sheet.
- **Priority Ladder**: `install_stylesheet` (USER + 1) carries the crate's sheets and `install_app_stylesheet` (USER + 2) carries the application's own; both are tracked per display and tier, so a re-install replaces its tier's provider instead of accumulating. Application rules override the shared base by priority, never by install timing. Both free functions are delegates over `vir_gtk::style::StyleManager` (§1.4), the named handle to a ladder rung.

### 1.3 Color Helpers (`vir_gtk::color`)

For custom-drawn surfaces (charts, waveforms, spectrums) that bypass CSS:

- `to_gdk_rgba(hex)` parses strict CSS hex (`#rgb`, `#rgba`, `#rrggbbaa` included) into a `gdk::RGBA`; `to_cairo_rgba(hex)` yields the 0.0-1.0 RGB triple cairo's source functions take. Malformed input is `None`, never a panic. No cairo dependency is introduced: consumers pass the triple to their own cairo context (`cairo-rs` is already transitive via `gdk4` where they link it).
- `redraw_on_theme_change(widget)` re-queues the widget's draw whenever the portal's composed dark/light state changes, holding only a weak reference so the hook dies with the widget.
- **Priority Injection**: Injects the resulting CSS at `STYLE_PROVIDER_PRIORITY_USER + 1` via `install_stylesheet`, which replaces the provider this crate previously installed on that display rather than accumulating providers across theme switches. This guarantees the injected theme reliably overrides the system's `~/.config/gtk-4.0/gtk.css` without breaking application-specific overrides.

### 1.4 Stylesheet Lifecycle (`vir_gtk::style`)

The provider-management layer over the priority ladder. Two invariants: the ladder is the override mechanism (the system `gtk.css` sits below `USER`; crate rungs install at `USER + 1`; application rungs at `USER + 2`; application runtime layers, such as Conservatory's accent provider, at `USER + 3`), and handles are keys, not owners (sheets are display-global state; dropping a handle changes nothing; teardown is explicit or structural).

- **`StyleManager`**: a handle to one rung of the ladder, tracked per (display, priority) on the default display. `crate_tier()` and `app_tier()` name the two rungs the §1.2 free functions delegate to; `at_priority(priority)` manages any rung above them. `install(css)` replaces the rung's provider (returning it; `None` without a display), `remove()` tears the rung down, `is_installed()` queries it, `priority()` reads its position. Clones share the rung; removing an uninstalled rung is a no-op.
- **`ThemeChoice`**: `System`/`Dark`/`Light`, the shape of a bound GSettings string key. `from_nick` accepts the portal resolver's nick forms plus the explicit `system`/`default`; unknown nicks are `None`, and the `StyleScope` binding warns on the `vir-gtk` log domain and stands on `System` (the portal's degrade-loudly rule). `forced()` yields the pinned palette state (`None` for `System`); `resolves_to(global_dark)` composes a forced choice over the portal's state.
- **`scope_css(css, class)`**: rewrites a flat stylesheet (rules and comments; no braces inside comments, no nested at-rules: the shape `base_css` and the consumer token templates emit) so its rules only match inside a subtree carrying `class`. Every selector is emitted in descendant form (`.{class} sel`) and with the class appended to its final compound (`sel.{class}`), so a rule whose subject is the scope root still matches. Comments and declarations pass through verbatim. CSS nodes that do not descend from the scope root in the CSS tree (tooltips, and popovers on some shells) keep the globally-installed look.
- **`StyleScope`**: a forced-palette scope over one widget subtree (a window, or a page inside one: the motivating Now Playing full-screen is a stack page). While the choice is forced, the target carries the `vir-style-scope` class and one display provider at `STYLE_SCOPE_PRIORITY` (USER + 4, above every global rung including runtime accent layers) serves the forced palette's `base_css` plus the builder's `extra_css` re-splice plus a root-canvas rule, all passed through `scope_css`; `System` removes the provider and the class. The scope re-applies only on choice changes; `System` tracks global flips through the app's own display-tier re-splice. `bind_settings(&settings, key)` makes a writable string key the source of truth (read at bind time, re-read on change) and the write-back target of `set_choice`; `choice()` reads. Widget-scoped `StyleContext` providers were rejected: deprecated since GTK 4.10 and scoped to the single widget anyway. The handle is a controller, not an owner: dropping it changes nothing, and the class-scoped provider dies structurally with the target's window.

## 2. API Contract

- `portal::init(settings: Option<gio::Settings>, key: Option<&str>, default_dark: bool)` must be called once during application startup. It returns without blocking; the portal's current scheme arrives asynchronously and re-resolves state when it lands.
- `portal::is_dark() -> bool` provides synchronous access to the currently resolved color state, safe from any thread.
- `portal::system_is_dark() -> bool` provides synchronous access to the raw system preference alone, before composition against the application's `gio::Settings`, safe from any thread.
- `portal::resolve_now()` re-runs the composition immediately from the stored settings key and the current system preference, and broadcasts to listeners only when the composed state changed. Safe from the main thread.
- `portal::connect_dark_changed(owner: &impl IsA<glib::Object>, f: F)` registers a callback that fires whenever the composed state changes; the owner is held as a weak reference, so callbacks bound to UI elements drop out when the element dies.
- `theme::base_css(palette: &Palette) -> String` returns the shared base widget sheet with `palette` spliced in; no `%TOKEN%` survives (test-enforced).
- `theme::install_app_stylesheet(css: &str) -> Option<CssProvider>` installs the application's sheet at `USER + 2`, replacing only its own tier's previous provider per display; `theme::install_stylesheet` remains the crate tier at `USER + 1`.
- `color::to_gdk_rgba(hex: &str) -> Option<gdk::RGBA>` and `color::to_cairo_rgba(hex: &str) -> Option<(f64, f64, f64)>` convert strict CSS hex for GDK and cairo drawing; `color::redraw_on_theme_change(widget)` queues redraws on portal dark/light flips through a weak reference.
- `style::StyleManager` manages one rung of the stylesheet ladder: `crate_tier()` (USER + 1) and `app_tier()` (USER + 2) name the built-in rungs, `at_priority(priority)` manages any rung above them; `install(css) -> Option<CssProvider>` replaces the rung's sheet, `remove()` tears it down, `is_installed() -> bool` queries it, and `priority() -> u32` reads its position. Handles are keys, not owners: sheets are display-global state, dropping a handle changes nothing, and clones share the rung.
- `style::ThemeChoice` is `System`/`Dark`/`Light`; `from_nick(nick) -> Option<ThemeChoice>` parses the settings nicks, `nick()` returns the canonical form, `forced() -> Option<bool>` is the pinned palette state, and `resolves_to(global_dark) -> bool` composes the final dark state.
- `style::scope_css(css, class) -> String` scopes a flat stylesheet to a subtree carrying `class` (descendant prefix plus class-appended root form; comments and declarations verbatim).
- `style::StyleScope::builder(target)` plus `extra_css(Fn(&Palette) -> String)` plus `build()` create a forced-palette scope over a widget subtree; `bind_settings(&gio::Settings, key)` binds a writable string key (unknown nicks warn and stand on `System`), `set_choice(ThemeChoice)` enforces a choice (writing back through the key when bound), and `choice()` reads it. While forced, the target wears `SCOPE_CLASS` (`vir-style-scope`) and a provider serves the scoped sheets at `STYLE_SCOPE_PRIORITY` (USER + 4).
