use std::fs::OpenOptions;
use std::io::{Write, BufReader, BufRead, Seek, SeekFrom};
use std::path::PathBuf;

use errors::DecodeError;
use crc32;
use constants::{NUL, CR, LF, ESCAPE, SPACE, DEFAULT_LINE_SIZE};

#[derive(Default,Debug)]
struct MetaData {
    name: Option<String>,
    line_length: Option<u16>,
    size: Option<usize>,
    crc32: Option<u32>,
    part: Option<u32>,
    begin: Option<usize>,
    end: Option<usize>,
}


/// Decodes the input file in a new output file.
///
/// If ok, returns the path of the decoded file.
///
/// # Example
/// ```rust,no_run
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
    let mut line_buf = Vec::<u8>::with_capacity(2 * DEFAULT_LINE_SIZE as usize);
    let mut checksum = crc32::Crc32::new();
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
        let mut metadata = parse_header_line(&line_buf, 8)?;
        output_pathbuf.push(metadata.name.unwrap().to_string().trim());
        let mut output_file =
            OpenOptions::new().create(true).write(true).open(output_pathbuf.as_path())?;

        let mut footer_found = false;
        while !footer_found {
            line_buf.clear();
            let length = rdr.read_until(LF, &mut line_buf)?;
            if length == 0 {
                break;
            }
            if line_buf.starts_with(b"=ypart ") {
                let part_metadata = parse_header_line(&line_buf, 7)?;
                metadata.part = part_metadata.part;
                metadata.begin = part_metadata.begin;
                metadata.end = part_metadata.end;
                if let Some(begin) = metadata.begin {
                    output_file.seek(SeekFrom::Start((begin - 1) as u64))?;
                }
            } else if line_buf.starts_with(b"=yend ") {
                footer_found = true;
            } else {
                let decoded = ydecode_buffer(&line_buf[0..length])?;
                checksum.update_with_slice(decoded.as_slice());
                output_file.write(decoded.as_slice())?;
            }
        }
        if footer_found {
            let metadata = parse_header_line(&line_buf, 6)?;
            println!("{:?}", metadata);
            if let Some(expected_size) = metadata.size {
                if expected_size != checksum.num_bytes {
                    return Err(DecodeError::IncompleteData {
                        expected_size: expected_size,
                        actual_size: checksum.num_bytes,
                    });
                }
            }
            if let Some(expected_crc) = metadata.crc32 {
                if expected_crc != checksum.crc {
                    return Err(DecodeError::InvalidChecksum);
                }
            }
        } else {
            if let Some(expected_size) = metadata.size {
                if expected_size != checksum.num_bytes {
                    return Err(DecodeError::IncompleteData {
                        expected_size: expected_size,
                        actual_size: checksum.num_bytes,
                    });
                }
            }
        }
    }
    Ok(output_pathbuf.to_str().unwrap().to_string())
}

/// Decode the yEncoded byte slice into a vector of bytes.
/// Carriage Return (CR) and Line Feed (LF) are ignored.
pub fn ydecode_buffer(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    // TODO remove heap allocation
    let mut output = Vec::<u8>::with_capacity((input.len() as f64 * 1.02) as usize);
    let mut iter = input.iter();
    while let Some(byte) = iter.next() {
        let mut byte = *byte;
        match byte {
            NUL | CR | LF => {
                // for now, just continue
                continue;
            }
            ESCAPE => {
                match iter.next() {
                    Some(b) => {
                        byte = b.overflowing_sub(64).0;
                    }
                    None => {
                        // for now, just continue, the only
                        continue;
                    }
                }
            }
            _ => {}
        }
        output.push(byte.overflowing_sub(42).0);
    }
    Ok(output)
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
    let mut keyword = Vec::<u8>::with_capacity(6);
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
                        if keyword.is_empty() || !is_known_keyword(&keyword) {
                            return Err(DecodeError::InvalidHeader {
                                line: header_line,
                                position: position,
                            });
                        } else {
                            state = State::Value;
                        }
                    }
                    CR | LF => {}
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position: position,
                        });
                    }
                }
            }
            State::Value => {
                match keyword.as_slice() {
                    b"name" => {
                        match c {
                            CR => {}
                            LF => {
                                state = State::End;
                                metadata.name = Some(String::from_utf8_lossy(&value).to_string());
                            }
                            _ => value.push(c),
                        }
                    }
                    b"size" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            SPACE => {
                                metadata.size =
                                    Some(usize::from_str_radix(&String::from_utf8_lossy(&value),
                                                               10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"begin" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            SPACE => {
                                metadata.begin =
                                    Some(usize::from_str_radix(&String::from_utf8_lossy(&value),
                                                               10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"end" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            SPACE | LF | CR => {
                                metadata.end =
                                    Some(usize::from_str_radix(&String::from_utf8_lossy(&value),
                                                               10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"line" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            SPACE => {
                                metadata.line_length =
                                    Some(u16::from_str_radix(&String::from_utf8_lossy(&value), 10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"part" => {
                        match c {
                            b'0'...b'9' => value.push(c),
                            SPACE => {
                                metadata.part =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 10)
                                        .unwrap());
                                state = State::Keyword;
                                keyword.clear();
                                value.clear();
                            }
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"crc32" => {
                        match c {
                            b'0'...b'9' | b'A'...b'F' | b'a'...b'f' => value.push(c),
                            SPACE => {
                                state = State::Keyword;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            LF => {
                                state = State::End;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            CR => {}
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
                            }
                        }
                    }
                    b"pcrc32" => {
                        match c {
                            b'0'...b'9' | b'A'...b'F' | b'a'...b'f' => value.push(c),
                            SPACE => {
                                state = State::Keyword;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            LF => {
                                state = State::End;
                                metadata.crc32 =
                                    Some(u32::from_str_radix(&String::from_utf8_lossy(&value), 16)
                                        .unwrap());
                                keyword.clear();
                                value.clear();
                            }
                            CR => {}
                            _ => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position: position,
                                });
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

fn is_known_keyword(keyword_slice: &[u8]) -> bool {
    keyword_slice == b"name" || keyword_slice == b"line" || keyword_slice == b"size" ||
    keyword_slice == b"part" || keyword_slice == b"begin" ||
    keyword_slice == b"end" || keyword_slice == b"pcrc32" ||
    keyword_slice == b"crc32"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_invalid() {
        assert!(ydecode_buffer(&[b'=']).unwrap().is_empty());
    }

    #[test]
    fn decode_valid_ff() {
        assert_eq!(&vec![0xff - 0x2A], &ydecode_buffer(&[0xff]).unwrap());
    }

    #[test]
    fn decode_valid_01() {
        assert_eq!(&vec![0xff - 0x28], &ydecode_buffer(&[0x01]).unwrap());
    }

    #[test]
    fn decode_valid_esc_ff() {
        assert_eq!(&vec![0xff - 0x40 - 0x2A],
                   &ydecode_buffer(&[b'=', 0xff]).unwrap());
    }

    #[test]
    fn decode_valid_esc_01() {
        assert_eq!(&vec![0xff - 0x40 - 0x2A + 2],
                   &ydecode_buffer(&[b'=', 0x01]).unwrap());
    }
}
