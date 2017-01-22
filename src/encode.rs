use crc32;
use constants::{NUL, CR, LF, SPACE, TAB, ESCAPE, DEFAULT_LINE_SIZE};

use std::fs::File;
use std::io;
use std::io::{Read, Write, Seek, SeekFrom};

/// Options for encoding
#[derive(Debug)]
pub struct EncodeOptions {
    line_length: u8,
    parts: u32,
    part: u32,
    begin: u64,
    end: u64,
}

impl EncodeOptions {
    /// Constructs a new EncodeOptions instance, with the following defaults:
    /// line_length = 128.
    /// parts = 1,
    /// part = begin = end = 0
    pub fn new() -> EncodeOptions {
        EncodeOptions {
            line_length: 128,
            parts: 1,
            part: 0,
            begin: 0,
            end: 0,
        }
    }

    /// Sets the maximum line length.
    pub fn line_length(mut self, line_length: u8) -> EncodeOptions {
        self.line_length = line_length;
        self
    }

    /// Sets the number of parts (default=1).
    /// When the number of parts equals 1, no '=ypart' line will be written
    /// in the ouput.
    pub fn parts(mut self, parts: u32) -> EncodeOptions {
        self.parts = parts;
        self
    }

    /// Sets the part number.
    /// Only used when `parts > 1`.
    /// The part number count starts at 1.
    pub fn part(mut self, part: u32) -> EncodeOptions {
        self.part = part;
        self
    }

    /// Sets the begin (which is the file offset + 1).
    /// Only used when `parts > 1`.
    /// The size of the part is `end - begin + 1`.
    pub fn begin(mut self, begin: u64) -> EncodeOptions {
        self.begin = begin;
        self
    }

    /// Sets the end.
    /// Only used when `parts > 1`.
    /// The size of the part is `end - begin + 1`.
    /// `end` should be larger than `begin`, otherwise an overflow error occurrs.
    pub fn end(mut self, end: u64) -> EncodeOptions {
        self.end = end;
        self
    }
}

/// Encodes the input file in a new output file.
/// # Example
/// ```rust,no_run
/// let mut file = std::fs::File::open("test1.bin").unwrap();
/// let encode_options = yenc::EncodeOptions::new().parts(1);
/// let mut output_file = std::fs::File::create("test1.bin.yenc").unwrap();
/// yenc::yencode_file(&mut file, "test1.bin", encode_options, &mut output_file);
/// ```
/// # Errors
/// - when the output file already exists
///
pub fn yencode_file(input_file: &mut File,
                    input_filename: &str,
                    encode_options: EncodeOptions,
                    output: &mut Write)
                    -> Result<(), io::Error> {
    let mut checksum = crc32::Crc32::new();
    let mut buffer = [0u8; 8192];
    let mut col = 0;

    output.write(format!("=ybegin line={} size={} name={}\r\n",
                       DEFAULT_LINE_SIZE,
                       input_file.metadata()?.len(),
                       input_filename)
            .as_bytes())?;

    if encode_options.parts > 1 {
        output.write(format!("=ypart part={} begin={} end={}\r\n",
                           encode_options.part,
                           encode_options.begin,
                           encode_options.end)
                .as_bytes())?;
    }

    input_file.seek(SeekFrom::Start(encode_options.begin - 1));

    let mut remainder = (encode_options.end - encode_options.begin + 1) as usize;
    while remainder > 0 {
        let bytes_to_read = if remainder > buffer.len() {
            buffer.len()
        } else {
            remainder
        };
        input_file.read_exact(&mut buffer[0..bytes_to_read])?;
        checksum.update_with_slice(&buffer[0..bytes_to_read]);
        output.write(yencode_buffer(&buffer[0..bytes_to_read],
                                  &mut col,
                                  encode_options.line_length)
                .as_slice())?;
        remainder -= bytes_to_read;
    }

    if encode_options.parts > 1 {
        output.write(format!("\r\n=yend part={} size={} pcrc32={:08x}\r\n",
                           encode_options.part,
                           checksum.num_bytes,
                           checksum.crc)
                .as_bytes())?;
    } else {
        output.write(format!("\r\n=yend size={} crc32={:08x}\r\n",
                           checksum.num_bytes,
                           checksum.crc)
                .as_bytes())?;
    }
    Ok(())
}

/// Encode the byte slice into a vector of yEncoded bytes, with the maximum of `line_length` characters per line.
pub fn yencode_buffer(input: &[u8], col: &mut u8, line_length: u8) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(input.len());
    for &b in input {
        let encoded = yencode_byte(b);
        output.extend_from_slice(&encoded);
        *col += encoded.len() as u8;
        if *col >= line_length {
            output.push(CR);
            output.push(LF);
            *col = 0;
        }
    }
    output
}

#[inline]
fn yencode_byte(input_byte: u8) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(2);
    let mut output_byte = input_byte.overflowing_add(42).0;
    match output_byte {
        NUL | CR | LF | ESCAPE | TAB | SPACE => {
            output.push(ESCAPE);
            output_byte = output_byte.overflowing_add(64).0;
        }
        _ => {}
    };
    output.push(output_byte);
    output
}


#[cfg(test)]
mod tests {
    use super::{yencode_byte, yencode_buffer};
    use super::super::constants::{ESCAPE, TAB, CR, LF, SPACE};

    #[test]
    fn escape_null() {
        assert_eq!(vec![ESCAPE, 0x40], yencode_byte(214));
    }

    #[test]
    fn escape_tab() {
        assert_eq!(vec![ESCAPE, 0x49], yencode_byte(214 + TAB));
    }

    #[test]
    fn escape_lf() {
        assert_eq!(vec![ESCAPE, 0x4A], yencode_byte(214 + LF));
    }

    #[test]
    fn escape_cr() {
        assert_eq!(vec![ESCAPE, 0x4D], yencode_byte(214 + CR));
    }

    #[test]
    fn escape_space() {
        assert_eq!(vec![ESCAPE, 0x60], yencode_byte(214 + SPACE));
    }

    #[test]
    fn escape_equal_sign() {
        assert_eq!(vec![ESCAPE, 0x7D], yencode_byte(ESCAPE - 42));
    }
}
