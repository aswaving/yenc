extern crate rand;
extern crate yenc;

use rand::random;
use std::env::temp_dir;
use std::fs::{create_dir, remove_file, File};
use std::io::{Read, Write};
use std::path::Path;

fn encode(input_filename: &str) {
    let parts = 1;

    println!("{}", input_filename);
    let input_file = File::open(&input_filename).expect("Cannot open file");

    let path = Path::new(&input_filename);
    let total_size = input_file.metadata().unwrap().len();
    let part_size: u64 = total_size / u64::from(parts);

    for part in 1..parts + 1 {
        let output_filename = format!("{}.{:03}", input_filename, part);
        let mut output_file = File::create(&output_filename).expect("Cannot create file");
        println!("{}", output_filename);

        let begin = (u64::from(part) - 1) * part_size + 1;
        let end = if begin + part_size < total_size {
            begin + part_size - 1
        } else {
            total_size
        };

        let encode_options = yenc::EncodeOptions::default()
            .parts(parts)
            .part(part)
            .begin(begin)
            .end(end);

        match yenc::encode_file(path, &encode_options, &mut output_file) {
            Err(err) => {
                println!(
                    "Error yEncoding {} to {}: {}",
                    input_filename, output_filename, err
                );
            }
            Ok(_) => {
                println!(
                    "Successfully yEncoded {} to {}",
                    input_filename, output_filename
                );
            }
        };
    }
}

fn decode(input_filename: &str, output_directory: &str) -> u32 {
    match yenc::decode_file(&input_filename, &output_directory) {
        Err(err) => {
            println!("Error yEnc decoding {}: {}", input_filename, err);
            1
        }
        Ok(output_filename) => {
            println!(
                "Successfully yEnc decoded {} to {}",
                input_filename, output_filename
            );
            0
        }
    }
}

fn encode_decode_are_equal(data: &[u8], filename: &str) -> bool {
    // create temp dir
    let tmpdir = temp_dir();
    println!("{}", tmpdir.display());
    // created 'decoded' dir in temp dir
    let mut decoded_dir = tmpdir.clone();
    decoded_dir.push("decoded");
    let _ = create_dir(decoded_dir.clone());

    // dump data to file
    let mut filepath = tmpdir.clone();
    filepath.push(filename);
    let mut f = File::create(&filepath).unwrap();
    f.write(data).unwrap();

    // encode file
    encode(filepath.to_str().unwrap());

    let mut decoded_file = decoded_dir.clone();
    decoded_file.push(filename);

    remove_file(&decoded_file).unwrap();

    // decode file
    decode(
        &(filepath.to_str().unwrap().to_owned() + ".001"),
        decoded_dir.to_str().unwrap(),
    );

    // check that files are identical
    let mut decoded_file = decoded_dir.clone();
    decoded_file.push(filename);
    identical(filepath, decoded_file)
}

fn identical<P: AsRef<Path>>(file1: P, file2: P) -> bool {
    let mut data1 = Vec::new();
    let size1 = File::open(file1).unwrap().read_to_end(&mut data1).unwrap();
    let mut data2 = Vec::new();
    let size2 = File::open(file2).unwrap().read_to_end(&mut data2).unwrap();
    size1 == size2 && &data1 == &data2
}

#[test]
fn test_ascii() {
    let data = (0..10_000_000)
        .map(|c| (c & 0x7f) as u8)
        .collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "ascii"));
}

#[test]
fn test_binary() {
    let data = (0..10_000_000)
        .map(|c| (c & 0xff) as u8)
        .collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "binary"));
}

#[test]
fn test_random() {
    let data = (0..10_000_000).map(|_| random::<u8>()).collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "random"));
}
