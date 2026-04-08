use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Frame {
    name: &'static str,
    page: u32,
    value: u64,
}

#[derive(Debug, Default)]
pub struct Profiler {
    stack: Vec<Frame>,
    page: u32,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            page: 0,
        }
    }

    #[inline]
    pub fn capture_duration<R, F: FnMut() -> R>(&mut self, name: &'static str, mut func: F) -> R {
        let page = self.page;

        let t0 = Instant::now();
        let func_return = func();
        let t1 = Instant::now();

        let d = (t1 - t0).as_nanos() as u64;
        self.stack.push(Frame {
            name,
            page,
            value: d,
        });

        func_return
    }

    pub fn page(&mut self) {
        self.page += 1;
    }

    pub fn current_page(&self) -> u32 {
        self.page
    }
}
