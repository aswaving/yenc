use super::constants::{CR, DEFAULT_LINE_SIZE, DOT, ESCAPE, LF, NUL};
use super::errors::EncodeError;

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Options for encoding.
/// The entry point for encoding a file (part)
/// to a file or (TCP) stream.
#[derive(Debug)]
#[must_use]
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
    /// Constructs a new EncodeOptions with defaults, see Default impl.
    pub fn new() -> EncodeOptions {
        Default::default()
    }

    /// Sets the maximum line length.
    pub fn line_length(mut self, line_length: u8) -> EncodeOptions {
        self.line_length = line_length;
        self
    }

    /// Sets the number of parts (default=1).
    /// When the number of parts is 1, no '=ypart' line will be written
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

    /// Encodes the input file and writes it to the writer. For multi-part encoding, only
    /// one part is encoded. In case of multipart, the part number, begin and end offset need
    /// to be specified in the `EncodeOptions`. When directly encoding to an NNTP stream, the
    /// caller needs to take care of the message header and end of multi-line block (`".\r\n"`).
    ///
    /// # Example
    /// ```rust,no_run
    /// let encode_options = yenc::EncodeOptions::default()
    ///                                         .parts(2)
    ///                                         .part(1)
    ///                                         .begin(1)
    ///                                         .end(38400);
    /// let mut output_file = std::fs::File::create("test1.bin.yenc.001").unwrap();
    /// encode_options.encode_file("test1.bin", &mut output_file).unwrap();
    /// ```
    /// # Errors
    /// - when the output file already exists
    ///
    pub fn encode_file<P, W>(&self, input_path: P, output: W) -> Result<(), EncodeError>
    where
        P: AsRef<Path>,
        W: Write,
    {
        let input_filename = input_path.as_ref().file_name();
        let input_filename = match input_filename {
            Some(s) => s.to_str().unwrap_or(""),
            None => "",
        };
        let input_file = File::open(&input_path)?;
        let length = input_file.metadata()?.len();

        self.encode_stream(input_file, output, length, input_filename)
    }

    /// Checks the options. Returns Ok(()) if all options are ok.
    /// # Return
    /// - EncodeError::PartNumberMissing
    /// - EncodeError::PartBeginOffsetMissing
    /// - EncodeError::PartEndOffsetMissing
    /// - EncodeError::PartOffsetsInvalidRange
    pub fn check_options(&self) -> Result<(), EncodeError> {
        if self.parts > 1 && self.part == 0 {
            return Err(EncodeError::PartNumberMissing);
        }
        if self.parts > 1 && self.begin == 0 {
            return Err(EncodeError::PartBeginOffsetMissing);
        }
        if self.parts > 1 && self.end == 0 {
            return Err(EncodeError::PartEndOffsetMissing);
        }
        if self.parts > 1 && self.begin > self.end {
            return Err(EncodeError::PartOffsetsInvalidRange);
        }
        Ok(())
    }

    /// Encodes the date from input from stream and writes the encoded data to the output stream.
    /// The input stream does not need to be a file, therefore, size and `input_filename``
    /// must be specified. The `input_filename` ends up as the filename in the yenc header.
    #[allow(clippy::write_with_newline)]
    pub fn encode_stream<R, W>(
        &self,
        input: R,
        output: W,
        length: u64,
        input_filename: &str,
    ) -> Result<(), EncodeError>
    where
        R: Read + Seek,
        W: Write,
    {
        self.check_options()?;
        let mut rdr = BufReader::new(input);
        let mut checksum = crc32fast::Hasher::new();
        let mut buffer = [0u8; 8192];
        let mut col = 0;
        let mut num_bytes = 0;
        let mut output = BufWriter::new(output);

        if self.parts == 1 {
            write!(
                output,
                "=ybegin line={} size={} name={}\r\n",
                self.line_length, length, input_filename
            )?;
        } else {
            write!(
                output,
                "=ybegin part={} line={} size={} name={}\r\n",
                self.part, self.line_length, length, input_filename
            )?;
        }

        if self.parts > 1 {
            write!(output, "=ypart begin={} end={}\r\n", self.begin, self.end)?;
        }

        let (seek_pos, mut remainder) = if self.parts == 1 {
            let r = usize::try_from(length).map_err(|_| EncodeError::PartOffsetsInvalidRange)?;
            (0u64, r)
        } else {
            let r = usize::try_from(self.end - self.begin + 1)
                .map_err(|_| EncodeError::PartOffsetsInvalidRange)?;
            (self.begin - 1, r)
        };
        rdr.seek(SeekFrom::Start(seek_pos))?;

        while remainder > 0 {
            let buf_slice = if remainder > buffer.len() {
                &mut buffer[..]
            } else {
                &mut buffer[0..remainder]
            };
            rdr.read_exact(buf_slice)?;
            checksum.update(buf_slice);
            num_bytes += buf_slice.len();
            col = encode_buffer(buf_slice, col, self.line_length, &mut output)?;
            remainder -= buf_slice.len();
        }

        if self.parts > 1 {
            write!(
                output,
                "\r\n=yend size={} part={} pcrc32={:08x}\r\n",
                num_bytes,
                self.part,
                checksum.finalize()
            )?;
        } else {
            write!(
                output,
                "\r\n=yend size={} crc32={:08x}\r\n",
                num_bytes,
                checksum.finalize()
            )?;
        }
        Ok(())
    }
}

/// Encodes the input buffer and writes it to the writer.
///
/// Lines are wrapped with a maximum of `line_length` characters per line.
/// Does not include the header and footer lines.
/// Only `encode_stream` and `encode_file` produce the headers in the output.
/// The `col` parameter is the starting offset in the row. The result contains the new offset.
pub fn encode_buffer<W>(
    input: &[u8],
    col: u8,
    line_length: u8,
    writer: W,
) -> Result<u8, EncodeError>
where
    W: Write,
{
    let mut col = col;
    let mut writer = writer;
    // Stack-allocated line buffer: worst case all bytes escaped (×2) + CRLF.
    // line_length is u8 (max 255), so max bytes per line = 255×2 + 2 = 512.
    let mut line_buf = [0u8; 514];
    let mut line_len = 0usize;

    for &b in input {
        let encoded = encode_byte(b);
        match encoded.0 {
            ESCAPE => {
                line_buf[line_len] = ESCAPE;
                line_buf[line_len + 1] = encoded.1;
                line_len += 2;
                col = col.wrapping_add(2);
            }
            DOT if col == 0 => {
                line_buf[line_len] = DOT;
                line_buf[line_len + 1] = DOT;
                line_len += 2;
                col = col.wrapping_add(2);
            }
            out => {
                line_buf[line_len] = out;
                line_len += 1;
                col = col.wrapping_add(1);
            }
        }
        if col >= line_length {
            line_buf[line_len] = CR;
            line_buf[line_len + 1] = LF;
            line_len += 2;
            writer.write_all(&line_buf[..line_len])?;
            col = 0;
            line_len = 0;
        }
    }
    if line_len > 0 {
        writer.write_all(&line_buf[..line_len])?;
    }
    Ok(col)
}

const fn build_encode_table() -> [(u8, u8); 256] {
    let mut table = [(0u8, 0u8); 256];
    let mut i = 0usize;
    while i < 256 {
        let output_byte = (i as u8).wrapping_add(42);
        table[i] = match output_byte {
            NUL | LF | CR | ESCAPE => (ESCAPE, output_byte.wrapping_add(64)),
            _ => (output_byte, 0),
        };
        i += 1;
    }
    table
}

const ENCODE_TABLE: [(u8, u8); 256] = build_encode_table();

#[inline(always)]
fn encode_byte(input_byte: u8) -> (u8, u8) {
    ENCODE_TABLE[input_byte as usize]
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::super::constants::{CR, ESCAPE, LF, NUL};
    use super::*;

    #[test]
    fn escape_null() {
        assert_eq!((ESCAPE, 0x40), encode_byte(214));
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
        assert_eq!((ESCAPE, 0x4A), encode_byte(214 + LF));
    }

    #[test]
    fn escape_cr() {
        assert_eq!((ESCAPE, 0x4D), encode_byte(214 + CR));
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
        assert_eq!((ESCAPE, 0x7D), encode_byte(ESCAPE - 42));
    }

    #[test]
    fn non_escaped() {
        for x in 0..256u16 {
            let encoded = (x as u8).overflowing_add(42).0;
            if encoded != NUL && encoded != CR && encoded != LF && encoded != ESCAPE {
                assert_eq!((encoded, 0), encode_byte(x as u8));
            }
        }
    }

    #[test]
    fn test_encode_buffer() {
        let buffer = (0..256u16).map(|c| c as u8).collect::<Vec<u8>>();
        #[rustfmt::skip]
        const EXPECTED: [u8; 264] =
                       [42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 
                       125, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 
                       81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 
                       101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 
                       117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 
                       133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 
                       149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 
                       165, 166, 167, 168, 13, 10, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 
                       179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 
                       195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 
                       211, 212, 213, 214, 215, 216,217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 
                       227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 
                       243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 61, 64, 1, 2, 3, 
                       4, 5, 6, 7, 8, 9, 61, 74, 11, 12, 61, 77, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 
                       24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 13, 10, 38, 39, 40, 41];
        let mut encoded = Vec::<u8>::new();
        let result = encode_buffer(&buffer, 0, 128, &mut encoded);
        assert!(result.is_ok());
        assert_eq!(encoded.as_slice(), &EXPECTED[..]);
    }

    #[test]
    fn encode_options_invalid_parts() {
        let encode_options = EncodeOptions::new().parts(2).begin(1).end(38400);
        let vr = encode_options.check_options();
        assert!(vr.is_err());
    }

    #[test]
    fn encode_options_invalid_begin() {
        let encode_options = EncodeOptions::new().parts(2).part(1).end(38400);
        let vr = encode_options.check_options();
        assert!(vr.is_err());
    }

    #[test]
    fn encode_options_invalid_end() {
        let encode_options = EncodeOptions::new().parts(2).part(1).begin(1);
        let vr = encode_options.check_options();
        assert!(vr.is_err());
    }

    #[test]
    fn encode_options_invalid_range() {
        let encode_options = EncodeOptions::new().parts(2).part(1).begin(38400).end(1);
        let vr = encode_options.check_options();
        assert!(vr.is_err());
    }

    #[test]
    fn encode_options_line_length() {
        let input = Cursor::new((0..512).map(|c| (c & 0xff) as u8).collect::<Vec<u8>>());
        let mut output = Vec::new();
        let output_cursor = Cursor::new(&mut output);

        let encode_options = EncodeOptions::new().line_length(80);
        encode_options
            .encode_stream(input, output_cursor, 80, "line_length_test")
            .unwrap();
        // Use lines() to skip the =ybegin header and check the first data line.
        // lines() strips the line ending, so len() == line_length for a full line.
        let first_data_line = String::from_utf8(output)
            .unwrap()
            .lines()
            .find(|l| !l.starts_with('='))
            .map(|l| l.to_owned())
            .unwrap();
        assert_eq!(80, first_data_line.len());
    }
}
