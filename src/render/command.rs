use std::os::raw::c_void;

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
pub struct GpuCommandQueue<C: DrawCmd> {
    queue: Vec<C>,
    gl_cap: isize,
    gl_obj: u32,
}

impl<C: DrawCmd> GpuCommandQueue<C> {
    pub fn new(capacity: usize) -> Self {
        let queue = Vec::with_capacity(capacity);
        let command_buffer = {
            let mut buf = 0;
            let length = (size_of::<C>() * capacity) as isize;

            unsafe {
                janus::gl::CreateBuffers(1, &mut buf);
                janus::gl::NamedBufferData(buf, length, std::ptr::null(), janus::gl::STREAM_DRAW);
            }
            buf
        };

        Self {
            queue,
            gl_cap: capacity as isize,
            gl_obj: command_buffer,
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

    pub fn upload(&mut self) {
        let len = self.queue.len() as isize;
        if len > self.gl_cap {
            unsafe {
                janus::gl::NamedBufferData(
                    self.gl_obj,
                    len,
                    std::ptr::null(),
                    janus::gl::STREAM_DRAW,
                );
            }
        }

        unsafe {
            janus::gl::NamedBufferSubData(
                self.gl_obj,
                0,
                len,
                self.queue.as_ptr() as *const c_void,
            );
        }
    }

    pub fn call(&self) {
        C::call(self.queue.len() as i32);
    }
}

impl<C: DrawCmd> Drop for GpuCommandQueue<C> {
    fn drop(&mut self) {
        if self.gl_obj == 0 {
            return;
        }

        unsafe {
            janus::gl::DeleteBuffers(1, &self.gl_obj);
        }
    }
}
