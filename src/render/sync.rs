use std::sync::atomic::{AtomicU8, Ordering};

use janus::gl::types::__GLsync;

use crate::render::buffer::StorageSection;

#[derive(Default, Debug, Clone)]
pub struct SyncBarrier {
    fences: [Option<*const __GLsync>; 3],
}

#[derive(Default, Debug)]
pub struct SyncState {
    locks: AtomicU8,
}

impl SyncBarrier {
    pub fn new() -> Self {
        Self {
            fences: [Option::None; 3],
        }
    }

    pub fn fetch(&mut self, to: &SyncState) {
        let mut bits = 0u8;
        for i in 0..3 {
            if let Some(fence) = self.fences[i].take() {
                let fence_query = unsafe { janus::gl::ClientWaitSync(fence, 0, 1) };
                if fence_query == janus::gl::CONDITION_SATISFIED
                    || fence_query == janus::gl::ALREADY_SIGNALED
                {
                    unsafe {
                        janus::gl::DeleteSync(fence);
                    }
                } else {
                    match i {
                        0 => bits |= StorageSection::Front as u8,
                        1 => bits |= StorageSection::Back as u8,
                        2 => bits |= StorageSection::Spare as u8,
                        _ => unreachable!(),
                    }
                    self.fences[i] = Some(fence);
                }
            }
        }
        to.set(bits);
    }

    pub fn set(&mut self, index: usize, fence: *const __GLsync) {
        self.fences[index] = Some(fence);
    }
}

impl Drop for SyncBarrier {
    fn drop(&mut self) {
        self.fences
            .into_iter()
            .filter_map(|maybe_fence| maybe_fence)
            .for_each(|fence| unsafe {
                janus::gl::DeleteSync(fence);
            });
    }
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            locks: AtomicU8::new(0),
        }
    }

    /// Performs an `OR` operation on the internal lock bit.
    fn lock_bits(&self, section: u8) {
        self.locks.fetch_or(section, Ordering::Release);
    }

    /// Performs an `AND` operation on the internal lock bit with the inverted
    /// `section` bits.
    fn unlock_bits(&self, section: u8) {
        self.locks.fetch_and(!section, Ordering::Release);
    }

    /// Performs an `OR` operation on the internal lock bit.
    fn lock(&self, section: StorageSection) {
        self.lock_bits(section as u8);
    }

    /// Performs an `AND` operation on the internal lock bit with the inverted
    /// `section` bit.
    fn unlock(&self, section: StorageSection) {
        self.unlock_bits(section as u8);
    }

    fn set(&self, bits: u8) {
        self.locks.store(bits, Ordering::Release);
    }

    pub fn has_lock(&self, section: StorageSection) -> bool {
        let bit = section as u8;
        self.locks.load(Ordering::Acquire) & bit == bit
    }
}
