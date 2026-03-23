mod config;
mod render;
mod shm;
mod state;

use state::AppState;
use wayland_client::Connection;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-dock starting");

    let conn = Connection::connect_to_env()
        .expect("could not connect to Wayland");

    let mut eq = conn.new_event_queue::<AppState>();
    let qh = eq.handle();
    conn.display().get_registry(&qh, ());

    let mut state = AppState::new();
    eq.roundtrip(&mut state).expect("roundtrip");
    state.setup_surface(&qh);
    eq.roundtrip(&mut state).expect("configure");

    tracing::info!("dock visible {}x{}", state.width, state.height);
    while state.running {
        eq.blocking_dispatch(&mut state).expect("dispatch");
    }
}
