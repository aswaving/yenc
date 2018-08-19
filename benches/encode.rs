#[macro_use]
extern crate criterion;
extern crate yenc;

use criterion::Criterion;

fn encode_buffer(c: &mut Criterion) {
    let buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    c.bench_function("encode 32k", move |b| {
        b.iter(|| {
            let mut col = 0;
            let mut output = Vec::with_capacity(32_768 * 102 / 100);
            yenc::encode_buffer(&buf, &mut col, 128, &mut output)
        })
    });
}

criterion_group!(benches, encode_buffer);
criterion_main!(benches);
