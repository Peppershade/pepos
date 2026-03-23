// AppState holds every Wayland object we own, plus our rendering state.
//
// The Dispatch impls below are Rust's way of saying "when the compositor sends
// event X to object Y, run this code". Think of it like signal handlers, but
// type-safe and generated from the protocol XML.

use wayland_client::{
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::WlCompositor,
        wl_registry::{self, WlRegistry},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::{self, WlSurface},
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

use crate::config::Config;
use crate::render::Renderer;
use crate::shm::ShmBuffer;

pub struct AppState {
    // ── Wayland globals (bound during registry roundtrip) ──────────────────
    pub compositor: Option<WlCompositor>,
    pub shm: Option<WlShm>,
    pub layer_shell: Option<ZwlrLayerShellV1>,

    // ── Our surface objects ────────────────────────────────────────────────
    pub surface: Option<WlSurface>,
    pub layer_surface: Option<ZwlrLayerSurfaceV1>,

    // ── Pixel buffer in shared memory ─────────────────────────────────────
    pub shm_buffer: Option<ShmBuffer>,

    // ── Dimensions (compositor tells us width via configure event) ─────────
    pub width: u32,
    pub height: u32,

    pub running: bool,
    pub config: Config,
    pub renderer: Option<Renderer>,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::load();
        let height = config.menubar.height;
        AppState {
            compositor: None,
            shm: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            shm_buffer: None,
            width: 0,
            height,
            running: true,
            config,
            renderer: None,
        }
    }

    /// Create the layer-shell surface. Call this after the first roundtrip
    /// so that globals are already bound.
    pub fn setup_surface(&mut self, qh: &QueueHandle<AppState>) {
        let compositor = self.compositor.as_ref()
            .expect("wl_compositor not found — compositor must support it");
        let layer_shell = self.layer_shell.as_ref()
            .expect("zwlr_layer_shell_v1 not found — use a wlroots compositor (Sway, etc.)");

        // A wl_surface is a rectangular canvas the compositor will display.
        let surface = compositor.create_surface(qh, ());

        // Upgrade it to a layer-shell surface so it lives in the shell UI layer
        // rather than the normal app-window layer.
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,                            // output — None = use default/primary
            zwlr_layer_shell_v1::Layer::Top, // above normal windows, below overlays
            "pepos-menubar".to_string(),     // namespace identifier
            qh,
            (),
        );

        // Anchor all three top edges so the bar spans the full screen width.
        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);

        // Width 0 = "fill the anchored edges". Height is our bar height.
        layer_surface.set_size(0, self.config.menubar.height);

        // Exclusive zone = how many pixels to reserve. Windows won't be placed here.
        layer_surface.set_exclusive_zone(self.config.menubar.height as i32);

        // The menu bar doesn't need keyboard focus.
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        // Commit applies everything we just set.
        surface.commit();

        self.surface = Some(surface);
        self.layer_surface = Some(layer_surface);
    }

    /// Render a frame and send it to the compositor.
    pub fn render_and_commit(&mut self, qh: &QueueHandle<AppState>) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        // Clone the WlShm handle so we can borrow self mutably below.
        // WlShm is a ref-counted proxy — cloning is cheap.
        let shm = match self.shm.clone() {
            Some(s) => s,
            None => return,
        };

        // Recreate renderer + buffer if this is the first render or dimensions changed.
        let needs_new = self.shm_buffer.as_ref()
            .map(|b| b.width != self.width || b.height != self.height)
            .unwrap_or(true);

        if needs_new {
            self.renderer = Some(Renderer::new(self.width, self.height, &self.config));
            self.shm_buffer = Some(ShmBuffer::new(&shm, self.width, self.height, qh));
        }

        let renderer = self.renderer.as_ref().unwrap();
        let shm_buf = self.shm_buffer.as_mut().unwrap();
        let surface = self.surface.as_ref().unwrap();

        renderer.render(shm_buf.mmap.as_mut(), &self.config);

        // Attach the pixel buffer and tell the compositor the whole surface changed.
        surface.attach(Some(&shm_buf.buffer), 0, 0);
        surface.damage(0, 0, self.width as i32, self.height as i32);
        surface.commit();
    }
}

// ── Dispatch implementations ──────────────────────────────────────────────────
//
// In Wayland, the client never polls for events — the compositor pushes them.
// Each impl below handles one protocol object's events.

impl Dispatch<WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(
                        registry.bind::<WlCompositor, _, _>(name, version.min(4), qh, ()),
                    );
                }
                "wl_shm" => {
                    state.shm = Some(
                        registry.bind::<WlShm, _, _>(name, version.min(1), qh, ()),
                    );
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(
                        registry.bind::<ZwlrLayerShellV1, _, _>(name, version.min(4), qh, ()),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            // Configure = compositor telling us our final dimensions.
            // We MUST ack_configure before the surface becomes visible.
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                layer_surface.ack_configure(serial);
                if width > 0 {
                    state.width = width;
                }
                if height > 0 {
                    state.height = height;
                }
                tracing::debug!("configure: {}×{}", state.width, state.height);
                state.render_and_commit(qh);
            }
            // Closed = compositor is removing our surface (e.g. output unplugged).
            zwlr_layer_surface_v1::Event::Closed => {
                tracing::info!("layer surface closed by compositor");
                state.running = false;
            }
            _ => {}
        }
    }
}

// The remaining impls handle objects whose events we don't act on.
// They're required by the type system — every object we create needs a Dispatch impl.

impl Dispatch<WlCompositor, ()> for AppState {
    fn event(_: &mut Self, _: &WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShm, ()> for AppState {
    fn event(_: &mut Self, _: &WlShm, event: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        // The compositor advertises supported pixel formats here.
        // We always use ARGB8888 which is universally supported, so we just log.
        if let wl_shm::Event::Format { format } = event {
            tracing::trace!("shm format available: {:?}", format);
        }
    }
}

impl Dispatch<WlShmPool, ()> for AppState {
    fn event(_: &mut Self, _: &WlShmPool, _: wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlBuffer, ()> for AppState {
    fn event(_: &mut Self, _: &WlBuffer, event: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        // Release = compositor is done reading this buffer, we can reuse it.
        if let wl_buffer::Event::Release = event {
            tracing::trace!("buffer released");
        }
    }
}

impl Dispatch<WlSurface, ()> for AppState {
    fn event(_: &mut Self, _: &WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for AppState {
    fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
