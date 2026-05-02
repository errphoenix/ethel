use crate::render::buffer::View;

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawArraysIndirectCommand {
    pub count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub base_instance: u32,
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

pub trait DrawCmd: std::fmt::Debug {
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
}

impl<C: DrawCmd + Clone + Copy> GpuCommandQueue<C> {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn pop(&mut self) -> Option<C> {
        self.queue.pop()
    }

    pub fn push(&mut self, command: C) {
        self.queue.push(command);
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn upload(&self, buffer: &mut [C]) -> Result<(), usize> {
        let count = self.queue.len();
        let cap = buffer.len();

        let safe_size = count.min(cap);
        let exceed = count.saturating_sub(cap);

        unsafe {
            let dst = buffer.as_ptr() as *mut C;
            let src = self.queue.as_ptr();
            std::ptr::copy_nonoverlapping(src, dst, safe_size);
        }

        if exceed != 0 {
            return Err(exceed);
        }

        Ok(())
    }
}

pub struct GpuCommandDispatch<'buf, C: DrawCmd + Clone + Copy> {
    command_buffer: View<'buf, C>,
}

impl<'buf, C: DrawCmd + Clone + Copy> GpuCommandDispatch<'buf, C> {
    pub const fn from_view(view: View<'buf, C>) -> Self {
        Self {
            command_buffer: view,
        }
    }

    pub fn dispatch(&self) {
        // todo: pass count, somehow; maybe read from shared buffer
        // would require making the command tri buffer a partitioned tri buffer

        let len = self.command_buffer.len() as i32;
        let gl_obj = self.command_buffer.source();

        unsafe {
            janus::gl::BindBuffer(janus::gl::DRAW_INDIRECT_BUFFER, gl_obj);
        }
        C::call(len);
    }
}
