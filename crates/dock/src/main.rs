// pepos-dock
// Application dock — anchored to the bottom of the screen via wlr-layer-shell.
// Shows pinned apps and running windows.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-dock starting");

    // TODO: connect to Wayland display
    // TODO: bind wlr-layer-shell and create a bottom-anchored surface
    // TODO: render dock contents
}
