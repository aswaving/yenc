#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    extern crate yenc;
    use test::Bencher;

    #[bench]
    fn encode_buffer(b: &mut Bencher) {
        let mut col = 0;
#[cfg_attr(feature = "cargo-clippy", allow(unreadable_literal))]
        let buf = (0..32768).map(|c| (c % 256) as u8).collect::<Vec<u8>>();
        b.iter(|| yenc::yencode_buffer(&buf, &mut col, 128));
    }
}
