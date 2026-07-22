use std::{
    ops::{Add, AddAssign, Div, DivAssign},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct AccumulationWindow<const LENGTH: usize, T: AccumValue> {
    bucket_size: Duration,
    buffer: [AccumulationBucket<T>; LENGTH],
    index: usize,
}
impl<const LENGTH: usize, T: AccumValue> AccumulationWindow<LENGTH, T> {
    pub fn new(bucket_size: Duration) -> Self {
        Self {
            bucket_size,
            buffer: std::array::from_fn(|_| AccumulationBucket::new(bucket_size)),
            index: 0,
        }
    }

    pub fn register(&mut self, value: T, time: Instant) {
        let current = &mut self.buffer[self.index];
        if !current.is_past() {
            current.accumulate(value, time);
        } else {
            self.index = (self.index + 1) % LENGTH;
            self.buffer[self.index] = AccumulationBucket::new(self.bucket_size);
        }
    }

    pub fn accumulated(&self) -> T {
        let mut accumulated = T::default();
        for i in 0..LENGTH {
            accumulated += self.buffer[i].value();
        }
        accumulated
    }

    pub fn bucket_size(&self) -> Duration {
        self.bucket_size
    }

    pub fn total_duration(&self) -> Duration {
        self.bucket_size.mul_f32(LENGTH as f32)
    }
}
impl<const LENGTH: usize, T: AverageValue> AccumulationWindow<LENGTH, T> {
    /// Get the average value within the [`total duration`](AccumulationWindow::total_duration)
    /// of the accumulation window.
    pub fn average_per_sec(&self) -> T {
        let duration = self.total_duration().as_secs_f32();
        self.accumulated() / duration
    }

    /// Get the average value within the [`total duration`](AccumulationWindow::total_duration)
    /// of the accumulation window.
    pub fn average_per_millis(&self) -> T {
        let duration = self.total_duration().as_millis() as f32;
        self.accumulated() / duration
    }
}

pub trait AccumValue: Default + Clone + Copy + Add<Self> + AddAssign<Self> {}
impl<T> AccumValue for T where T: Default + Clone + Copy + Add<T> + AddAssign<T> {}

pub trait AverageValue: AccumValue + Div<f32, Output = Self> + DivAssign<f32> {}
impl<T> AverageValue for T where T: AccumValue + Div<f32, Output = T> + DivAssign<f32> {}

#[derive(Debug, Clone, Copy)]
pub struct AccumulationBucket<T: AccumValue> {
    target_size: Duration,
    start: Instant,
    last_update: Instant,
    accumulated: T,
}

impl<T: AccumValue> AccumulationBucket<T> {
    pub fn new(target_size: Duration) -> Self {
        Self {
            target_size,
            start: Instant::now(),
            last_update: Instant::now(),
            accumulated: T::default(),
        }
    }

    pub fn is_past(&self) -> bool {
        let d = self.last_update - self.start;
        d >= self.target_size
    }

    pub fn accumulate(&mut self, value: T, time: Instant) -> T {
        let d = self.last_update - self.start;
        if d < self.target_size {
            self.accumulated += value;
        }
        self.last_update = time;
        self.accumulated
    }

    pub fn value(&self) -> T {
        self.accumulated
    }

    /// The duration of the slice of time represented.
    pub fn size(&self) -> Duration {
        self.last_update - self.start
    }

    /// The target (or max) duration of the slice of time representable.
    pub fn target_size(&self) -> Duration {
        self.target_size
    }
}
