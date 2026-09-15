# vir-gtk

A standalone Rust library that provides the shared GTK4 styling and D-Bus portal interaction layer for the VirInvictus desktop suite.

`vir-gtk` exists to replace `libadwaita`. It provides the visual identity for `Atrium`, `Conservatory`, and `Viaduct`: a Kanagawa-themed stylesheet set installed under standard GTK4 widgets, plus the portal listener behind it. (Colophon and Framework still style themselves; the extended-palette migration waits on Colophon asking, and Framework's `capi/` adoption is pending.) The applications share one theme engine and one `org.freedesktop.portal.Settings` listener, so they render the same palette and follow system dark/light toggles without each carrying its own copy of the plumbing.

## Consumers

All three track this crate for the portal, the base sheet, the style lifecycle, and the widget kit:

- [Atrium](https://github.com/VirInvictus/Atrium): the GTK4 calendar/task manager.
- [Conservatory](https://github.com/VirInvictus/Conservatory): the audiobook/podcast player.
- [Viaduct](https://github.com/VirInvictus/Viaduct): the RSS reader.

Framework's adoption of the C API (`capi/`) is pending: it was planned for Framework 1.0.1, which shipped without it, and stays recorded in the roadmap's C-ABI box.

## Architecture and Capabilities

`vir-gtk` is divided into five primary modules:

### The Portal Module (`vir_gtk::portal`)

The portal module is responsible for reading and monitoring the system's preferred color scheme via the `org.freedesktop.portal.Settings` D-Bus interface.

It handles the complexity of composing the desktop's system color scheme against an application's internal preferences (for example, if a user sets the app to `force-dark` while the system is light). It exposes an `is_dark()` accessor and a change-listener registry that holds owners weakly, so callbacks bound to widgets that die simply drop out at the next broadcast. Malformed portal bodies degrade instead of panicking, stale read replies cannot overwrite fresher signal state, and the module re-reads when the portal process (re)starts, so changes made while it was down are picked up.

Note the one global side effect: on every composed change, the `gtk-application-prefer-dark-theme` key on the default `GtkSettings` tracks the resolved state.

### The Theme Module (`vir_gtk::theme`)

The theme module provides the definitive Kanagawa Dragon (dark) and Kanagawa Lotus (light) hex palettes used across the suite.

It exposes methods to inject these palettes into GTK4 applications. It supports generating standard GTK 4.16+ custom property blocks (e.g., `--c-bg-window`) or performing direct string token replacement (e.g., swapping `%BG_WINDOW%` for the hex code) on legacy CSS stylesheets.

It also ships `base_css()`, the shared flat, square base widget sheet (window chrome, headerbar, lists, buttons, entries, popovers, the Adwaita utility classes, and a scoped focus ring) spliced with a palette, plus a two-tier install ladder: `install_stylesheet` carries the crate's sheets at `USER + 1` and `install_app_stylesheet` carries the application's own sheet at `USER + 2`, so app-specific overrides win by priority rather than by install order.

### The Style Lifecycle Module (`vir_gtk::style`)

The named lifecycle API over that ladder. A `StyleManager` handle is a key to one rung: the crate tier (`USER + 1`), the app tier (`USER + 2`), or any explicit priority above (Conservatory's runtime accent provider lives at `USER + 3`). Each rung can `install` (replacing its previous sheet), `remove` (tearing it down), and be queried with `is_installed`; handles are keys, not owners, so dropping one never uninstalls anything.

The same module ships per-subtree forced palettes: a `ThemeChoice` (`system`, `dark`, or `light`, parsed from the nick forms a GSettings key holds) and a `StyleScope`, which pins one widget subtree (a window, or a page inside one) to the dark or light palette regardless of the system theme. While forced, the target wears the `vir-style-scope` class and a display provider at `USER + 4` serves `base_css` re-spliced with the forced palette plus your own `extra_css` template, all scoped under the class; `bind_settings` drives the choice from a writable string key, `System` hands the subtree back to the global ladder, and the target's destroy tears the provider down.

The ladder is a tie-breaker, not a trump: GTK CSS decides by specificity first and only falls back to provider priority for equal-specificity rules. Match the base rule's selector shape when overriding, or the app sheet's more general rule can lose to the base's more specific one.

### The Color Module (`vir_gtk::color`)

For widgets that draw themselves (charts, waveforms, spectrums): `to_gdk_rgba` and `to_cairo_rgba` turn the palette's hex strings into GDK and cairo values (strict CSS hex parsing; malformed input is `None`, not a panic), and `redraw_on_theme_change` re-queues a widget's draw when the portal flips dark/light. Cairo needs no dependency here: the cairo values are plain floats you pass to your own cairo context.

### The Widget Kit (`vir_gtk::widgets`)

The shared plain-GTK row and dialog builders: `row`/`action_row`/`switch_row`/`spin_row`/`combo_row`/`button_row`/`entry_row` and the `Group` box, plus `Alert`, the modal dialog replacement with named, styled responses and a close-response guarantee, and `close_on_escape`. Deliberately adwaita-shaped so call sites port mechanically; composite widgets like StatusPage and Clamp stay application-side.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vir-gtk = { git = "https://github.com/VirInvictus/vir-gtk.git", branch = "main" }
```

That branch pin is the suite's actual consumption model: consumers track `main` while their `Cargo.lock` pins the exact revision, so an upgrade is an explicit `cargo update -p vir-gtk` event (usually a coordinated adoption wave), never a silent drift. A consumer that prefers tags as the contract can pin a `v*` tag instead; every tag is a verbatim release record.

A C application consumes the same engine through the `vir-gtk-capi` cdylib (header and pkg-config file under [`capi/`](capi/)).

## Usage

The one-call setup: start the portal and keep the shared base sheet spliced with the palette the resolved state calls for, on every flip, forever.

```rust
use vir_gtk::theme;

fn main() {
    // true = fall back dark until the portal answers (and on "no preference").
    theme::install_default(true);
}
```

The manual equivalent, for applications that build their own sheets: `init()` seeds the state and the portal listener, and the callback re-splices on every flip. Without that loop (or `install_default`), the initial splice is the only one that ever happens and the app never follows the system again.

```rust
use vir_gtk::portal;
use vir_gtk::theme::{base_css, install_stylesheet, Palette};

fn resplice() {
    let palette = if portal::is_dark() {
        Palette::dragon()
    } else {
        Palette::lotus()
    };
    install_stylesheet(&base_css(&palette));
}

fn main() {
    // The arguments allow you to compose an app's gio::Settings key.
    portal::init(None, None, true);

    // The callback fires on the main loop for every state change. The
    // owner can be any GObject the application keeps alive; here, its
    // `gtk::Application` (declared elsewhere in a real app).
    portal::connect_dark_changed(&app, |_| resplice());

    // Re-splice once now, so startup is themed before the portal answers.
    resplice();
}
```

The style lifecycle module, for managed rungs and a subtree that forces its own palette (Conservatory's Now Playing full-screen staying dark in a light system theme):

```rust
use vir_gtk::style::{StyleManager, StyleScope};

// Managed ladder rungs: install replaces, remove tears down.
StyleManager::app_tier().install(&sheet);

// A page forced dark or light by a `gio::Settings` string key holding
// "system" / "dark" / "light".
let scope = StyleScope::builder(&now_playing_page)
    .extra_css(|p| p.replace_tokens(APP_TEMPLATE))
    .build();
scope.bind_settings(&settings, "now-playing-theme");
```

## Design Philosophy

The VirInvictus suite uses flat, calm interfaces built on the Kanagawa color palette. `vir-gtk` strips away the rounded corners, gradients, and heavy shadows of Adwaita in favor of sharp, distinct boundaries and muted contrast. The library assumes that the application will style standard `gtk::Box`, `gtk::HeaderBar`, and `gtk::Button` widgets manually using the injected custom properties.

## Support

If vir-gtk's useful to you and you'd like to chip in:

- liberapay · [liberapay.com/bdkl](https://liberapay.com/bdkl/)
- bitcoin
  ```
  bc1qkge6zr45tzqfwfmvma2ylumt6mg7wlwmhr05yv
  ```
