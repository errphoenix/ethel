use std::{
    os::raw::c_void,
    sync::atomic::{AtomicU16, AtomicUsize, Ordering},
};

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawArraysIndirectCommand {
    count: u32,
    instance_count: u32,
    first_vertex: u32,
    base_instance: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawElementsIndirectCommand {
    count: u32,
    instance_count: u32,
    first_vertex: u32,
    base_vertex: i32,
    base_instance: u32,
}

pub trait DrawCmd {
    fn call(draw_count: i32);
}

impl DrawCmd for DrawArraysIndirectCommand {
    fn call(draw_count: i32) {
        unsafe {
            janus::gl::MultiDrawArraysIndirect(
                janus::gl::TRIANGLES,
                std::ptr::null(),
                draw_count,
                0,
            );
        }
    }
}

impl DrawCmd for DrawElementsIndirectCommand {
    fn call(draw_count: i32) {
        unsafe {
            janus::gl::MultiDrawElementsIndirect(
                janus::gl::TRIANGLES,
                janus::gl::UNSIGNED_INT,
                std::ptr::null(),
                draw_count,
                0,
            );
        }
    }
}

#[derive(Debug, Default)]
pub struct GpuCommandQueue<C: DrawCmd + Clone + Copy> {
    queue: Vec<C>,
    upload_head: AtomicUsize,
    fixed_buffer_len: usize,
}

impl<C: DrawCmd + Clone + Copy> GpuCommandQueue<C> {
    pub fn new(buffer_len: usize) -> Self {
        Self {
            queue: Vec::with_capacity(buffer_len),
            upload_head: AtomicUsize::new(0),
            fixed_buffer_len: buffer_len,
        }
    }

    pub fn clear(&mut self) {
        self.upload_head.store(0, Ordering::Release);
        self.queue.clear();
    }

    pub fn pop(&mut self) -> Option<C> {
        self.queue.pop()
    }

    pub fn push(&mut self, command: C) {
        self.queue.push(command);
    }

    /// Perform an uploading operation onto a command `buffer`.
    ///
    /// One upload operation can only upload up to the buffer size initially
    /// set when creating the command queue, which corresponds to the size of
    /// the command buffer on the GPU.
    ///
    /// It may be required to perform this operation multiple times per frame
    /// if the total command count in the queue surpasses the buffer capacity.
    /// The command queue keeps track of the last uploaded command, so this
    /// can be done safely from the caller.
    ///
    /// Although, since a second upload operation will begin drawing at the
    /// beginning of the command buffer, dispatching the draw call in-between
    /// uploads is required or the commands will be lost.
    ///
    /// # Returns
    /// * `Ok` if the operation was successful and all commands were uploaded
    /// * `Err` with the amount of left-over commands to upload if not all
    ///   commands were uploaded.
    pub fn upload(&self, buffer: &mut [C]) -> Result<(), usize> {
        let count = self.queue.len();

        let head = self.upload_head.load(Ordering::Acquire);
        let remaining = count - head;
        let upload_size = remaining.min(self.fixed_buffer_len);

        let mut i = 0;
        for j in head..upload_size {
            buffer[i] = self.queue[j];
            i += 1;
        }
        self.upload_head.store(head + i, Ordering::Release);

        let exceed = count.saturating_sub(self.fixed_buffer_len);
        if exceed != 0 {
            return Err(exceed);
        }

        Ok(())
    }

    pub fn call(&self) {
        C::call(self.fixed_buffer_len as i32);
    }
}
