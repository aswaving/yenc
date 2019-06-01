use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

use super::constants::{CR, DEFAULT_LINE_SIZE, DOT, ESCAPE, LF, NUL, SPACE};
use super::crc32;
use super::errors::DecodeError;

/// Options for decoding.
/// The entry point for decoding from a file or (TCP) stream to an output directory.
#[derive(Debug)]
pub struct DecodeOptions<P> {
    output_dir: P,
}

#[derive(Default, Debug)]
struct MetaData {
    name: Option<String>,
    line_length: Option<u16>,
    size: Option<usize>,
    crc32: Option<u32>,
    pcrc32: Option<u32>,
    part: Option<u32>,
    total: Option<u32>,
    begin: Option<usize>,
    end: Option<usize>,
}

impl<P> DecodeOptions<P>
where
    P: AsRef<Path>,
{
    /// Construct new DecodeOptions using the specified path as output directory.
    /// The output directory is
    pub fn new(output_dir: P) -> DecodeOptions<P> {
        DecodeOptions { output_dir }
    }
    /// Decodes the input file in a new output file.
    ///
    /// If ok, returns the path of the decoded file.
    ///
    /// # Example
    /// ```rust,no_run
    /// let decode_options = yenc::DecodeOptions::new("/tmp/decoded");
    /// decode_options.decode_file("test2.bin.yenc");
    /// ```
    /// # Errors
    /// - when the output file already exists
    /// - when I/O error occurs
    ///
    pub fn decode_file(&self, input_filename: &str) -> Result<String, DecodeError> {
        let mut input_file = OpenOptions::new().read(true).open(input_filename)?;
        self.decode_stream(&mut input_file)
    }

    /// Decodes the data from a stream to the specified directory.
    ///
    /// Writes the output to a file with the filename from the header line, and places it in the
    /// output path. The path of the output file is returned as String.
    pub fn decode_stream<R>(&self, read_stream: R) -> Result<String, DecodeError>
    where
        R: Read,
    {
        let mut rdr = BufReader::new(read_stream);
        let mut output_pathbuf = self.output_dir.as_ref().to_path_buf();

        let mut checksum = crc32::Crc32::new();
        let mut yenc_block_found = false;
        let mut metadata: MetaData = Default::default();

        while !yenc_block_found {
            let mut line_buf = Vec::<u8>::with_capacity(2 * DEFAULT_LINE_SIZE as usize);
            let length = rdr.read_until(LF, &mut line_buf)?;
            if length == 0 {
                break;
            }
            if line_buf.starts_with(b"=ybegin ") {
                yenc_block_found = true;
                // parse header line and determine output filename
                metadata = parse_header_line(&line_buf)?;
                if let Some(ref name) = metadata.name {
                    output_pathbuf.push(name.trim());
                }
            }
        }

        if yenc_block_found {
            let mut output_file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(output_pathbuf.as_path())?;

            let mut footer_found = false;
            while !footer_found {
                let mut line_buf = Vec::<u8>::with_capacity(2 * DEFAULT_LINE_SIZE as usize);
                let length = rdr.read_until(LF, &mut line_buf)?;
                if length == 0 {
                    break;
                }
                if line_buf.starts_with(b"=ypart ") {
                    let part_metadata = parse_header_line(&line_buf)?;
                    metadata.begin = part_metadata.begin;
                    metadata.end = part_metadata.end;
                    if let Some(begin) = metadata.begin {
                        output_file.seek(SeekFrom::Start((begin - 1) as u64))?;
                    }
                } else if line_buf.starts_with(b"=yend ") {
                    footer_found = true;
                    let mm = parse_header_line(&line_buf)?;
                    metadata.size = mm.size;
                    metadata.crc32 = mm.crc32;
                    metadata.pcrc32 = mm.pcrc32;
                } else {
                    let decoded = decode_buffer(&line_buf[0..length])?;
                    checksum.update_with_slice(decoded.as_slice());
                    output_file.write_all(decoded.as_slice())?;
                }
            }
            if footer_found {
                if let Some(expected_part_crc) = metadata.pcrc32 {
                    if expected_part_crc != checksum.crc {
                        return Err(DecodeError::InvalidChecksum);
                    }
                } else if let Some(expected_crc) = metadata.crc32 {
                    if expected_crc != checksum.crc {
                        return Err(DecodeError::InvalidChecksum);
                    }
                }
            }
            if let Some(expected_size) = metadata.size {
                if expected_size != checksum.num_bytes {
                    return Err(DecodeError::IncompleteData {
                        expected_size,
                        actual_size: checksum.num_bytes,
                    });
                }
            }
        }
        Ok(output_pathbuf.to_str().unwrap().to_string())
    }
}

/// Decode the encoded byte slice into a vector of bytes.
///
/// Carriage Return (CR) and Line Feed (LF) are ignored.
pub fn decode_buffer(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut output = Vec::<u8>::with_capacity((input.len() as f64 * 1.02) as usize);
    let mut iter = input.iter().enumerate();
    while let Some((col, byte)) = iter.next() {
        let mut byte = *byte;
        match byte {
            NUL | CR | LF => {
                // for now, just continue
                continue;
            }
            DOT => {
                if col == 0 {
                    match iter.next() {
                        Some((_, &DOT)) => {}
                        Some((_, b)) => {
                            output.push(byte.overflowing_sub(42).0);
                            byte = *b;
                        }
                        None => {}
                    }
                }
            }
            ESCAPE => {
                match iter.next() {
                    Some((_, b)) => {
                        byte = b.overflowing_sub(64).0;
                    }
                    None => {
                        // for now, just continue
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

fn parse_header_line(line_buf: &[u8]) -> Result<MetaData, DecodeError> {
    #[derive(Debug)]
    enum State {
        Keyword,
        Value,
        End,
    };

    let header_line = String::from_utf8_lossy(line_buf).to_string();
    if !(header_line.starts_with("=ybegin ")
        || header_line.starts_with("=yend ")
        || header_line.starts_with("=ypart "))
    {
        return Err(DecodeError::InvalidHeader {
            line: header_line,
            position: 0,
        });
    }

    let offset = match line_buf.iter().position(|&c| c == b' ') {
        Some(pos) => pos + 1,
        None => {
            return Err(DecodeError::InvalidHeader {
                line: header_line,
                position: 9,
            })
        }
    };

    let mut metadata: MetaData = Default::default();
    let mut state = State::Keyword;

    let mut keyword: &[u8] = &[];
    let mut keyword_start_idx: Option<usize> = None;
    let mut value: &[u8] = &[];
    let mut value_start_idx: Option<usize> = None;

    for (i, &c) in line_buf[offset..].iter().enumerate() {
        let position = i + offset;
        match state {
            State::End => unreachable!(),
            State::Keyword => match c {
                b'a'..=b'z' | b'0'..=b'9' => {
                    if keyword_start_idx.is_none() {
                        keyword_start_idx = Some(position);
                    }
                    keyword = match keyword_start_idx {
                        Some(idx) => &line_buf[idx..=position],
                        None => {
                            return Err(DecodeError::InvalidHeader {
                                line: header_line,
                                position,
                            })
                        }
                    };
                }
                b'=' => {
                    if keyword.is_empty() || !is_known_keyword(keyword) {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    } else {
                        state = State::Value;
                    }
                }
                CR | LF => {}
                _ => {
                    return Err(DecodeError::InvalidHeader {
                        line: header_line,
                        position,
                    });
                }
            },
            State::Value => match keyword {
                b"name" => match c {
                    CR => {}
                    LF => {
                        state = State::End;
                        metadata.name = Some(String::from_utf8_lossy(value).to_string());
                    }
                    _ => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                },
                b"size" => match c {
                    b'0'..=b'9' => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                    SPACE => {
                        metadata.size =
                            match usize::from_str_radix(&String::from_utf8_lossy(value), 10) {
                                Ok(size) => Some(size),
                                Err(_) => {
                                    return Err(DecodeError::InvalidHeader {
                                        line: header_line,
                                        position,
                                    })
                                }
                            };
                        state = State::Keyword;
                        keyword_start_idx = None;
                        value_start_idx = None;
                    }
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    }
                },
                b"begin" | b"end" => match c {
                    b'0'..=b'9' => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                    SPACE | LF | CR => {
                        let nr = match usize::from_str_radix(&String::from_utf8_lossy(value), 10) {
                            Ok(size) => Some(size),
                            Err(_) => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };

                        if keyword == b"begin" {
                            metadata.begin = nr;
                        } else {
                            metadata.end = nr;
                        }
                        state = State::Keyword;
                        keyword_start_idx = None;
                        value_start_idx = None;
                    }
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    }
                },
                b"line" => match c {
                    b'0'..=b'9' => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                    SPACE => {
                        metadata.line_length =
                            match u16::from_str_radix(&String::from_utf8_lossy(value), 10) {
                                Ok(size) => Some(size),
                                Err(_) => {
                                    return Err(DecodeError::InvalidHeader {
                                        line: header_line,
                                        position,
                                    })
                                }
                            };
                        state = State::Keyword;
                        keyword_start_idx = None;
                        value_start_idx = None;
                    }
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    }
                },
                b"part" | b"total" => match c {
                    b'0'..=b'9' => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                    SPACE => {
                        let number = match u32::from_str_radix(&String::from_utf8_lossy(value), 10)
                        {
                            Ok(size) => Some(size),
                            Err(_) => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                        if keyword == b"part" {
                            metadata.part = number;
                        } else {
                            metadata.total = number;
                        }
                        state = State::Keyword;
                        keyword_start_idx = None;
                        value_start_idx = None;
                    }
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    }
                },
                b"crc32" | b"pcrc32" => match c {
                    b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                        if value_start_idx.is_none() {
                            value_start_idx = Some(position);
                        }
                        value = match value_start_idx {
                            Some(idx) => &line_buf[idx..=position],
                            None => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                    }
                    SPACE | LF => {
                        state = if c == SPACE {
                            State::Keyword
                        } else {
                            State::End
                        };
                        let crc = match u32::from_str_radix(&String::from_utf8_lossy(value), 16) {
                            Ok(size) => Some(size),
                            Err(_) => {
                                return Err(DecodeError::InvalidHeader {
                                    line: header_line,
                                    position,
                                })
                            }
                        };
                        if keyword == b"crc32" {
                            metadata.crc32 = crc;
                        } else {
                            metadata.pcrc32 = crc;
                        }
                        keyword_start_idx = None;
                        value_start_idx = None;
                    }
                    CR => {}
                    _ => {
                        return Err(DecodeError::InvalidHeader {
                            line: header_line,
                            position,
                        });
                    }
                },
                _ => unreachable!(),
            },
        };
    }
    Ok(metadata)
}

fn is_known_keyword(keyword_slice: &[u8]) -> bool {
    match keyword_slice {
        b"begin" | b"crc32" | b"end" | b"line" | b"name" | b"part" | b"pcrc32" | b"size"
        | b"total" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_buffer, parse_header_line};

    #[test]
    fn parse_valid_footer_end_nl() {
        let parse_result = parse_header_line(b"=yend size=26624 part=1 pcrc32=ae052b48\n");
        assert!(parse_result.is_ok());
        let metadata = parse_result.unwrap();
        assert_eq!(Some(1), metadata.part);
        assert_eq!(Some(26624), metadata.size);
        assert_eq!(Some(0xae052b48), metadata.pcrc32);
        assert!(metadata.crc32.is_none());
    }

    #[test]
    fn parse_valid_footer_end_crlf() {
        let parse_result =
            parse_header_line(b"=yend size=26624 part=1 pcrc32=ae052b48 crc32=ff00ff00\r\n");
        assert!(parse_result.is_ok());
        let metadata = parse_result.unwrap();
        assert_eq!(Some(1), metadata.part);
        assert_eq!(Some(26624), metadata.size);
        assert_eq!(Some(0xae052b48), metadata.pcrc32);
        assert_eq!(Some(0xff00ff00), metadata.crc32);
    }

    #[test]
    fn parse_valid_footer_end_space() {
        let parse_result = parse_header_line(b"=yend size=26624 part=1 pcrc32=ae052b48 \n");
        assert!(parse_result.is_ok());
        let metadata = parse_result.unwrap();
        assert_eq!(Some(1), metadata.part);
        assert_eq!(Some(26624), metadata.size);
        assert_eq!(Some(0xae052b48), metadata.pcrc32);
    }

    #[test]
    fn parse_valid_header_begin() {
        let parse_result = parse_header_line(
            b"=ybegin part=1 line=128 size=189463 name=CatOnKeyboardInSpace001.jpg\n",
        );
        assert!(parse_result.is_ok());
        let metadata = parse_result.unwrap();
        assert_eq!(metadata.part, Some(1));
        assert_eq!(metadata.size, Some(189463));
        assert_eq!(metadata.line_length, Some(128));
        assert_eq!(
            Some("CatOnKeyboardInSpace001.jpg".to_string()),
            metadata.name,
        );
    }

    #[test]
    fn parse_valid_header_part() {
        let parse_result = parse_header_line(b"=ypart begin=1 end=189463\n");
        assert!(parse_result.is_ok());
        let metadata = parse_result.unwrap();
        assert_eq!(metadata.begin, Some(1));
        assert_eq!(metadata.end, Some(189463));
    }

    #[test]
    fn invalid_header_tag() {
        let parse_result = parse_header_line(b"=yparts begin=1 end=189463\n");
        assert!(parse_result.is_err());
    }

    #[test]
    fn invalid_header_unknown_keyword() {
        let parse_result = parse_header_line(b"=ybegin parts=1 total=4 name=party.jpg\r\n");
        assert!(parse_result.is_err());
    }

    #[test]
    fn invalid_header_invalid_begin() {
        let parse_result = parse_header_line(b"=ypart begin=a end=189463\n");
        assert!(parse_result.is_err());
    }

    #[test]
    fn invalid_header_invalid_end() {
        let parse_result = parse_header_line(b"=ypart begin=1 end=18_9463\n");
        assert!(parse_result.is_err());
    }

    #[test]
    fn invalid_header_empty_keyword() {
        let parse_result = parse_header_line(b"=ypart =1 end=189463\n");
        assert!(parse_result.is_err());
    }

    #[test]
    fn decode_invalid() {
        assert!(decode_buffer(&[b'=']).unwrap().is_empty());
    }

    #[test]
    fn decode_valid_ff() {
        assert_eq!(&vec![0xff - 0x2A], &decode_buffer(&[0xff]).unwrap());
    }

    #[test]
    fn decode_valid_01() {
        assert_eq!(&vec![0xff - 0x28], &decode_buffer(&[0x01]).unwrap());
    }

    #[test]
    fn decode_valid_esc_ff() {
        assert_eq!(
            &vec![0xff - 0x40 - 0x2A],
            &decode_buffer(&[b'=', 0xff]).unwrap()
        );
    }

    #[test]
    fn decode_valid_esc_01() {
        assert_eq!(
            &vec![0xff - 0x40 - 0x2A + 2],
            &decode_buffer(&[b'=', 0x01]).unwrap()
        );
    }

    #[test]
    fn decode_valid_prepended_dots() {
        assert_eq!(&vec![b'.' - 0x2A], &decode_buffer(b"..").unwrap());
    }

    #[test]
    fn decode_valid_prepended_single_dot() {
        assert_eq!(
            &vec![b'.' - 0x2A, 0xff - 0x2A],
            &decode_buffer(&[b'.', 0xff]).unwrap()
        );
    }
}
