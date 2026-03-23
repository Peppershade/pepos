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
    _file: File,
    _pool: WlShmPool,
}

impl ShmBuffer {
    pub fn new<D>(shm: &wl_shm::WlShm, width: u32, height: u32, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, ()> + 'static,
    {
        let stride = width * 4;
        let size = (stride * height) as usize;
        let file = tempfile::tempfile().expect("shm file");
        file.set_len(size as u64).expect("shm resize");
        let mmap = unsafe { MmapMut::map_mut(&file) }.expect("mmap");
        let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
        let buffer = pool.create_buffer(0, width as i32, height as i32, stride as i32, wl_shm::Format::Argb8888, qh, ());
        ShmBuffer { buffer, mmap, width, height, _file: file, _pool: pool }
    }
}
