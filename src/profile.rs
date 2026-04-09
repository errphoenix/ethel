use std::{time::Instant, u32};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, Deserialize, Serialize)]
pub struct Frame {
    name: &'static str,
    page: u64,
    value: u64,
    trace_handle: TraceId,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TraceId {
    index: u32,
    length: u32,
}

const TRACE_STACK_LENGTH: usize = 8;

#[derive(Debug)]
pub struct Profiler {
    stack: Vec<Frame>,
    page: u64,
    last_dump_page: u64,

    trace_stack: [&'static str; TRACE_STACK_LENGTH],
    trace_index: usize,
    full_trace: String,

    frame_traces: String,
    frame_trace_current: TraceId,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            page: 1,
            last_dump_page: 0,
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

        self.frame_traces.push_str(&self.full_trace);
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

    pub fn frames_stack(&self) -> &[Frame] {
        &self.stack
    }

    pub fn write_bytes<'b>(&mut self, out: &'b mut [u8]) -> postcard::Result<&'b mut [u8]> {
        let bytes = postcard::to_slice(&self.stack, out);
        self.stack.clear();
        self.frame_traces.clear();
        bytes
    }

    pub fn write_plain<W: std::io::Write>(&mut self, out: &mut W) -> std::io::Result<()> {
        let page_count = self.page - self.last_dump_page;
        let frame_count = self.stack.len();

        writeln!(out, "Ethel Profiler Dump")?;
        writeln!(out, "- Total Pages: {page_count}")?;
        writeln!(out, "- Total Frames: {frame_count}")?;
        writeln!(out)?;

        let mut last_page = 0;
        let mut frame_index_abs = 0;
        let mut frame_index_page = 0;

        self.stack.drain(..).try_for_each(|frame| {
            let page = frame.page;
            if last_page != page {
                writeln!(out, "+ New Page: {page}")?;
                last_page = page;
                frame_index_page = 0;
            }

            let TraceId { index, length } = frame.trace_handle;
            let start = index as usize;
            let end = (index + length) as usize;
            let trace = &self.frame_traces[start..end];

            write!(out, "[{frame_index_abs};{frame_index_page}] ")?;
            write!(out, "{trace}#{}", frame.name)?;
            writeln!(out, " = {}", frame.value)?;

            frame_index_abs += 1;
            frame_index_page += 1;

            Ok::<(), std::io::Error>(())
        })?;
        out.flush()?;
        self.frame_traces.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    #[test]
    fn profiler_dump() {
        let mut profiler = Profiler::new();

        profiler.capture_duration("initialise", || thread::sleep(Duration::from_millis(50)));

        for _ in 0..100 {
            profiler.capture_duration("page_init", || thread::sleep(Duration::from_micros(25)));

            profiler.push_trace("group_1");
            profiler.capture_duration("fn_1_a", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_1_b", || thread::sleep(Duration::from_micros(10)));

            profiler.push_trace("group_1_sub_a");
            profiler.capture_duration("fn_1a_a", || thread::sleep(Duration::from_micros(15)));
            profiler.capture_duration("fn_1a_b", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_1a_c", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_1a_d", || thread::sleep(Duration::from_micros(10)));
            profiler.pop_trace();

            profiler.push_trace("group_1_sub_b");
            profiler.capture_duration("fn_1b_a", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_1b_b", || thread::sleep(Duration::from_micros(20)));
            profiler.capture_duration("fn_1a_c", || thread::sleep(Duration::from_micros(10)));
            profiler.pop_trace();
            profiler.pop_trace();

            profiler.push_trace("group_2");
            profiler.capture_duration("fn_2_a", || thread::sleep(Duration::from_micros(50)));
            profiler.capture_duration("fn_2_b", || thread::sleep(Duration::from_micros(20)));
            profiler.capture_duration("fn_2_c", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_2_d", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_2_e", || thread::sleep(Duration::from_micros(10)));
            profiler.capture_duration("fn_2_f", || thread::sleep(Duration::from_micros(10)));
            profiler.pop_trace();

            profiler.push_trace("group_3");
            profiler.capture_duration("fn_3_a", || thread::sleep(Duration::from_micros(25)));
            profiler.capture_duration("fn_3_b", || thread::sleep(Duration::from_micros(32)));
            profiler.capture_duration("fn_3_c", || thread::sleep(Duration::from_micros(32)));
            profiler.capture_duration("fn_3_d", || thread::sleep(Duration::from_micros(32)));
            profiler.capture_duration("fn_3_e", || thread::sleep(Duration::from_micros(32)));

            profiler.push_trace("group_1_sub_a");
            profiler.capture_duration("fn_3a_a", || thread::sleep(Duration::from_micros(50)));
            profiler.capture_duration("fn_3a_b", || thread::sleep(Duration::from_micros(60)));
            profiler.capture_duration("fn_3a_c", || thread::sleep(Duration::from_micros(32)));
            profiler.pop_trace();
            profiler.pop_trace();

            profiler.page();
        }

        profiler.capture_duration("finalize", || thread::sleep(Duration::from_millis(50)));

        profiler.write_plain(&mut std::io::stdout()).unwrap();
    }
}
