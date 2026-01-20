use criterion::{Criterion, criterion_group, criterion_main};
use ethel::state::column::Column;

criterion_group!(col_benches, col_iteration);
criterion_main!(col_benches);

fn col_iteration(cr: &mut Criterion) {
    const COUNT: usize = 100_000;

    let col = {
        let mut col = Column::with_capacity(COUNT);
        (1..=COUNT).for_each(|i| {
            col.put(Data::new(i));
        });
        col
    };

    cr.bench_function("iter_mapped", |b| {
        b.iter(|| {
            let mut sum = 0i128;
            col.iter().for_each(|e| {
                sum += op(e);
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("iter_nomap", |b| {
        b.iter(|| {
            let mut sum = 0i128;
            col.direct().iter().for_each(|e| {
                sum += op(e.inner_value());
            });
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("loop_mapped", |b| {
        b.iter(|| {
            let mut sum = 0i128;
            for e in col.iter() {
                sum += op(e);
            }
            std::hint::black_box(sum)
        })
    });

    cr.bench_function("loop_nomap", |b| {
        b.iter(|| {
            let mut sum = 0i128;
            for e in col.direct() {
                sum += op(e.inner_value());
            }
            std::hint::black_box(sum)
        })
    });
}

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
