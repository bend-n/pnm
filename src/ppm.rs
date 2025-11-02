//! [Portable PixMap Format](https://en.wikipedia.org/wiki/Netpbm#PPM_example) RGB (no alpha) image encoding and decoding.
pub(crate) const CHANNELS: usize = 3;
pub type Input<'a> = Image<&'a [u8], 3>;
pub type Output = Image<Vec<u8>, 3>;
pub type Uninit = fimg::uninit::Image<u8, 3>;
use crate::encode::{encodeu32, P};
use atools::prelude::*;
use fimg::Image;

#[cfg(test)]
fn tdata() -> &'static [u8] {
    include_bytes!("../tdata/fimg-rainbow.imgbuf")
}

/// Module for handling plain ascii (human readable) [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) (Y) images.
pub mod plain {
    use crate::encode::encode_;

    use super::*;
    pub const MAGIC: u8 = 3;

    /// Encode an <code>[Image]<[u8], 3></code> into a [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) ASCII Image.
    pub fn encode<T: AsRef<[u8]>>(x: Image<T, 3>) -> String {
        let mut y = Vec::with_capacity(size(x.as_ref()));
        let n = unsafe { encode_into(x.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        unsafe { String::from_utf8_unchecked(y) }
    }

    crate::decode::dec_fn! {
        max "Decode an ASCII [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) image into an <code>[Image]<[Box]<[u8]>, 3></code>"
    }

    #[doc = include_str!("decode_body_into.md")]
    pub fn decode_body_into(x: &[u8], mut into: Uninit, max: u8) -> Result<Output> {
        let mut out = into.buf().as_mut_ptr() as *mut u8;
        let pixels = into.width() * into.height();
        for b in x
            .split(u8::is_ascii_whitespace)
            .filter(|x| !x.is_empty() && x.len() <= 3)
            .filter(|x| x.iter().all(u8::is_ascii_digit))
            .flat_map(|x| {
                x.iter()
                    .try_fold(0u8, |acc, &x| acc.checked_mul(10)?.checked_add(x - b'0'))
            })
            .map(|x| {
                if max == 255 {
                    x
                } else {
                    ((x as f32 / max as f32) * 255.) as u8
                }
            })
            .array_chunks::<3>()
            .take(pixels as usize)
        {
            // SAFETY: iterator over `pixels` elements.
            unsafe { out.put(b) };
        }
        if unsafe {
            out.offset_from_unsigned(into.buf().as_mut_ptr().cast()) < (pixels as usize * 3)
        } {
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
        o.put(*b" 255\n");
        for row in x.flatten().chunks_exact(x.width() as _) {
            for &on in row.iter().flatten() {
                o.put(encode_(on));
            }
            // cosmetic
            o.push(b'\n');
        }
        o.offset_from_unsigned(out)
    }

    #[doc = include_str!("est.md")]
    pub fn size(x: Input) -> usize {
        2 // P1
            + 23 // \n4294967295 4294967295\n
            + x.height() as usize // \n
            + x.len() * 4 // '255 '
    }

    #[test]
    fn test_encode() {
        assert_eq!(
            encode(Image::build(20, 15).buf(tdata())),
            include_str!("../tdata/fimg-rainbowA.ppm")
        );
    }

    #[test]
    fn test_decode() {
        assert_eq!(
            &**decode(include_bytes!("../tdata/fimg-rainbowA.ppm"))
                .unwrap()
                .buffer(),
            tdata()
        )
    }
}

/// Module for handling raw (binary) [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) (rgb) images.
pub mod raw {
    use super::*;
    pub const MAGIC: u8 = 6;
    /// Encode an <code>[Image]<[u8], 3></code> [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) Raw (binary) Image.
    pub fn encode<T: AsRef<[u8]>>(x: Image<T, 3>) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(x.as_ref()));
        let n = unsafe { encode_into(x.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    crate::decode::dec_fn! {
        "Decode a raw binary [PPM](https://en.wikipedia.org/wiki/Netpbm#PPM_example) image into an <code>[Image]<[Box]<[u8]>, 3></code>"
    }

    #[doc = include_str!("encode_into.md")]
    pub unsafe fn encode_into(x: Input, out: *mut u8) -> usize {
        let mut o = out;
        o.put(b'P'.join(MAGIC + b'0'));
        o.push(b' ');
        encodeu32(x.width(), &mut o);
        o.push(b' ');
        encodeu32(x.height(), &mut o);
        o.put(*b" 255\n");
        o.copy_from(x.buffer().as_ptr(), x.len());
        o.offset_from_unsigned(out) + x.len()
    }

    #[doc = include_str!("decode_body_into.md")]
    pub fn decode_body_into(x: &[u8], mut into: Uninit) -> Result<Output> {
        let mut out = into.buf().as_mut_ptr() as *mut u8;
        let pixels = into.width() * into.height();
        for b in x.iter().copied().array_chunks::<3>().take(pixels as _) {
            // SAFETY: took `pixels` pixels.
            unsafe { out.put(b) };
        }
        if unsafe {
            out.offset_from_unsigned(into.buf().as_mut_ptr().cast()) < (pixels as usize * 3)
        } {
            return Err(Error::MissingData);
        }
        // SAFETY: checked that the pixels have been initialized.
        Ok(unsafe { into.assume_init() })
    }

    #[doc = include_str!("est.md")]
    pub fn size(x: Input) -> usize {
        2 // magic
            + 23 // w h
            + x.len() // data
    }

    #[test]
    fn test_decode() {
        assert_eq!(
            &**decode(include_bytes!("../tdata/fimg-rainbowR.ppm"))
                .unwrap()
                .buffer(),
            tdata()
        )
    }

    #[test]
    fn test_encode() {
        assert_eq!(
            encode(Image::build(20, 15).buf(tdata())),
            include_bytes!("../tdata/fimg-rainbowR.ppm")
        );
    }
}
