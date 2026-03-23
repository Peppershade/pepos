// pepos-compositor
// Wayland compositor built with Smithay.
// Manages windows, input, rendering, and the overall desktop session.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-compositor starting");

    // TODO: initialize Smithay backend (DRM/KMS for real hardware, winit for nested dev session)
    // TODO: set up Wayland socket and protocol handlers
    // TODO: run event loop
}
