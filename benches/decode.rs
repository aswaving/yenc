#[macro_use]
extern crate criterion;

extern crate yenc;
use criterion::*;

fn decode(c: &mut Criterion) {
    let mut buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    let length = buf.len();
    let mut encoded = Vec::with_capacity(32_768 * 102 / 100);
    yenc::encode_buffer(&buf, 0, 128, &mut encoded).unwrap();

    c.bench(
        "decode",
        Benchmark::new("decode 32k", move |b| {
            buf.clear();
            b.iter(|| yenc::decode_buffer(&encoded).unwrap())
        })
        .throughput(Throughput::Bytes(length as u32)),
    );
}

criterion_group!(benches, decode);
criterion_main!(benches);
