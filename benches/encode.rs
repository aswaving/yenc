use criterion::*;
use yenc;

fn encode_buffer(c: &mut Criterion) {
    let buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    let length = buf.len();
    let mut output = Vec::with_capacity(32_768 * 102 / 100);
    c.bench(
        "encode",
        Benchmark::new("encode 32k", move |b| {
            b.iter(|| {
                output.clear();
                yenc::encode_buffer(&buf, 0, 128, &mut output).unwrap()
            })
        })
        .throughput(Throughput::Bytes(length as u64)),
    );
}

criterion_group!(benches, encode_buffer);
criterion_main!(benches);
