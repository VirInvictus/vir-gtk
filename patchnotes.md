## v1.0.2 (2026-08-25)

- **Tests:** Added the crate's first unit tests (8): `Palette::dragon()` pinned to the Kanagawa Dragon reference hexes, Dragon/Lotus distinctness, full `%TOKEN%` substitution coverage (including the documented `%BG%`/`%HEADING%` gap from roadmap Phase 3), CSS custom-property generation, and the portal `color-scheme` value mapping plus theme-nick resolution helpers.

## v1.0.1 (2026-08-23)

- **Build:** build: add GitHub Actions CI workflow

# vir-gtk Patch Notes

## v1.0.0 (2026-08-22)

**Initial Extraction and Release**
`vir-gtk` has been extracted from Atrium, Conservatory, Viaduct, and Colophon into a standalone shared library. This centralizes the VirInvictus design idiom into a single repository and permanently removes the need for individual applications to duplicate D-Bus portal listening code or hardcode hex values.

*   **Portal Module**: Introduced the `portal` module to handle `org.freedesktop.portal.Settings` resolution. The module automatically syncs with the desktop environment's `color-scheme` broadcast, dropping dead weak references safely to prevent memory leaks in GTK's main loop.
*   **Theme Module**: Added the `theme` module containing the authoritative Kanagawa Dragon and Lotus color palettes. It supports CSS generation and string token replacement to securely inject themes into GTK4 `CssProvider` instances.
*   **Priority CSS**: The library strictly enforces CSS injection at `STYLE_PROVIDER_PRIORITY_USER + 1`, ensuring that `vir-gtk` styling always overrides standard desktop stylesheets while preserving the application's ability to selectively override properties.
