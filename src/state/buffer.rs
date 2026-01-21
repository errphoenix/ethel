pub trait SwapBuffers<T> {
    fn swap_buffers(&mut self);

    /// Immutable view to the front buffer.
    ///
    /// This is the "ready" version of the buffer and what should be presented
    /// to the user.
    ///
    /// See [`SwapBuffers::front_mut`]
    fn front(&self) -> &T;

    /// Mutable view to the front buffer.
    ///
    /// This is the "ready" version of the buffer and what should be presented
    /// to the user.
    ///
    /// See [`SwapBuffers::front`]
    fn front_mut(&mut self) -> &mut T;

    /// Immutable view to the back buffer.
    ///
    /// This is the "WIP" version of the buffer and what should be used in
    /// order to make changes before [`swapping`](SwapBuffers::swap_buffers)
    /// is called.
    ///
    /// See [`SwapBuffers::back_mut`]
    fn back(&self) -> &T;

    /// Mutable view to the back buffer.
    ///
    /// This is the "WIP" version of the buffer and what should be used in
    /// order to make changes before [`swapping`](SwapBuffers::swap_buffers)
    /// is called.
    ///
    /// See [`SwapBuffers::back`]
    fn back_mut(&mut self) -> &mut T;
}

#[derive(Debug, Default)]
pub struct DoubleBuffer<T: Default> {
    current: T,
    next: T,
}

impl<T: Default> DoubleBuffer<Vec<T>> {
    /// Allocates both vectors to the same given `capacity`.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            current: Vec::with_capacity(capacity),
            next: Vec::with_capacity(capacity),
        }
    }
}

impl<T: Default> DoubleBuffer<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn next(&self) -> &T {
        &self.next
    }

    pub fn current_mut(&mut self) -> &mut T {
        &mut self.current
    }

    pub fn next_mut(&mut self) -> &mut T {
        &mut self.next
    }
}

impl<T: Default> SwapBuffers<T> for DoubleBuffer<T> {
    fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.current, &mut self.next);
    }

    fn front(&self) -> &T {
        &self.current
    }

    fn front_mut(&mut self) -> &mut T {
        &mut self.current
    }

    fn back(&self) -> &T {
        &self.next
    }

    fn back_mut(&mut self) -> &mut T {
        &mut self.next
    }
}

#[derive(Debug, Default)]
pub struct TripleBuffer<T: Default> {
    buffers: [T; 3],
    head: usize,
}

impl<T: Default> TripleBuffer<Vec<T>> {
    /// Allocates all 3 vectors to the same given `capacity`.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffers: [
                Vec::with_capacity(capacity),
                Vec::with_capacity(capacity),
                Vec::with_capacity(capacity),
            ],
            head: 0,
        }
    }
}

impl<T: Default> TripleBuffer<T> {
    const FRONT: usize = 0;
    const BACK: usize = 1;
    const THIRD: usize = 2;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn front(&self) -> &T {
        &self.buffers[Self::FRONT]
    }

    pub fn back(&self) -> &T {
        &self.buffers[Self::BACK]
    }

    pub fn third(&self) -> &T {
        &self.buffers[Self::THIRD]
    }

    pub fn front_mut(&mut self) -> &mut T {
        &mut self.buffers[Self::FRONT]
    }

    pub fn back_mut(&mut self) -> &mut T {
        &mut self.buffers[Self::BACK]
    }

    pub fn third_mut(&mut self) -> &mut T {
        &mut self.buffers[Self::THIRD]
    }

    fn next_buffer(&self) -> usize {
        (self.head + 1) % 3
    }

    pub fn rotate_buffers(&mut self) {
        self.head = self.next_buffer()
    }
}

impl<T: Default> SwapBuffers<T> for TripleBuffer<T> {
    fn swap_buffers(&mut self) {
        self.rotate_buffers();
    }

    fn front(&self) -> &T {
        &self.buffers[self.head]
    }

    fn front_mut(&mut self) -> &mut T {
        &mut self.buffers[self.head]
    }

    fn back(&self) -> &T {
        &self.buffers[self.next_buffer()]
    }

    fn back_mut(&mut self) -> &mut T {
        &mut self.buffers[self.next_buffer()]
    }
}
