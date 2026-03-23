// The launcher state includes keyboard input handling via wl_seat/wl_keyboard.
// When a key is pressed, we update the search query, filter the app list, and redraw.

use wayland_client::{
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::{self, WlCompositor},
        wl_keyboard::{self, WlKeyboard},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::{self, WlSurface},
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

use crate::apps::App;
use crate::config::Config;
use crate::render::Renderer;
use crate::shm::ShmBuffer;

pub struct AppState {
    pub compositor:    Option<WlCompositor>,
    pub shm:           Option<WlShm>,
    pub layer_shell:   Option<ZwlrLayerShellV1>,
    pub seat:          Option<WlSeat>,
    pub keyboard:      Option<WlKeyboard>,
    pub surface:       Option<WlSurface>,
    pub layer_surface: Option<ZwlrLayerSurfaceV1>,
    pub shm_buffer:    Option<ShmBuffer>,
    pub width:         u32,
    pub height:        u32,
    pub running:       bool,
    pub config:        Config,
    pub renderer:      Option<Renderer>,

    // Search state
    pub query:    String,
    pub apps:     Vec<App>,
    pub filtered: Vec<usize>, // indices into apps that match query
    pub selected: usize,
}

impl AppState {
    pub fn new(apps: Vec<App>) -> Self {
        let config = Config::load();
        let width  = config.launcher.width;
        let height = config.panel_height();
        let n = apps.len();
        AppState {
            compositor: None, shm: None, layer_shell: None,
            seat: None, keyboard: None,
            surface: None, layer_surface: None, shm_buffer: None,
            width, height, running: true, config, renderer: None,
            query: String::new(),
            filtered: (0..n).collect(), // all apps visible initially
            apps,
            selected: 0,
        }
    }

    pub fn setup_surface(&mut self, qh: &QueueHandle<AppState>) {
        let compositor  = self.compositor.as_ref().expect("wl_compositor");
        let layer_shell = self.layer_shell.as_ref().expect("zwlr_layer_shell_v1");

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay, // above everything
            "pepos-launcher".to_string(),
            qh, (),
        );

        // No anchor = centered on screen
        layer_surface.set_anchor(Anchor::empty());
        layer_surface.set_size(self.width, self.height);
        // Exclusive keyboard: all key presses come to us
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
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
            self.renderer   = Some(Renderer::new(self.width, self.height));
            self.shm_buffer = Some(ShmBuffer::new(&shm, self.width, self.height, qh));
        }

        let results: Vec<&App> = self.filtered.iter()
            .map(|&i| &self.apps[i])
            .collect();

        let renderer = self.renderer.as_ref().unwrap();
        let shm_buf  = self.shm_buffer.as_mut().unwrap();
        let surface  = self.surface.as_ref().unwrap();

        renderer.render(shm_buf.mmap.as_mut(), &self.config, &self.query, &results, self.selected);
        surface.attach(Some(&shm_buf.buffer), 0, 0);
        surface.damage(0, 0, self.width as i32, self.height as i32);
        surface.commit();
    }

    fn update_filter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered = self.apps.iter().enumerate()
            .filter(|(_, a)| a.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    fn launch_selected(&mut self) {
        if let Some(&idx) = self.filtered.get(self.selected) {
            let exec = &self.apps[idx].exec;
            let parts: Vec<&str> = exec.split_whitespace().collect();
            if let Some(cmd) = parts.first() {
                let _ = std::process::Command::new(cmd)
                    .args(&parts[1..])
                    .spawn();
                tracing::info!("launched: {}", exec);
            }
        }
        self.running = false;
    }

    pub fn handle_key(&mut self, scancode: u32, qh: &QueueHandle<AppState>) {
        match scancode {
            1  => { self.running = false; }  // Escape — close
            28 => { self.launch_selected(); } // Enter  — launch
            14 => {                            // Backspace
                self.query.pop();
                self.update_filter();
                self.render_and_commit(qh);
            }
            103 => {                           // Up arrow
                if self.selected > 0 { self.selected -= 1; }
                self.render_and_commit(qh);
            }
            108 => {                           // Down arrow
                let max = self.filtered.len().saturating_sub(1);
                if self.selected < max { self.selected += 1; }
                self.render_and_commit(qh);
            }
            _ => {
                if let Some(ch) = scancode_to_char(scancode) {
                    self.query.push(ch);
                    self.update_filter();
                    self.render_and_commit(qh);
                }
            }
        }
    }
}

// ── Scancode → character (US QWERTY, unshifted) ──────────────────────────────
// Using Linux input event keycodes. Good enough for typing app names.

fn scancode_to_char(code: u32) -> Option<char> {
    Some(match code {
        2  => '1', 3  => '2', 4  => '3', 5  => '4', 6  => '5',
        7  => '6', 8  => '7', 9  => '8', 10 => '9', 11 => '0',
        16 => 'q', 17 => 'w', 18 => 'e', 19 => 'r', 20 => 't',
        21 => 'y', 22 => 'u', 23 => 'i', 24 => 'o', 25 => 'p',
        30 => 'a', 31 => 's', 32 => 'd', 33 => 'f', 34 => 'g',
        35 => 'h', 36 => 'j', 37 => 'k', 38 => 'l',
        44 => 'z', 45 => 'x', 46 => 'c', 47 => 'v', 48 => 'b',
        49 => 'n', 50 => 'm',
        57 => ' ',
        _ => return None,
    })
}

// ── Dispatch implementations ──────────────────────────────────────────────────

impl Dispatch<WlRegistry, ()> for AppState {
    fn event(state: &mut Self, registry: &WlRegistry, event: wl_registry::Event,
             _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_compositor"       => { state.compositor  = Some(registry.bind::<WlCompositor,    _, _>(name, version.min(4), qh, ())); }
                "wl_shm"              => { state.shm         = Some(registry.bind::<WlShm,           _, _>(name, version.min(1), qh, ())); }
                "zwlr_layer_shell_v1" => { state.layer_shell = Some(registry.bind::<ZwlrLayerShellV1,_, _>(name, version.min(4), qh, ())); }
                // wl_seat gives us access to keyboard, pointer, touch
                "wl_seat"             => { state.seat        = Some(registry.bind::<WlSeat,          _, _>(name, version.min(5), qh, ())); }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for AppState {
    fn event(state: &mut Self, seat: &WlSeat, event: wl_seat::Event,
             _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        // The seat tells us what input devices are available.
        // When keyboard capability appears, grab a wl_keyboard object.
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if let WEnum::Value(caps) = capabilities {
                if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                    state.keyboard = Some(seat.get_keyboard(qh, ()));
                }
            }
        }
    }
}

impl Dispatch<WlKeyboard, ()> for AppState {
    fn event(state: &mut Self, _: &WlKeyboard, event: wl_keyboard::Event,
             _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_keyboard::Event::Key { key, state: key_state, .. } = event {
            // Only act on key press, not release
            if matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed)) {
                state.handle_key(key, qh);
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

impl Dispatch<WlCompositor, ()>    for AppState { fn event(_:&mut Self,_:&WlCompositor,   _:wl_compositor::Event, _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlShm, ()>           for AppState { fn event(_:&mut Self,_:&WlShm,           e:wl_shm::Event,        _:&(),_:&Connection,_:&QueueHandle<Self>){ let _ = e; } }
impl Dispatch<WlShmPool, ()>       for AppState { fn event(_:&mut Self,_:&WlShmPool,       _:wl_shm_pool::Event,   _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlBuffer, ()>        for AppState { fn event(_:&mut Self,_:&WlBuffer,        _:wl_buffer::Event,     _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<WlSurface, ()>       for AppState { fn event(_:&mut Self,_:&WlSurface,       _:wl_surface::Event,    _:&(),_:&Connection,_:&QueueHandle<Self>){} }
impl Dispatch<ZwlrLayerShellV1,()> for AppState { fn event(_:&mut Self,_:&ZwlrLayerShellV1,_:zwlr_layer_shell_v1::Event,_:&(),_:&Connection,_:&QueueHandle<Self>){} }
