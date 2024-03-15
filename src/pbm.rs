//! [Portable BitMap Format](https://en.wikipedia.org/wiki/Netpbm#PBM_example) black and white image encoding and decoding.
//!
//! Unstable api.
pub type Input<'a> = Image<&'a [bool], 1>;
pub type Output = Image<Vec<bool>, 1>;
pub type Uninit = fimg::uninit::Image<bool, 1>;
use crate::encode::{encodeu32, P};
use atools::prelude::*;
use fimg::Image;

#[cfg(test)]
fn tdata() -> Vec<bool> {
    include_bytes!("../tdata/fimg.imgbuf")
        .iter()
        .map(|&x| x <= 128)
        .collect::<Vec<_>>()
}

/// Module for handling plain ascii (human readable) [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) (black and white) images.
pub mod plain {
    use crate::encode::encode_bool;

    use super::*;
    pub const MAGIC: u8 = 1;

    /// Encode an <code>[Image]<[bool], 1></code> into a [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) ASCII Image.
    pub fn encode<T: AsRef<[bool]>>(x: Image<T, 1>) -> String {
        let mut y = Vec::with_capacity(size(x.as_ref()));
        let n = unsafe { encode_into(x.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        unsafe { String::from_utf8_unchecked(y) }
    }

    crate::decode::dec_fn! {
        "Decode an ASCII [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) image into an <code>[Image]<[Box]<[bool]>, 1></code>"
    }

    #[doc = include_str!("decode_body_into.md")]
    pub fn decode_body_into(x: &[u8], mut into: Uninit) -> Result<Output> {
        let mut out = into.buf().as_mut_ptr() as *mut bool;
        let pixels = into.width() * into.height();
        for &b in x
            .iter()
            .filter(|&&x| matches!(x, b'0' | b'1'))
            .take(pixels as usize)
        {
            // SAFETY: iterator over `pixels` elements.
            unsafe { out.push(b == b'1') };
        }
        if unsafe { out.sub_ptr(into.buf().as_mut_ptr().cast()) < pixels as usize } {
            return Err(Error::MissingData);
        }
        // SAFETY: checked that the pixels have been initialized.
        Ok(unsafe { into.assume_init() })
    }

    /// Converts 0 to 255 and 1 to 0, for your u8 image experience.
    pub fn decode_body_into_u8(
        x: &[u8],
        mut into: fimg::uninit::Image<u8, 1>,
    ) -> Result<Image<Vec<u8>, 1>> {
        let mut out = into.buf().as_mut_ptr() as *mut u8;
        let pixels = into.width() * into.height();
        for &b in x
            .iter()
            .filter(|&&x| matches!(x, b'0' | b'1'))
            .take(pixels as usize)
        {
            // SAFETY: iterator over `pixels` elements.
            unsafe { out.push((b == b'0') as u8 * 0xff) };
        }
        if unsafe { out.sub_ptr(into.buf().as_mut_ptr().cast()) < pixels as usize } {
            return Err(Error::MissingData);
        }
        // SAFETY: checked that the pixels have been initialized.
        Ok(unsafe { into.assume_init() })
    }

    #[doc = include_str!("encode_into.md")]
    pub unsafe fn encode_into(x: Input, out: *mut u8) -> usize {
        let mut o = out;
        o.put(b'P'.join(MAGIC + b'0'));
        o.push(b' ');
        encodeu32(x.width(), &mut o);
        o.push(b' ');
        encodeu32(x.height(), &mut o);
        o.push(b'\n');
        for row in x.buffer().chunks_exact(x.width() as _) {
            for &on in row {
                o.push(encode_bool(on));
                // cosmetic
                o.push(b' ');
            }
            // cosmetic
            o.push(b'\n');
        }
        o.sub_ptr(out)
    }

    #[doc = include_str!("est.md")]
    pub fn size(x: Input) -> usize {
        2 // P1
            + 23 // \n4294967295 4294967295\n
            + x.height() as usize // \n
            + x.len() * 2 // ' 1'
    }

    #[test]
    fn test_encode() {
        assert_eq!(
            encode(Image::build(20, 15).buf(tdata())),
            include_str!("../tdata/fimgA.pbm")
        );
    }

    #[test]
    fn test_decode() {
        assert_eq!(
            &**decode(include_bytes!("../tdata/fimgA.pbm"))
                .unwrap()
                .buffer(),
            tdata()
        )
    }
}

/// Module for handling raw (packed binary) [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) (black and white) images.
pub mod raw {
    use super::*;
    pub const MAGIC: u8 = 4;
    /// Encode an <code>[Image]<[bool], 1></code> [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) Raw (packed binary) Image.
    pub fn encode<T: AsRef<[bool]>>(x: Image<T, 1>) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(x.as_ref()));
        let n = unsafe { encode_into(x.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    crate::decode::dec_fn! {
        "Decode a raw binary [PBM](https://en.wikipedia.org/wiki/Netpbm#PBM_example) image into an <code>[Image]<[Box]<[bool]>, 1></code>"
    }

    #[doc = include_str!("encode_into.md")]
    pub unsafe fn encode_into(x: Input, out: *mut u8) -> usize {
        let mut o = out;
        o.put(b'P'.join(MAGIC + b'0'));
        o.push(b' ');
        encodeu32(x.width(), &mut o);
        o.push(b' ');
        encodeu32(x.height(), &mut o);
        o.push(b'\n');
        x.buffer()
            .chunks_exact(x.width() as _)
            .flat_map(|x| x.chunks(8))
            .map(|chunk| {
                chunk
                    .iter()
                    .copied()
                    .chain(std::iter::repeat(false).take(8 - chunk.len()))
                    .zip(0u8..)
                    .fold(0, |acc, (x, i)| acc | (x as u8) << 7 - i)
            })
            .for_each(|x| o.push(x));

        o.sub_ptr(out)
    }

    #[doc = include_str!("decode_body_into.md")]
    pub fn decode_body_into(x: &[u8], mut into: Uninit) -> Result<Output> {
        let mut out = into.buf().as_mut_ptr() as *mut bool;
        let pixels = into.width() * into.height();
        let padding = into.width() % 8;
        for &x in x
            .iter()
            .copied()
            // expand the bits
            .flat_map(|b| atools::range::<8>().rev().map(|x| b & (1 << x) != 0))
            // TODO skip?
            .collect::<Vec<_>>()
            .chunks_exact((into.width() + padding) as _)
            .map(|x| &x[..into.width() as _])
            .take(pixels as _)
            .flatten()
        {
            // SAFETY: took `pixels` pixels.
            unsafe { out.push(x) };
        }
        if unsafe { out.sub_ptr(into.buf().as_mut_ptr().cast()) < pixels as usize } {
            return Err(Error::MissingData);
        }
        // SAFETY: checked that the pixels have been initialized.
        Ok(unsafe { into.assume_init() })
    }

    #[doc = include_str!("decode_body_into.md")]
    pub fn decode_body_into_u8(
        x: &[u8],
        mut into: fimg::uninit::Image<u8, 1>,
    ) -> Result<Image<Vec<u8>, 1>> {
        let mut out = into.buf().as_mut_ptr() as *mut u8;
        let pixels = into.width() * into.height();
        let padding = into.width() % 8;
        for x in x
            .iter()
            .copied()
            // expand the bits
            .flat_map(|b| atools::range::<8>().rev().map(|x| b & (1 << x) == 0))
            // TODO skip?
            .collect::<Vec<_>>()
            .chunks_exact((into.width() + padding) as _)
            .map(|x| &x[..into.width() as _])
            .take(pixels as _)
            .flatten()
            .map(|&x| x as u8 * 0xff)
        {
            // SAFETY: took `pixels` pixels.
            unsafe { out.push(x) };
        }
        if unsafe { out.sub_ptr(into.buf().as_mut_ptr().cast()) < pixels as usize } {
            return Err(Error::MissingData);
        }
        // SAFETY: checked that the pixels have been initialized.
        Ok(unsafe { into.assume_init() })
    }
    #[doc = include_str!("est.md")]
    pub fn size(x: Input) -> usize {
        2 // magic
            + 23 // w h
            + (x.len() / 8) // packed pixels
            + ((x.width() as usize % 8 != 0) as usize * x.height() as usize) // padding
    }

    #[test]
    fn test_decode() {
        assert_eq!(
            &**decode(include_bytes!("../tdata/fimgR.pbm"))
                .unwrap()
                .buffer(),
            tdata()
        )
    }

    #[test]
    fn test_encode() {
        assert_eq!(
            encode(Image::build(20, 15).buf(tdata())),
            include_bytes!("../tdata/fimgR.pbm")
        );
    }
}
