use std::{
    ops::{Add, AddAssign},
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
    pub fn average(&self) -> T {
        let total_samples = self
            .buffer
            .iter()
            .fold(0, |accum, bucket| accum + bucket.sample_count);
        self.accumulated().average(total_samples)
    }
}

pub trait AccumValue: Default + Clone + Copy + Add<Self, Output = Self> + AddAssign<Self> {}
impl<T> AccumValue for T where T: Default + Clone + Copy + Add<Self, Output = Self> + AddAssign<Self>
{}

pub trait AverageValue: AccumValue {
    fn average(self, sample_count: u32) -> Self;
}
impl AverageValue for u32 {
    fn average(self, sample_count: u32) -> Self {
        self / sample_count
    }
}
impl AverageValue for i32 {
    fn average(self, sample_count: u32) -> Self {
        self / sample_count as i32
    }
}
impl AverageValue for u64 {
    fn average(self, sample_count: u32) -> Self {
        self / sample_count as u64
    }
}
impl AverageValue for i64 {
    fn average(self, sample_count: u32) -> Self {
        self / sample_count as i64
    }
}
impl AverageValue for f32 {
    fn average(self, sample_count: u32) -> Self {
        self / sample_count as f32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccumulationBucket<T: AccumValue> {
    target_size: Duration,
    start: Instant,
    last_update: Instant,
    accumulated: T,
    sample_count: u32,
}
impl<T: AccumValue> AccumulationBucket<T> {
    pub fn new(target_size: Duration) -> Self {
        Self {
            target_size,
            start: Instant::now(),
            last_update: Instant::now(),
            accumulated: T::default(),
            sample_count: 0,
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
        self.sample_count += 1;
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
impl<T: AverageValue> AccumulationBucket<T> {
    pub fn average(&self) -> T {
        self.value().average(self.sample_count)
    }
}
