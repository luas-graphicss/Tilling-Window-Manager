use std::os::fd::AsFd;

use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::{Format, WlShm};

use crate::error::ClientError;
use crate::state::ClientState;

const BYTES_PER_PIXEL: u32 = 4;
const MEMORY_NAME: &str = "tilling-test-buffer";

pub(crate) fn create(
    shm: &WlShm,
    queue: &QueueHandle<ClientState>,
    width: u32,
    height: u32,
) -> Result<WlBuffer, ClientError> {
    let too_large = || ClientError::BufferTooLarge { width, height };
    let stride = width.checked_mul(BYTES_PER_PIXEL).ok_or_else(too_large)?;
    let bytes = stride.checked_mul(height).ok_or_else(too_large)?;
    let pool_size = i32::try_from(bytes).map_err(|_| too_large())?;
    let memory = memfd_create(MEMORY_NAME, MemfdFlags::CLOEXEC).map_err(ClientError::Buffer)?;
    ftruncate(&memory, u64::from(bytes)).map_err(ClientError::Buffer)?;
    let pool = shm.create_pool(memory.as_fd(), pool_size, queue, ());
    let buffer = pool.create_buffer(
        0,
        i32::try_from(width).map_err(|_| too_large())?,
        i32::try_from(height).map_err(|_| too_large())?,
        i32::try_from(stride).map_err(|_| too_large())?,
        Format::Argb8888,
        queue,
        (),
    );
    pool.destroy();
    Ok(buffer)
}
