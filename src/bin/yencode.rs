extern crate yenc;

use std::fs::File;

use std::str::FromStr;
use std::path::Path;
use std::env::args;

fn main() {
    let input_filename = args().nth(1).expect("Specify input file");
    let parts = u32::from_str(&args().nth(2).expect("Specify number of parts")).unwrap();


    let mut input_file = File::open(&input_filename).expect("Cannot open file");

    let path = Path::new(&input_filename);
    let filename = path.file_name().unwrap().to_str().unwrap(); // yikes!
    let total_size = input_file.metadata().unwrap().len();
    let part_size: u64 = total_size / (parts as u64);

    for part in 1..parts + 1 {
        let output_filename = format!("{}.{:03}", input_filename, part);
        let mut output_file = File::create(&output_filename).expect("Cannot create file");

        let begin = (part as u64 - 1) * part_size + 1;
        let end = if begin + part_size < total_size {
            begin + part_size - 1
        } else {
            total_size
        };

        let encode_options =
            yenc::EncodeOptions::new().parts(parts).part(part).begin(begin).end(end);

        match yenc::yencode_file(&mut input_file, filename, encode_options, &mut output_file) {
            Err(err) => {
                println!("Error yEncoding {} to {}: {}",
                         input_filename,
                         output_filename,
                         err);
            }
            Ok(_) => {
                println!("Successfully yEncoded {} to {}",
                         input_filename,
                         output_filename);
            }
        };
    }
}
