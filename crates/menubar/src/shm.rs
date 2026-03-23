// Shared memory buffer — how Wayland apps send pixels to the compositor.
//
// The idea: we create a file, memory-map it, draw pixels into it, then hand
// the compositor a reference to that memory. Both processes share the same
// physical RAM — no copying needed.

use std::fs::File;
use std::os::unix::io::AsFd;

use memmap2::MmapMut;
use wayland_client::{
    protocol::{wl_buffer::WlBuffer, wl_shm, wl_shm_pool::WlShmPool},
    Dispatch, QueueHandle,
};

pub struct ShmBuffer {
    pub buffer: WlBuffer,
    pub mmap: MmapMut,
    pub width: u32,
    pub height: u32,
    // Keep the file alive — the fd must remain valid while the pool exists.
    _file: File,
    // Keep the pool alive until we're done with the buffer.
    _pool: WlShmPool,
}

impl ShmBuffer {
    pub fn new<D>(shm: &wl_shm::WlShm, width: u32, height: u32, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, ()> + 'static,
    {
        let stride = width * 4; // ARGB8888 = 4 bytes per pixel
        let size = (stride * height) as usize;

        // tempfile creates an anonymous file (immediately unlinked from the filesystem).
        // It lives only as long as we hold the File handle.
        let file = tempfile::tempfile().expect("failed to create shm file");
        file.set_len(size as u64).expect("failed to resize shm file");

        // Map it into our address space so we can write pixels directly.
        let mmap = unsafe { MmapMut::map_mut(&file) }.expect("failed to mmap shm file");

        // Tell Wayland about this shared memory region.
        // The compositor will also map the same file to read our pixels.
        let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());

        // Allocate a buffer within the pool.
        // ARGB8888: each pixel is 4 bytes, layout in memory = [B, G, R, A] on little-endian.
        let buffer = pool.create_buffer(
            0,             // offset into pool (we only have one buffer, so 0)
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        ShmBuffer { buffer, mmap, width, height, _file: file, _pool: pool }
    }
}
