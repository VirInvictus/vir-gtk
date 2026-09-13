// SPDX-License-Identifier: MIT
//! The shared GTK4 styling and D-Bus portal layer for the VirInvictus
//! desktop suite: the libadwaita replacement.
//!
//! The crate exists to guarantee cross-app visual parity after the suite
//! removed `libadwaita`. It ships four things:
//!
//! - [`portal`] reads and tracks the freedesktop `color-scheme` preference
//!   over D-Bus and composes it with an application settings key, so apps
//!   answer system dark/light flips without duplicating portal boilerplate.
//! - [`theme`] is the definitive Kanagawa Dragon / Lotus palette set plus
//!   the CSS generation utilities (token replacement, custom properties,
//!   the shared [`theme::base_css`](theme::base_css) widget sheet) and the
//!   two-rung install ladder.
//! - [`style`] is the managed lifecycle over that ladder: named
//!   [`style::StyleManager`] rungs and class-scoped forced palettes
//!   ([`style::StyleScope`]).
//! - [`color`] turns palette hexes into GDK/cairo values for widgets that
//!   draw themselves, and [`widgets`] is the shared plain-GTK row and
//!   dialog kit (`ActionRow`/`SwitchRow`/`ComboRow`/`EntryRow`/`SpinRow`/
//!   `PreferencesGroup`/`AlertDialog` replacements).
//!
//! Two invariants govern everything: **no libadwaita** (this crate is its
//! replacement, not a wrapper), and the **install ladder is the override
//! mechanism** (crate sheets at `USER + 1`, application sheets at
//! `USER + 2`, scopes at `USER + 4`; see the [`style`] module docs for the
//! specificity caveat).
//!
//! A minimal application wires the portal and the shared sheet with one
//! call, then reacts to flips through the portal's listener (or lets
//! [`theme::install_default`] do both):
//!
//! ```no_run
//! use vir_gtk::theme;
//!
//! // Portal listener + a base sheet that re-splices on every flip.
//! theme::install_default(true);
//!
//! // Later, from any thread: the resolved state.
//! if vir_gtk::portal::is_dark() {
//!     println!("dark");
//! }
//! ```
//!
//! # Panics
//!
//! The library never panics into the host application: malformed portal
//! bodies, missing displays, and dead widgets all degrade (with a
//! `g_warning` on the `vir-gtk` log domain where the condition is
//! interesting) rather than unwinding.

#![warn(missing_docs)]

pub mod color;
pub mod portal;
pub mod style;
pub mod theme;
pub mod widgets;
