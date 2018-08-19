use constants::{CR, DEFAULT_LINE_SIZE, ESCAPE, LF, NUL};
use crc32;

use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Options for encoding
#[derive(Debug)]
pub struct EncodeOptions {
    line_length: u8,
    parts: u32,
    part: u32,
    begin: u64,
    end: u64,
}

impl Default for EncodeOptions {
    /// Constructs a new EncodeOptions instance, with the following defaults:
    /// line_length = 128.
    /// parts = 1,
    /// part = begin = end = 0
    fn default() -> Self {
        EncodeOptions {
            line_length: DEFAULT_LINE_SIZE,
            parts: 1,
            part: 0,
            begin: 0,
            end: 0,
        }
    }
}

impl EncodeOptions {
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
/// let encode_options = yenc::EncodeOptions::default().parts(1);
/// let mut output_file = std::fs::File::create("test1.bin.yenc").unwrap();
/// yenc::encode_file("test1.bin", &encode_options, &mut output_file);
/// ```
/// # Errors
/// - when the output file already exists
///
pub fn encode_file<P: AsRef<Path>>(
    input_path: P,
    encode_options: &EncodeOptions,
    output: &mut Write,
) -> Result<(), io::Error> {
    let mut checksum = crc32::Crc32::new();
    let mut buffer = [0u8; 8192];
    let mut col = 0;
    let input_filename = input_path.as_ref().file_name();
    let input_filename = match input_filename {
        Some(s) => s.to_str().unwrap_or(""),
        None => "",
    };
    let mut input_file = File::open(&input_path)?;

    if encode_options.parts == 1 {
        output.write_all(
            format!(
                "=ybegin line={} size={} name={}\r\n",
                encode_options.line_length,
                input_file.metadata()?.len(),
                input_filename
            ).as_bytes(),
        )?;
    } else {
        output.write_all(
            format!(
                "=ybegin part={} line={} size={} name={}\r\n",
                encode_options.part,
                encode_options.line_length,
                input_file.metadata()?.len(),
                input_filename
            ).as_bytes(),
        )?;
    }

    if encode_options.parts > 1 {
        output.write_all(
            format!(
                "=ypart begin={} end={}\r\n",
                encode_options.begin, encode_options.end
            ).as_bytes(),
        )?;
    }

    input_file.seek(SeekFrom::Start(encode_options.begin - 1))?;

    let mut remainder = (encode_options.end - encode_options.begin + 1) as usize;
    while remainder > 0 {
        let bytes_to_read = if remainder > buffer.len() {
            buffer.len()
        } else {
            remainder
        };
        input_file.read_exact(&mut buffer[0..bytes_to_read])?;
        checksum.update_with_slice(&buffer[0..bytes_to_read]);
        encode_buffer(
            &buffer[0..bytes_to_read],
            &mut col,
            encode_options.line_length,
            output,
        );
        remainder -= bytes_to_read;
    }

    if encode_options.parts > 1 {
        output.write_all(
            format!(
                "\r\n=yend size={} part={} pcrc32={:08x}\r\n",
                checksum.num_bytes, encode_options.part, checksum.crc
            ).as_bytes(),
        )?;
    } else {
        output.write_all(
            format!(
                "\r\n=yend size={} crc32={:08x}\r\n",
                checksum.num_bytes, checksum.crc
            ).as_bytes(),
        )?;
    }
    Ok(())
}

/// Encode the byte slice into a vector of yEncoded bytes.
///
/// Lines are wrapped with a maximum of `line_length` characters per line.
/// Does not include the header and footer lines. These are only produced
/// by `encode_stream` and `encode_file`.
pub fn encode_buffer(input: &[u8], col: &mut u8, line_length: u8, writer: &mut Write) {
    for &b in input {
        let (encoded, encoded_len) = encode_byte(b);
        writer.write_all(&encoded[0..encoded_len]).unwrap();
        *col += encoded_len as u8;
        if *col >= line_length {
            writer.write_all(&[CR, LF]).unwrap();
            *col = 0;
        }
    }
}

#[inline]
fn encode_byte(input_byte: u8) -> ([u8; 2], usize) {
    let mut output = (0, 0);

    let output_byte = input_byte.overflowing_add(42).0;
    let len = match output_byte {
        NUL | CR | LF | ESCAPE => {
            output.0 = ESCAPE;
            output.1 = output_byte.overflowing_add(64).0;
            2
        }
        _ => {
            output.0 = output_byte;
            1
        }
    };
    let output_array = [output.0, output.1];
    (output_array, len)
}

#[cfg(test)]
mod tests {
    use super::super::constants::{CR, ESCAPE, LF};
    use super::{encode_buffer, encode_byte};

    #[test]
    fn escape_null() {
        assert_eq!(([ESCAPE, 0x40], 2), encode_byte(214));
    }

    /*
    #[test]
    fn escape_tab() {
        let mut output = [0u8; 2];
        assert_eq!(2, encode_byte(214 + TAB, &mut output));
        assert_eq!(vec![ESCAPE, 0x49], output);
    }
    */

    #[test]
    fn escape_lf() {
        assert_eq!(([ESCAPE, 0x4A], 2), encode_byte(214 + LF));
    }

    #[test]
    fn escape_cr() {
        assert_eq!(([ESCAPE, 0x4D], 2), encode_byte(214 + CR));
    }

    /*    
    #[test]
    fn escape_space() {
        let mut output = [0u8; 2];
        assert_eq!(2, encode_byte(214 + SPACE, &mut output));
        assert_eq!(vec![ESCAPE, 0x60], output);
    }
    */

    #[test]
    fn escape_equal_sign() {
        assert_eq!(([ESCAPE, 0x7D], 2), encode_byte(ESCAPE - 42));
    }

    #[test]
    fn non_escaped() {
        assert_eq!(([42, 0], 1), encode_byte(0));
    }
}
