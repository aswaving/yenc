const NUL: u8 = 0;
const TAB: u8 = b'\t';
const LF: u8 = b'\n';
const DOT: u8 = b'.';
const CR: u8 = b'\r';
const SPACE: u8 = b' ';
const ESCAPE: u8 = b'=';

#[inline]
fn yencode_byte(input_byte: u8) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(2);
    let mut output_byte = input_byte.overflowing_add(42).0;
    match output_byte {
        NUL | CR | LF | ESCAPE | TAB | DOT | SPACE => {
            output.push(ESCAPE);
            output_byte = output_byte.overflowing_add(64).0;
        }
        _ => {}
    }
    output.push(output_byte);
    output
}

pub fn ydecode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let mut byte = input[i];
        if byte == ESCAPE {
            i += 1;
            byte = input[i].overflowing_sub(64).0;
        };
        output.push(byte.overflowing_sub(42).0);
        i += 1;
    }
    output
}

pub fn yencode(input: &[u8]) -> Vec<u8> {
    input.iter().flat_map(|&b| yencode_byte(b)).collect::<Vec<u8>>()
}

#[cfg(test)]
mod tests {
    use super::{ESCAPE, TAB, LF, CR, SPACE, DOT, yencode, ydecode, yencode_byte};

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
    fn escape_dot() {
        assert_eq!(vec![ESCAPE, 0x6E], yencode_byte(DOT - 42));
    }

    #[test]
    fn escape_equal_sign() {
        assert_eq!(vec![ESCAPE, 0x7D], yencode_byte(ESCAPE - 42));
    }

    #[test]
    fn equality() {
        let b = (0..256).map(|c| c as u8).collect::<Vec<u8>>();
        assert_eq!(b, ydecode(&yencode(&b)).as_slice());
    }
}
