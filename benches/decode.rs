use criterion::*;

fn decode_buffer(c: &mut Criterion) {
    let mut buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    let length = buf.len();
    let mut encoded = Vec::with_capacity(32_768 * 102 / 100);
    yenc::encode_buffer(&buf, 0, 128, &mut encoded).unwrap();

    let mut group = c.benchmark_group("decode");

    group
        .bench_function("decode 32k", move |b| {
            buf.clear();
            b.iter(|| yenc::decode_buffer(&encoded).unwrap())
        })
        .throughput(Throughput::Bytes(length as u64));

    group.finish();
}

fn decode_stream(c: &mut Criterion) {
    let buf = (0..32768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    let length = buf.len();
    let options = yenc::EncodeOptions::new().begin(1).end(length as u64);
    let output = vec![0; length * 110 / 100];
    let mut input_r = std::io::Cursor::new(buf);
    let mut output_r = std::io::Cursor::new(output);
    options
        .encode_stream(&mut input_r, &mut output_r, length as u64, "test")
        .unwrap();
    let input = output_r.into_inner();

    let mut group = c.benchmark_group("decode_stream");
    group
        .bench_function("decode_stream 32k", move |b| {
            b.iter(|| {
                let i = input.clone();
                let mut input_r = std::io::Cursor::new(i);
                let options = yenc::DecodeOptions::new("/tmp", None);
                options.decode_stream(&mut input_r).unwrap();
            });
        })
        .throughput(Throughput::Bytes(length as u64));
}

criterion_group!(benches, decode_buffer, decode_stream);
criterion_main!(benches);
