use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Frame {
    name: &'static str,
    page: u64,
    value: u64,
    trace_handle: TraceId,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TraceId {
    index: u32,
    length: u32,
}

const TRACE_STACK_LENGTH: usize = 8;

#[derive(Debug, Default)]
pub struct Profiler {
    stack: Vec<Frame>,
    page: u64,

    trace_stack: [&'static str; TRACE_STACK_LENGTH],
    trace_index: usize,
    full_trace: String,

    frame_traces: String,
    frame_trace_current: TraceId,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            page: 0,
            trace_stack: [""; TRACE_STACK_LENGTH],
            trace_index: 0,
            full_trace: String::new(),
            frame_traces: String::new(),
            frame_trace_current: TraceId::default(),
        }
    }

    pub fn push_trace(&mut self, name: &'static str) {
        #[cfg(debug_assertions)]
        if self.trace_index == TRACE_STACK_LENGTH {
            panic!("cannot add trace group {name} to stack: limit of {TRACE_STACK_LENGTH} reached.")
        }

        let i = self.trace_index;
        self.trace_stack[i] = name;
        self.trace_index += 1;
        self.compose_trace();
    }

    pub fn pop_trace(&mut self) {
        #[cfg(debug_assertions)]
        if self.trace_index == 0 {
            panic!("cannot pop trace group to below index 0")
        }

        self.trace_index -= 1;
        self.compose_trace();
    }

    fn compose_trace(&mut self) {
        const TRACE_GROUND_SEPARATOR: &str = "::";

        self.full_trace.clear();
        for i in 0..self.trace_index {
            if i > 0 {
                self.full_trace += TRACE_GROUND_SEPARATOR;
            }
            self.full_trace += self.trace_stack[i];
        }

        self.frame_trace_current = TraceId {
            index: self.frame_traces.len() as u32,
            length: self.full_trace.len() as u32,
        };
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
            trace_handle: self.frame_trace_current,
        });

        func_return
    }

    pub fn page(&mut self) {
        self.page += 1;
    }

    pub fn current_page(&self) -> u64 {
        self.page
    }
}
