/* vir-gtk.h: C API over the shared VirInvictus GTK4 theming layer.
 *
 * Portal-driven dark/light theming for C GTK4 applications: the
 * replacement for Framework's manual `fw-theme.c` D-Bus port. Call from
 * the main thread only, like the rest of GTK. Typical use:
 *
 *     vir_gtk_theme_install (TRUE);
 *     vir_gtk_on_dark_changed (on_dark_changed, self, NULL);
 *
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <gtk/gtk.h>

G_BEGIN_DECLS

/* Install the theme: start the freedesktop portal listener, apply the
 * current color scheme, and keep the shared base stylesheet installed,
 * re-spliced on every dark/light flip. Call once, after a GdkDisplay
 * exists (i.e. from the application's startup, after gtk_init); later
 * calls re-splice and are otherwise no-ops.
 *
 * Side effect: the global GtkSettings:gtk-application-prefer-dark-theme
 * key tracks the resolved state. */
void vir_gtk_theme_install (gboolean default_dark);

/* The composed dark/light state (the portal's preference composed over
 * the application default). TRUE when dark. Safe from any thread. */
gboolean vir_gtk_is_dark (void);

/* The dark/light flip callback: fired with the resolved state on the
 * main loop thread. */
typedef void (*VirGtkDarkChanged) (gboolean dark, gpointer user_data);

/* Register callback to fire on every dark/light flip. callback must be
 * non-NULL (a NULL callback warns and returns 0, never a real id).
 * destroy, when given, runs on user_data exactly once at disconnection,
 * and no callback fires after vir_gtk_disconnect_dark_changed returns.
 * Returns the registration id to disconnect with. */
guint vir_gtk_on_dark_changed (VirGtkDarkChanged callback,
                               gpointer user_data,
                               GDestroyNotify destroy);

/* Remove the registration id; the destroy notify, if any, runs before
 * the call returns. Unknown ids are no-ops. */
void vir_gtk_disconnect_dark_changed (guint id);

G_END_DECLS
