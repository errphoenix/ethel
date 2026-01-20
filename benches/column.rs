use criterion::{Criterion, criterion_group, criterion_main};
use ethel::state::column::Column;

criterion_group!(col_benches, col_iteration);
criterion_main!(col_benches);

fn col_iteration(cr: &mut Criterion) {
    const COUNT: usize = 500_000;

    let col = {
        let mut col = Column::with_capacity(COUNT);
        (0..COUNT).for_each(|i| {
            col.put(Data::new(i));
        });
        col
    };

    cr.bench_function("iter_mapped", |b| {
        b.iter(|| {
            let mut sum = 0u128;
            col.iter().for_each(|e| {
                sum += e.a;
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("iter_nomap", |b| {
        b.iter(|| {
            let mut sum = 0u128;
            col.direct().iter().for_each(|e| {
                sum += e.inner_value().a;
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("loop_mapped", |b| {
        b.iter(|| {
            let mut sum = 0u128;
            for e in col.iter() {
                sum += e.a;
            }
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("loop_nomap", |b| {
        b.iter(|| {
            let mut sum = 0u128;
            for e in col.direct() {
                sum += e.inner_value().a;
            }
            std::hint::black_box(sum)
        })
    });
}

#[derive(Clone, Debug, Default)]
struct Data {
    a: u128,
    _b: u128,
}

impl Data {
    fn new(n: usize) -> Self {
        Self {
            a: n as u128,
            _b: (n + n / 2) as u128,
        }
    }
}
