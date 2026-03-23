mod apps;
mod config;
mod render;
mod shm;
mod state;

use state::AppState;
use wayland_client::Connection;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("pepos-launcher starting");

    // Load installed applications before connecting to Wayland
    let app_list = apps::load();
    tracing::info!("found {} apps", app_list.len());

    let conn = Connection::connect_to_env()
        .expect("could not connect to Wayland");

    let mut eq = conn.new_event_queue::<AppState>();
    let qh = eq.handle();
    conn.display().get_registry(&qh, ());

    let mut state = AppState::new(app_list);
    eq.roundtrip(&mut state).expect("roundtrip");
    state.setup_surface(&qh);
    eq.roundtrip(&mut state).expect("configure");

    tracing::info!("launcher ready — type to search, Enter to launch, Esc to close");
    while state.running {
        eq.blocking_dispatch(&mut state).expect("dispatch");
    }
}
