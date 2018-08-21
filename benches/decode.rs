#[macro_use]
extern crate criterion;

extern crate yenc;

use std::env::temp_dir;
use std::io::Cursor;

use criterion::Criterion;

fn decode(c: &mut Criterion) {
    // write random data in file
    let buf = (0..32_768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
    let length = buf.len() as u64;
    
    let mut out = Vec::<u8>::with_capacity(32768 * 102 / 100);
    let options = yenc::EncodeOptions::default().begin(1).end(length);
    let cur = Cursor::new(buf);
    yenc::encode_stream(cur, &mut out, length, "test", &options).unwrap();

    c.bench_function("decode 32k", move |b| {
        let mut cur = Cursor::new(out.clone());
        let output_dir = temp_dir();
        let output_dir = output_dir.to_str().unwrap();
        b.iter(|| yenc::decode_stream(&mut cur, output_dir).unwrap())
    });
}

criterion_group!(benches, decode);
criterion_main!(benches);
