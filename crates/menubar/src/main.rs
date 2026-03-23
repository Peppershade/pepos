mod config;
mod render;
mod shm;
mod state;

use state::AppState;
use wayland_client::Connection;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-menubar starting");

    // Connect to the running Wayland compositor via $WAYLAND_DISPLAY.
    let conn = Connection::connect_to_env()
        .expect("could not connect to Wayland — is a compositor running?");

    let mut event_queue = conn.new_event_queue::<AppState>();
    let qh = event_queue.handle();

    // Ask the compositor to list everything it supports (wl_compositor, wl_shm, etc.)
    conn.display().get_registry(&qh, ());

    let mut state = AppState::new();

    // Roundtrip 1: compositor sends us the global list, we bind what we need.
    event_queue.roundtrip(&mut state).expect("initial roundtrip failed");

    // Now that we have wl_compositor and zwlr_layer_shell_v1, create our surface.
    state.setup_surface(&qh);

    // Roundtrip 2: compositor processes our surface setup and sends a configure event,
    // which triggers our first render inside AppState::render_and_commit.
    event_queue.roundtrip(&mut state).expect("configure roundtrip failed");

    tracing::info!("menubar visible at {}×{}", state.width, state.height);

    // Main loop: wait for events (clock updates, resize, close) and handle them.
    while state.running {
        event_queue.blocking_dispatch(&mut state).expect("dispatch error");
    }
}
