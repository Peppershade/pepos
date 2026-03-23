use wayland_client::{
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::{self, WlCompositor},
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
    pub compositor:   Option<WlCompositor>,
    pub shm:          Option<WlShm>,
    pub layer_shell:  Option<ZwlrLayerShellV1>,
    pub surface:      Option<WlSurface>,
    pub layer_surface: Option<ZwlrLayerSurfaceV1>,
    pub shm_buffer:   Option<ShmBuffer>,
    pub width:        u32,
    pub height:       u32,
    pub running:      bool,
    pub config:       Config,
    pub renderer:     Option<Renderer>,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::load();
        let height = config.bar_height();
        AppState {
            compositor: None, shm: None, layer_shell: None,
            surface: None, layer_surface: None, shm_buffer: None,
            width: 0, height, running: true, config, renderer: None,
        }
    }

    pub fn setup_surface(&mut self, qh: &QueueHandle<AppState>) {
        let compositor  = self.compositor.as_ref().expect("wl_compositor");
        let layer_shell = self.layer_shell.as_ref().expect("zwlr_layer_shell_v1");

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Top,
            "pepos-dock".to_string(),
            qh, (),
        );

        // Anchor to bottom, full width
        layer_surface.set_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, self.config.bar_height());
        layer_surface.set_exclusive_zone(self.config.bar_height() as i32);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        surface.commit();

        self.surface = Some(surface);
        self.layer_surface = Some(layer_surface);
    }

    pub fn render_and_commit(&mut self, qh: &QueueHandle<AppState>) {
        if self.width == 0 || self.height == 0 { return; }

        let shm = match self.shm.clone() { Some(s) => s, None => return };

        let needs_new = self.shm_buffer.as_ref()
            .map(|b| b.width != self.width || b.height != self.height)
            .unwrap_or(true);

        if needs_new {
            self.renderer    = Some(Renderer::new(self.width, self.height));
            self.shm_buffer  = Some(ShmBuffer::new(&shm, self.width, self.height, qh));
        }

        let renderer = self.renderer.as_ref().unwrap();
        let shm_buf  = self.shm_buffer.as_mut().unwrap();
        let surface  = self.surface.as_ref().unwrap();

        renderer.render(shm_buf.mmap.as_mut(), &self.config);
        surface.attach(Some(&shm_buf.buffer), 0, 0);
        surface.damage(0, 0, self.width as i32, self.height as i32);
        surface.commit();
    }
}

impl Dispatch<WlRegistry, ()> for AppState {
    fn event(state: &mut Self, registry: &WlRegistry, event: wl_registry::Event,
             _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor"      => { state.compositor  = Some(registry.bind::<WlCompositor,    _, _>(name, version.min(4), qh, ())); }
                "wl_shm"             => { state.shm         = Some(registry.bind::<WlShm,           _, _>(name, version.min(1), qh, ())); }
                "zwlr_layer_shell_v1"=> { state.layer_shell = Some(registry.bind::<ZwlrLayerShellV1,_, _>(name, version.min(4), qh, ())); }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(state: &mut Self, ls: &ZwlrLayerSurfaceV1, event: zwlr_layer_surface_v1::Event,
             _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                ls.ack_configure(serial);
                if width  > 0 { state.width  = width;  }
                if height > 0 { state.height = height; }
                state.render_and_commit(qh);
            }
            zwlr_layer_surface_v1::Event::Closed => { state.running = false; }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()>  for AppState { fn event(_:&mut Self,_:&WlCompositor, _:wl_compositor::Event, _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlShm, ()>         for AppState { fn event(_:&mut Self,_:&WlShm,         e:wl_shm::Event,        _:&(),_:&Connection,_:&QueueHandle<Self>){ let _ = e; } }
impl Dispatch<WlShmPool, ()>     for AppState { fn event(_:&mut Self,_:&WlShmPool,     _:wl_shm_pool::Event,   _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlBuffer, ()>      for AppState { fn event(_:&mut Self,_:&WlBuffer,      _:wl_buffer::Event,     _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlSurface, ()>     for AppState { fn event(_:&mut Self,_:&WlSurface,     _:wl_surface::Event,    _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<ZwlrLayerShellV1,()> for AppState { fn event(_:&mut Self,_:&ZwlrLayerShellV1,_:zwlr_layer_shell_v1::Event,_:&(),_:&Connection,_:&QueueHandle<Self>){} }
