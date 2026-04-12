use std::{
    time::{Duration, Instant},
    u32,
};

use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProfileMetrics<'a> {
    pub sys_info: SystemInformation,
    #[serde(borrow)]
    pub stackframes: Vec<StackFrame<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Deserialize, Serialize)]
pub struct StackFrame<'a> {
    pub name: &'a str,
    pub trace: &'a str,
    pub page: u64,
    pub timestamp: u64,
    /// elapsed time local to page in nanoseconds
    pub start: u64,
    /// elapsed time local to page in nanoseconds
    pub end: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Deserialize, Serialize)]
pub struct Frame {
    name: &'static str,
    page: u64,
    timestamp: u64,
    // elapsed time local to page in nanoseconds
    start: u64,
    // elapsed time local to page in nanoseconds
    end: u64,
    trace_handle: TraceId,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TraceId {
    index: u32,
    length: u32,
}

const TRACE_STACK_LENGTH: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInformation {
    name: String,
    os_version: String,
    kernel_v: String,

    cpu_count: u32,
    cpu_arch: String,
    cpu_phys_count: u32,

    cpu_brand: String,
    cpu_speed: u64,

    total_memory: u64,

    uptime_at_init: u64,
}

impl std::fmt::Display for SystemInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            r#"System Information:
Name: {}
Version: {}
Kernel version: {}
Uptime (at init.): {}
Architecture: {}
CPU brand: {}
CPU cores (logical, or threads): {}
CPU cores (physical): {}
CPU speed: {} MHz
Memory: {} bytes"#,
            self.name,
            self.os_version,
            self.kernel_v,
            self.uptime_at_init,
            self.cpu_arch,
            self.cpu_brand,
            self.cpu_count,
            self.cpu_phys_count,
            self.cpu_speed,
            self.total_memory,
        )
    }
}

impl SystemInformation {
    const FALLBACK: &str = "N/A";

    pub fn new() -> Self {
        let sys = System::new_all();
        Self {
            name: System::name().unwrap_or_else(|| Self::FALLBACK.to_owned()),
            os_version: System::os_version().unwrap_or_else(|| Self::FALLBACK.to_owned()),
            kernel_v: System::kernel_version().unwrap_or_else(|| Self::FALLBACK.to_owned()),

            cpu_count: sys.cpus().len() as u32,
            cpu_arch: System::cpu_arch(),
            cpu_phys_count: System::physical_core_count().unwrap_or_default() as u32,

            cpu_brand: sys.cpus()[0].brand().to_owned(),
            cpu_speed: sys.cpus()[0].frequency(),

            total_memory: sys.total_memory(),

            uptime_at_init: System::uptime(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Profiler {
    sys_info: SystemInformation,

    stack: Vec<Frame>,

    init_time: Instant,

    page: u64,
    last_dump_page: u64,
    page_time: Instant,

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
            sys_info: SystemInformation::new(),
            stack: Vec::new(),
            init_time: Instant::now(),
            page: 1,
            last_dump_page: 0,
            page_time: Instant::now(),
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
        let time_offset = self.page_time;

        let t0 = Instant::now();
        let func_return = func();
        let t1 = Instant::now();

        self.stack.push(Frame {
            name,
            page,
            timestamp: (t0 - self.init_time).as_micros() as u64,
            start: (t0 - time_offset).as_nanos() as u64,
            end: (t1 - time_offset).as_nanos() as u64,
            trace_handle: self.frame_trace_current,
        });

        func_return
    }

    pub fn page(&mut self) {
        self.page += 1;
        self.page_time = Instant::now();
    }

    pub fn current_page(&self) -> u64 {
        self.page
    }

    pub fn frames_stack(&self) -> &[Frame] {
        &self.stack
    }

    fn build_stackframe(&self, frame: &Frame) -> StackFrame<'_> {
        let TraceId { index, length } = frame.trace_handle;
        let start = index as usize;
        let end = (index + length) as usize;
        let trace = &self.frame_traces[start..end];

        StackFrame {
            name: frame.name,
            trace,
            timestamp: frame.timestamp,
            page: frame.page,
            start: frame.start,
            end: frame.end,
        }
    }

    pub fn present_encoded<W: std::io::Write>(&mut self, out: &mut W) -> std::io::Result<()> {
        let stackframes = self
            .stack
            .iter()
            .map(|frame| self.build_stackframe(&frame))
            .collect::<Vec<_>>();

        let metrics = ProfileMetrics {
            sys_info: self.sys_info.clone(),
            stackframes,
        };

        let bytes = postcard::to_allocvec(&metrics)
            .expect("failed to encode profiler metrics to dynamic buffer");

        self.stack.clear();
        self.frame_traces.clear();
        self.init_time = Instant::now();

        out.write_all(&bytes)?;
        out.flush()?;
        Ok(())
    }

    pub fn present_plain<W: std::io::Write>(&mut self, out: &mut W) -> std::io::Result<()> {
        let page_count = self.page - self.last_dump_page;
        let frame_count = self.stack.len();

        writeln!(out, "Ethel Profiler Dump")?;
        writeln!(out, "- Total Pages: {page_count}")?;
        writeln!(out, "- Total Frames: {frame_count}")?;
        writeln!(out, "{}", self.sys_info)?;
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

            let d = Duration::from_nanos(frame.end - frame.start);
            writeln!(out, " = {} microseconds", d.as_micros())?;

            frame_index_abs += 1;
            frame_index_page += 1;

            Ok::<(), std::io::Error>(())
        })?;
        self.frame_traces.clear();
        self.init_time = Instant::now();
        out.flush()?;
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

        profiler.capture_duration("initialise", || thread::sleep(Duration::from_micros(50)));

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

        profiler.capture_duration("finalize", || thread::sleep(Duration::from_micros(50)));
        profiler.present_plain(&mut std::io::stdout()).unwrap();
    }
}
