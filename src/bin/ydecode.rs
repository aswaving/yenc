extern crate yenc;

use std::env::args;
use std::process::exit;

fn main() {
    let input_filename = args().nth(1).expect("Specify input file");
    let output_directory = args().nth(2).expect("Specify output directory");

    exit(match yenc::ydecode_file(&input_filename, &output_directory) {
        Err(err) => {
            println!("Error yEnc decoding {}: {}", input_filename, err);
            1
        }
        Ok(output_filename) => {
            println!("Successfully yEnc decoded {} to {}",
                     input_filename,
                     output_filename);
            0
        }
    });
}
