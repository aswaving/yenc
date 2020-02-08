use criterion::*;
use std::io::Cursor;
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
        .throughput(Throughput::Bytes(length as u32)),
    );
}
fn encode_stream(c: &mut Criterion) {
    c.bench(
        "encode_stream",
        Benchmark::new("encode_stream 32k", |b| {
            b.iter(|| {
                let buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
                assert_eq!(32_768, buf.len());
                let length = buf.len();
                let options = yenc::EncodeOptions::new().end(length as u64).begin(1);
                let output = vec![0; length * 110 /100];
                let mut input_r = Cursor::new(buf);
                let mut output_r = Cursor::new(output);
                options
                    .encode_stream(&mut input_r, &mut output_r, length as u64, "test")
                    .unwrap();
            })
        })
        .throughput(Throughput::Bytes(32_768 as u32)),
    );
}

criterion_group!(benches, encode_buffer, encode_stream);
criterion_main!(benches);
