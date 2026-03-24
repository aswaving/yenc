use criterion::*;
use std::io::Cursor;

fn encode_buffer(c: &mut Criterion) {
    let buf = (0..32_768_u32)
        .map(|i| (i % 256) as u8)
        .collect::<Vec<u8>>();
    let length = buf.len();
    let mut output = Vec::with_capacity(length * 102 / 100);

    let mut group = c.benchmark_group("encode");
    group
        .throughput(Throughput::Bytes(length as u64))
        .bench_function("encode 32k", |b| {
            b.iter(|| {
                output.clear();
                yenc::encode_buffer(&buf, 0, 128, &mut output).unwrap()
            })
        });
    group.finish();
}

fn encode_stream(c: &mut Criterion) {
    let buf = (0..32_768_u32)
        .map(|i| (i % 256) as u8)
        .collect::<Vec<u8>>();
    let length = buf.len();
    let output_capacity = length * 110 / 100;
    let options = yenc::EncodeOptions::new();

    let mut group = c.benchmark_group("encode_stream");
    group
        .throughput(Throughput::Bytes(length as u64))
        .bench_function("encode_stream 32k", |b| {
            b.iter_batched(
                || {
                    (
                        Cursor::new(buf.clone()),
                        Cursor::new(vec![0u8; output_capacity]),
                    )
                },
                |(mut input_r, mut output_r)| {
                    options
                        .encode_stream(&mut input_r, &mut output_r, length as u64, "test")
                        .unwrap()
                },
                BatchSize::LargeInput,
            )
        });
    group.finish();
}

criterion_group!(benches, encode_buffer, encode_stream);
criterion_main!(benches);
