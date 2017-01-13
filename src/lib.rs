//! [yEnc](http://www.yenc.org) is an encoding scheme to include binary files in Usenet messages.
mod crc32;

use std::fs::OpenOptions;
use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::path::PathBuf;
use std::convert::From;

const NUL: u8 = 0;
const TAB: u8 = b'\t';
const LF: u8 = b'\n';
const CR: u8 = b'\r';
const SPACE: u8 = b' ';
const ESCAPE: u8 = b'=';
const LINE_SIZE: u16 = 128;

#[derive(Debug)]
pub enum DecodeError {
    IncompleteData(usize, usize),
    InvalidHeader(String, usize),
    InvalidChecksum,
    IoError(io::Error),
}

impl From<io::Error> for DecodeError {
    fn from(error: io::Error) -> DecodeError {
        DecodeError::IoError(error)
    }
}

#[derive(Default,Debug)]
struct MetaData {
    name: Option<String>,
    line: Option<u16>,
    size: Option<usize>,
    crc32: Option<u32>,
}

/// Encodes the input file in a new output file.
/// # Example
/// ```
/// yenc::yencode_file("test1.bin", "test1.bin.yenc");
/// ```
/// # Errors
/// - when the output file already exists
///
pub fn yencode_file(input_filename: &str, output_filename: &str) -> Result<(), io::Error> {
    let mut input_file = OpenOptions::new().read(true).open(input_filename)?;
    let mut output_file = OpenOptions::new().write(true).create_new(true).open(output_filename)?;
    let mut checksum: crc32::Crc32 = Default::default();
    let mut buffer = [0u8; 65536];
    let mut col = 0;
    output_file.write(format!("=ybegin line={} size={} name={}\r\n",
                       LINE_SIZE,
                       input_file.metadata()?.len(),
                       input_filename)
            .as_bytes())?;
    loop {
        let bytes_read = input_file.read(&mut buffer)?;
        match bytes_read {
            0 => break,
            _ => {
                checksum.update_with_slice(&buffer[0..bytes_read]);
                output_file.write(yencode_buffer(&buffer[0..bytes_read], &mut col, LINE_SIZE).as_slice())?;
            }
        };
    }

    output_file.write(format!("\r\n=yend size={} crc32={:08X}\r\n",
                       checksum.num_bytes,
                       checksum.crc)
            .as_bytes())?;
    Ok(())
}

/// Encode the byte slice into a vector of yEncoded bytes, with the maximum of `line_length` characters per line.
pub fn yencode_buffer(input: &[u8], col: &mut u16, line_length: u16) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(input.len());
    for &b in input {
        let encoded = yencode_byte(b);
        output.extend_from_slice(&encoded);
        *col += encoded.len() as u16;
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


/// Decodes the input file in a new output file.
///
/// If ok, returns the path of the decoded file.
///
/// # Example
/// ```
/// yenc::ydecode_file("test2.bin.yenc", "test2.bin");
/// ```
/// # Errors
/// - when the output file already exists
///
pub fn ydecode_file(input_filename: &str, output_path: &str) -> Result<String, DecodeError> {
    let input_file = OpenOptions::new().read(true).open(input_filename)?;
    let mut output_pathbuf = PathBuf::new();
    output_pathbuf.push(output_path);
    let mut rdr = BufReader::new(input_file);
    let mut line_buf = Vec::<u8>::with_capacity(2 * LINE_SIZE as usize);
    let mut checksum: crc32::Crc32 = Default::default();
    let mut yenc_block_found = false;

    while !yenc_block_found {
        line_buf.clear();
        let length = rdr.read_until(LF, &mut line_buf)?;
        if length == 0 {
            break;
        }
        if line_buf.starts_with(b"=ybegin ") {
            yenc_block_found = true;
        }
    }

    if yenc_block_found {
        // parse header line and determine output filename
        let metadata = parse_header_line(&line_buf, 8)?;
        output_pathbuf.push(metadata.name.unwrap().to_string().trim());
        let mut output_file =
            OpenOptions::new().write(true).create_new(true).open(output_pathbuf.as_path())?;

        let mut footer_found = false;
        while !footer_found {
            line_buf.clear();
            let length = rdr.read_until(LF, &mut line_buf)?;
            if length == 0 {
                break;
            }
            if line_buf.starts_with(b"=yend ") {
                footer_found = true;
            } else {
                let decoded = ydecode_buffer(&line_buf[0..length]);
                checksum.update_with_slice(decoded.as_slice());
                output_file.write(decoded.as_slice())?;
            }
        }
        if let Some(expected_size) = metadata.size {
            if expected_size != checksum.num_bytes {
                return Err(DecodeError::IncompleteData(expected_size, checksum.num_bytes));
            }
        }
        if footer_found {
            let metadata = parse_header_line(&line_buf, 6)?;
            if let Some(expected_size) = metadata.size {
                if expected_size != checksum.num_bytes {
                    return Err(DecodeError::IncompleteData(expected_size, checksum.num_bytes));
                }
            }
            if let Some(expected_crc) = metadata.crc32 {
                if expected_crc != checksum.crc {
                    return Err(DecodeError::InvalidChecksum);
                }
            }
        }
    }
    Ok(output_pathbuf.to_str().unwrap().to_string())
}

/// Decode the yEncoded byte slice into a vector of bytes.
/// Carriage Return (CR) and Line Feed (LF) are ignored.
pub fn ydecode_buffer(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity((input.len() as f64 * 1.02) as usize);
    let mut i = 0;
    while i < input.len() {
        let mut byte = input[i];
        i += 1;
        match byte {
            CR | LF => continue,
            ESCAPE => {
                byte = input[i].overflowing_sub(64).0;
                i += 1;
            }
            _ => {}
        };
        output.push(byte.overflowing_sub(42).0);
    }
    output
}

fn parse_header_line(line_buf: &[u8], offset: usize) -> Result<MetaData, DecodeError> {
    #[derive(Debug)]
    enum State {
        Keyword,
        Value,
        End,
    };

    let mut metadata: MetaData = Default::default();
    let mut state = State::Keyword;
    let mut keyword = Vec::<u8>::with_capacity(4);
    let mut value = Vec::<u8>::with_capacity(96);

    let header_line = String::from_utf8_lossy(line_buf).to_string();
    for (i, &c) in line_buf[offset..].iter().enumerate() {
        let position = i + offset;
        match state {
            State::End => unreachable!(),
            State::Keyword => {
                match c {
                    b'a'...b'z' | b'0'...b'9' => keyword.push(c),
                    b'=' => {
                        if keyword.is_empty() ||
                           !(keyword.as_slice() == b"name" || keyword.as_slice() == b"line" ||
                             keyword.as_slice() == b"size" ||
                             keyword.as_slice() == b"crc32") {
                            return Err(DecodeError::InvalidHeader(header_line, position));
                        } else {
                            state = State::Value;
                        }
                    }
                    b'\r' | b'\n' => {}
                    _ => {
                        return Err(DecodeError::InvalidHeader(header_line, position));
                    }
                }
            }
            State::Value => {
                match keyword.as_slice() {
                    b"name" => {
                        match c {
                            b'\r' => {}
                            b'\n' => {
                                state = State::End;
                                metadata.name = Some(String::from_utf8_lossy(&value).to_string());
                            }
                            _ => value.push(c),
                        }
                    }
                    b"size" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            b' ' => {
                                metadata.size =
                                    Some(usize::from_str_radix(&String::from_utf8_lossy(&value),
                                                               10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader(header_line, position));
                            }
                        }
                    }
                    b"line" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            b' ' => {
                                metadata.line =
                                    Some(u16::from_str_radix(&String::from_utf8_lossy(&value), 10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader(header_line, position));
                            }
                        }
                    }
                    b"crc32" => {
                        match c {
                            b'0'...b'9' | b'A'...b'F' | b'a'...b'f' => value.push(c),
                            b' ' => {
                                state = State::Keyword;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            b'\n' => {
                                state = State::End;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            b'\r' => {}
                            _ => {
                                return Err(DecodeError::InvalidHeader(header_line, position));
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        };
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::{ESCAPE, TAB, LF, CR, SPACE, yencode_buffer, ydecode_buffer, yencode_byte};

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

    #[test]
    fn equality() {
        let b = (0..256).map(|c| c as u8).collect::<Vec<u8>>();
        let mut col = 0;
        assert_eq!(b,
                   ydecode_buffer(&yencode_buffer(&b, &mut col, 128)).as_slice());
    }
}
