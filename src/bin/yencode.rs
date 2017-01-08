extern crate yenc;

use std::env::args;
use std::process::exit;

fn main() {
    let input_filename = args().nth(1).expect("Specify input file");
    let output_filename = format!("{}.yenc", input_filename);

    exit(match yenc::yencode_file(&input_filename, &output_filename) {
        Err(err) => {
            println!("Error yEncoding {} to {}: {:?}",
                     input_filename,
                     output_filename,
                     err);
            1
        }
        Ok(_) => {
            println!("Successfully yEncoded {} to {}",
                     input_filename,
                     output_filename);
            0
        }
    });
}