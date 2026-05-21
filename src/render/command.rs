use std::sync::atomic::{AtomicU32, Ordering};

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

pub trait DrawCmd: std::fmt::Debug + Clone + Copy {
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

/// Trait to identify draw command groups for [`instructions`](Instruction),
/// used for [`GpuCommandQueue`].
///
/// It is recommended to properly document the correct order and usage of the
/// custom [`DrawGroups`] definition.
pub trait DrawGroups: Clone + Copy + PartialEq + Eq + std::fmt::Debug + std::fmt::Display {
    fn as_str(&self) -> &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Instruction<C: DrawCmd, G: DrawGroups> {
    Draw(C),
    Switch(G),
}

impl<C: DrawCmd, G: DrawGroups> std::fmt::Display for Instruction<C, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Draw(_) => write!(f, "draw: {}", stringify!(C)),
            Instruction::Switch(g) => write!(f, "switch to group: {g}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct GpuCommandQueue<C: DrawCmd, G: DrawGroups> {
    queue: Vec<Instruction<C, G>>,
    head: AtomicU32,
    first_group: Option<G>,
}

impl<C: DrawCmd, G: DrawGroups> GpuCommandQueue<C, G> {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            head: AtomicU32::new(0),
            first_group: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: Vec::with_capacity(capacity),
            head: AtomicU32::new(0),
            first_group: None,
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.head.store(0, Ordering::Release);
        self.first_group = None;
    }

    pub fn pop(&mut self) -> Option<Instruction<C, G>> {
        self.queue.pop()
    }

    /// Returns the first group that was uploaded to the instruction queue
    /// since the last [`GpuCommandQueue::clear`] call.
    ///
    /// This only tracks the first group, any subsequent group must be
    /// retrieved from [`GpuCommandQueue::upload_next_group`].
    pub fn first_group(&self) -> Option<G> {
        self.first_group
    }

    /// Push a new draw command.
    ///
    /// This creates a new [`Instruction::Draw`] entry in the instruction
    /// queue.
    ///
    /// Note that it is recommended to keep commands of the same group
    /// contiguous in the queue, to minimize both the amount of gpu draw
    /// dispatches and the possibility of a programmer error.
    pub fn push_command(&mut self, command: C) {
        self.queue.push(Instruction::Draw(command));
    }

    /// Push a new draw group.
    ///
    /// This creates a new [`Instruction::Switch`] entry in the instruction
    /// queue.
    ///
    /// Note that it is recommended to keep commands of the same group
    /// contiguous in the queue, to minimize both the amount of gpu draw
    /// dispatches and the possibility of programmer error.
    pub fn push_group(&mut self, group: G) {
        if self.first_group.is_none() {
            self.first_group = Some(group);
        } else {
            self.queue.push(Instruction::Switch(group));
        }
    }

    /// Total length of instructions across all groups (including
    /// switch-group instructions)
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn index(&self) -> u32 {
        self.head.load(Ordering::Relaxed)
    }

    fn get_head(&self) -> Option<Instruction<C, G>> {
        let head = self.head.load(Ordering::Acquire);
        let instr = self.queue.get(head as usize);
        if instr.is_some() {
            self.head.fetch_add(1, Ordering::Release);
        }
        instr.copied()
    }

    /// Upload the next contiguous group of draw instructions.
    ///
    /// The programmer must be aware of the current `DrawGroup` that is
    /// going to be uploaded and the order of [`DrawGroups`] used for this
    /// queue.
    ///
    /// The order of the [`DrawGroups`] present in queue is always the same
    /// order as when they are pushed to the queue.
    ///
    /// It is recommended to keep the draw commands of each group contiguous
    /// in the queue to minimize dispatch calls and the possibility of
    /// programmer error.
    ///
    /// This will upload all [`Instruction::Draw`] entries until the queue is
    /// empty or an [`Instruction::Switch] entry is encountered.
    ///
    /// # Returns
    /// `Some` with the group up next if there is one.
    pub fn upload_next_group(&self, buffer: &mut [C]) -> Option<G> {
        let dst = buffer.as_ptr() as *mut C;
        let mut dst_offset = 0;

        while let Some(instruction) = self.get_head() {
            match instruction {
                Instruction::Draw(cmd) => unsafe {
                    std::ptr::copy_nonoverlapping(&cmd, dst.add(dst_offset), 1);
                    dst_offset += 1;
                    continue;
                },
                Instruction::Switch(g) => return Some(g),
            }
        }

        None
    }
}

#[derive(Clone, Copy, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Groups {
        A,
        B,
        C,
    }

    impl std::fmt::Display for Groups {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.as_str())
        }
    }

    impl DrawGroups for Groups {
        fn as_str(&self) -> &'static str {
            match self {
                Groups::A => "a",
                Groups::B => "b",
                Groups::C => "c",
            }
        }
    }

    #[test]
    fn gpu_cmd_queue_groups() {
        let mut queue = GpuCommandQueue::new();
        assert_eq!(queue.first_group(), None);

        queue.push_group(Groups::A);
        assert_eq!(queue.first_group(), Some(Groups::A));

        for _ in 0..50 {
            queue.push_command(DrawArraysIndirectCommand::default());
        }

        queue.push_group(Groups::B);
        assert_eq!(queue.first_group(), Some(Groups::A));

        for _ in 0..20 {
            queue.push_command(DrawArraysIndirectCommand::default());
        }

        queue.push_group(Groups::C);

        assert_eq!(queue.len(), 50 + 20 + 2);

        {
            let mut buf = vec![DrawArraysIndirectCommand::default(); 50];
            let next = queue.upload_next_group(&mut buf);
            assert_eq!(next, Some(Groups::B));
        }
        {
            let mut buf = vec![DrawArraysIndirectCommand::default(); 20];
            let next = queue.upload_next_group(&mut buf);
            assert_eq!(next, Some(Groups::C));
        }
        {
            let mut buf = vec![DrawArraysIndirectCommand::default(); 1];
            let next = queue.upload_next_group(&mut buf);
            assert_eq!(next, None);
        }
    }
}
