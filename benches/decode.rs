use criterion::*;
use std::io::Cursor;

fn decode_buffer(c: &mut Criterion) {
    let buf = (0..32_768_u32)
        .map(|i| (i % 256) as u8)
        .collect::<Vec<u8>>();
    let length = buf.len();
    let mut encoded = Vec::with_capacity(length * 102 / 100);
    yenc::encode_buffer(&buf, 0, 128, &mut encoded).unwrap();
    let mut output = Vec::with_capacity(length);

    let mut group = c.benchmark_group("decode");
    group
        .throughput(Throughput::Bytes(length as u64))
        .bench_function("decode 32k", |b| {
            b.iter(|| {
                output.clear();
                yenc::decode_buffer(&encoded, &mut output).unwrap()
            })
        });
    group.finish();
}

fn decode_stream(c: &mut Criterion) {
    let buf = (0..32_768_u32)
        .map(|i| (i % 256) as u8)
        .collect::<Vec<u8>>();
    let length = buf.len();
    let options = yenc::EncodeOptions::new();
    let output = vec![0u8; length * 110 / 100];
    let mut input_r = Cursor::new(buf);
    let mut output_r = Cursor::new(output);
    options
        .encode_stream(&mut input_r, &mut output_r, length as u64, "test")
        .unwrap();
    let encoded = output_r.into_inner();

    let mut group = c.benchmark_group("decode_stream");
    group
        .throughput(Throughput::Bytes(length as u64))
        .bench_function("decode_stream 32k", |b| {
            b.iter_batched(
                || Cursor::new(encoded.clone()),
                |mut input_r| {
                    yenc::DecodeOptions::new("/tmp")
                        .decode_stream(&mut input_r)
                        .unwrap()
                },
                BatchSize::LargeInput,
            )
        });
    group.finish();
}

criterion_group!(benches, decode_buffer, decode_stream);
criterion_main!(benches);
