use rand::random;
use std::env::temp_dir;
use std::fs::{create_dir, remove_dir, remove_file, File};
use std::io::{Read, Result, Write};
use std::path::Path;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref M: Mutex<u8> = Mutex::new(0);
}

fn encode(input_filename: &str) {
    let parts = 1;

    println!("{}", input_filename);
    let input_file = File::open(&input_filename).expect("Cannot open file");

    let path = Path::new(&input_filename);
    let total_size = input_file.metadata().unwrap().len();
    let part_size: u64 = total_size / u64::from(parts);

    for part in 1..=parts {
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

        match encode_options.encode_file(path, &mut output_file) {
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
    let decode_options = yenc::DecodeOptions::new(output_directory);
    match decode_options.decode_file(&input_filename) {
        Err(err) => {
            println!("Error yEnc decoding {}: {}", input_filename, err);
            1
        }
        Ok(output_filename) => {
            println!(
                "Successfully yEnc decoded {} to {}",
                input_filename,
                output_filename.display()
            );
            0
        }
    }
}

fn encode_decode_are_equal(data: &[u8], filename: &str) -> Result<bool> {
    // synchronize, to prevent test cases run in parallel and mess up directories
    let _x = M.lock().unwrap();

    // create temp dir
    let tmpdir = temp_dir();
    println!("{}", tmpdir.display());

    // created 'decoded' dir in temp dir
    let mut decoded_dir = tmpdir.clone();
    decoded_dir.push("decoded");
    create_dir(decoded_dir.clone())?;

    // dump data to file
    let mut filepath = tmpdir;
    filepath.push(filename);
    let mut f = File::create(&filepath)?;
    f.write_all(data).unwrap();

    // encode file
    encode(filepath.to_str().unwrap());

    let mut decoded_file = decoded_dir.clone();
    decoded_file.push(filename);

    // decode file
    decode(
        &(filepath.to_str().unwrap().to_owned() + ".001"),
        decoded_dir.to_str().unwrap(),
    );

    // check that files are identical
    let mut decoded_file = decoded_dir.clone();
    decoded_file.push(filename);
    let result = identical(filepath.clone(), decoded_file.clone());

    //clean up
    remove_file(filepath)?;
    remove_file(decoded_file)?;
    remove_dir(decoded_dir)?;

    Ok(result)
}

fn identical<P: AsRef<Path>>(file1: P, file2: P) -> bool {
    let mut data1 = Vec::new();
    let size1 = File::open(file1).unwrap().read_to_end(&mut data1).unwrap();
    let mut data2 = Vec::new();
    let size2 = File::open(file2).unwrap().read_to_end(&mut data2).unwrap();
    size1 == size2 && data1 == data2
}

#[test]
fn test_ascii() {
    let data = (0..10_000_000)
        .map(|c| (c & 0x7f) as u8)
        .collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "ascii").unwrap());
}

#[test]
fn test_binary() {
    let data = (0..10_000_000)
        .map(|c| (c & 0xff) as u8)
        .collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "binary").unwrap());
}

#[test]
fn test_random() {
    let data = (0..10_000_000).map(|_| random::<u8>()).collect::<Vec<u8>>();

    assert!(encode_decode_are_equal(&data, "random").unwrap());
}
