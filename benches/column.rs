use criterion::{Criterion, criterion_group, criterion_main};
use ethel::state::column::{Column, IndexArrayColumn, IterColumn, ParallelIndexArrayColumn};

criterion_group!(col_benches, col_iteration);
criterion_main!(col_benches);

fn col_iteration(cr: &mut Criterion) {
    const COUNT: usize = 100_000;

    cr.bench_function("index_column_iter_mapped", |b| {
        let col = {
            let mut col = IndexArrayColumn::with_capacity(COUNT);
            (1..=COUNT).for_each(|i| {
                col.put(Data::new(i));
            });
            col
        };

        b.iter(|| {
            let mut sum = 0i128;
            col.iter().map(|e| e.inner_value()).for_each(|e| {
                sum += op(e);
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("index_column_iter_entries", |b| {
        let col = {
            let mut col = IndexArrayColumn::with_capacity(COUNT);
            (1..=COUNT).for_each(|i| {
                col.put(Data::new(i));
            });
            col
        };

        b.iter(|| {
            let mut sum = 0i128;
            col.iter().for_each(|e| {
                sum += op(e.inner_value());
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("parallel_index_column_iter", |b| {
        let col = {
            let mut col = ParallelIndexArrayColumn::with_capacity(COUNT);
            (1..=COUNT).for_each(|i| {
                col.put(Data::new(i));
            });
            col
        };

        b.iter(|| {
            let mut sum = 0i128;
            col.iter().for_each(|e| {
                sum += op(e);
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("parallel_index_column_iter_zip_handles", |b| {
        let col = {
            let mut col = ParallelIndexArrayColumn::with_capacity(COUNT);
            (1..=COUNT).for_each(|i| {
                col.put(Data::new(i));
            });
            col
        };

        b.iter(|| {
            let mut sum = 0i128;
            col.iter().zip(col.handles()).for_each(|(e, handle)| {
                sum += op(e) * *handle as i128;
                std::hint::black_box(handle);
            });
            std::hint::black_box(sum)
        })
    });
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
struct Data {
    a: i128,
    b: i128,
}

impl Data {
    fn new(n: usize) -> Self {
        Self {
            a: n as i128,
            b: (n + n / 2) as i128,
        }
    }
}

#[inline(always)]
fn op(data: &Data) -> i128 {
    let d = data.a - data.b;
    d / (1 + (d as i64).abs() as i128)
}
