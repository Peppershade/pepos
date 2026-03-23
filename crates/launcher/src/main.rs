// pepos-launcher
// Keyboard-driven app launcher — appears as a centered overlay.
// Searches installed applications by name.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-launcher starting");

    // TODO: connect to Wayland display
    // TODO: bind wlr-layer-shell as an overlay surface
    // TODO: render search input and results
}
