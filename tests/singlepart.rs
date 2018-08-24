extern crate yenc;

use std::env::temp_dir;
use std::fs::File;
use std::io::Read;

#[test]
fn encode() {
    let data = include_bytes!("../testdata/yenc.org/testfile.txt");
    let expected_encoded = include_bytes!("../testdata/yenc.org/testfile.txt.yenc");

    let mut encoded = Vec::<u8>::new();
    let options = yenc::EncodeOptions::new()
        .begin(1)
        .end(data.len() as u64);
    let mut c = std::io::Cursor::new(&data[..]);
    yenc::encode_stream(
        &mut c,
        &mut encoded,
        data.len() as u64,
        "testfile.txt",
        &options,
    ).unwrap();

    assert_eq!(encoded.as_slice(), &expected_encoded[..]);
}

#[test]
fn decode() {
    let data = include_bytes!("../testdata/yenc.org/testfile.txt.yenc");
    let expected_decoded = include_bytes!("../testdata/yenc.org/testfile.txt");
    let mut decoded = Vec::<u8>::new();
    let mut c = std::io::Cursor::new(&data[..]);
    let tmpdir = temp_dir();
    let mut tmpfile = tmpdir.clone();
    let tmpdir_str = tmpdir.to_string_lossy();
    tmpfile.push("testfile.txt");
    yenc::decode_stream(&mut c, &tmpdir_str).unwrap();
    File::open(&tmpfile)
        .unwrap()
        .read_to_end(&mut decoded)
        .unwrap();
    assert_eq!(decoded.as_slice(), &expected_decoded[..]);
}
