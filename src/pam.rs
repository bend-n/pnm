//! [Portable Arbitrary Format](https://en.wikipedia.org/wiki/Netpbm#PAM_graphics_format) RGB (no alpha) image encoding and decoding.
pub type Input<'a> = Image<&'a [u8], 3>;
pub type Uninit = fimg::uninit::Image<u8, 3>;
use std::num::NonZeroU32;

use crate::decode::{read_til, Error, Read, Result};
use crate::encode::{encodeu32, P};
use atools::Join;
use fimg::{DynImage, Image};

pub const MAGIC: u8 = 7;

/// Encode this <code>[Image]<[u8], N></code> to a [PAM](https://en.wikipedia.org/wiki/Netpbm#PAM_graphics_format) Raw (binary) Image.
///
/// ```
/// # use pnm::pam;
/// # use fimg::Image;
/// let out = pam::encode(
///   Image::<_, 1>::build(20, 15).buf(&include_bytes!("../tdata/fimg-gray.imgbuf")[..])
/// );
/// ```
pub fn encode(x: impl PAM) -> Vec<u8> {
    x.encode()
}

/// Encode this <code>[Image]<[bool], N></code> to a [PAM](https://en.wikipedia.org/wiki/Netpbm#PAM_graphics_format) Raw (binary) Image.
pub fn encode_bitmap(x: impl PAMBit) -> Vec<u8> {
    x.encode_bitmap()
}

#[doc(hidden)]
pub trait PAM {
    fn encode(self) -> Vec<u8>;
    #[doc = include_str!("encode_into.md")]
    unsafe fn encode_into(x: Self, out: *mut u8) -> usize;
}

#[doc(hidden)]
pub trait PAMBit {
    fn encode_bitmap(self) -> Vec<u8>;
    #[doc = include_str!("encode_into.md")]
    unsafe fn encode_into(x: Self, out: *mut u8) -> usize;
}

impl<T: AsRef<[u8]>> PAM for Image<T, 1> {
    fn encode(self) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(self.bytes()));
        let n = unsafe { PAM::encode_into(self.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        encode_into((x.bytes(), (x.width(), x.height())), out, b"GRAYSCALE", 1)
    }
}

impl<T: AsRef<[bool]>> PAMBit for Image<T, 1> {
    fn encode_bitmap(self) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(self.as_ref().buffer()));
        let n = unsafe { PAMBit::encode_into(self.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        let b = x.buffer().as_ref();
        let b = std::slice::from_raw_parts(b.as_ptr() as *mut u8, b.len());
        encode_into((b, (x.width(), x.height())), out, b"BLACKANDWHITE", 1)
    }
}

impl<T: AsRef<[u8]>> PAM for Image<T, 2> {
    fn encode(self) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(self.bytes()));
        let n = unsafe { PAM::encode_into(self.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        encode_into(
            (x.bytes(), (x.width(), x.height())),
            out,
            b"GRAYSCALE_ALPHA",
            2,
        )
    }
}

impl<T: AsRef<[u8]>> PAM for Image<T, 3> {
    fn encode(self) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(self.bytes()));
        let n = unsafe { PAM::encode_into(self.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        encode_into((x.bytes(), (x.width(), x.height())), out, b"RGB", 3)
    }
}

impl<T: AsRef<[u8]>> PAM for Image<T, 4> {
    fn encode(self) -> Vec<u8> {
        let mut y = Vec::with_capacity(size(self.bytes()));
        let n = unsafe { PAM::encode_into(self.as_ref(), y.as_mut_ptr()) };
        unsafe { y.set_len(n) };
        y
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        encode_into((x.bytes(), (x.width(), x.height())), out, b"RGB_ALPHA", 2)
    }
}

impl<T: AsRef<[u8]>> PAM for DynImage<T> {
    fn encode(self) -> Vec<u8> {
        super::e!(self, |x| encode(x))
    }

    unsafe fn encode_into(x: Self, out: *mut u8) -> usize {
        super::e!(x, |x| PAM::encode_into(x, out))
    }
}

#[inline]
unsafe fn encode_into<const N: usize>(
    (buf, (w, h)): (&[u8], (u32, u32)),
    out: *mut u8,
    tupltype: &[u8; N],
    depth: u8,
) -> usize {
    let mut o = out;
    o.put(b'P'.join(MAGIC + b'0'));
    o.put(*b"\nWIDTH ");
    encodeu32(w, &mut o);
    o.put(*b"\nHEIGHT ");
    encodeu32(h, &mut o);
    o.put(*b"\nDEPTH ");
    o.push(depth + b'0');
    o.put(*b"\nMAXVAL 255\n");
    o.put(*b"TUPLTYPE ");
    o.put(*tupltype);
    o.put(*b"\nENDHDR\n");
    if tupltype[..] == *b"BLACKANDWHITE" {
        for &x in buf {
            o.push(x ^ 1)
        }
        o.offset_from_unsigned(out)
    } else {
        o.copy_from(buf.as_ptr(), buf.len());
        o.offset_from_unsigned(out) + buf.len()
    }
}

#[derive(Copy, Clone, Debug)]
/// Header for PAM images.
pub struct PAMHeader {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    /// Channel count
    pub depth: u8,
    /// Max value
    pub max: u8,
    /// Data type
    pub tupltype: Type,
}

/// Tupltype. See [pam wikipedia page](https://en.wikipedia.org/wiki/Netpbm#PAM_graphics_format) for more informaiton.
#[derive(Copy, Clone, Debug)]
pub enum Type {
    /// Black and white bitmap type, corresponding to `BLACKANDWHITE`
    Bit,
    /// Grayscale type, corresponds to `GRAYSCALE`
    Y,
    RGB,
    /// Black and white with alpha, `BLACKANDWHITE_ALPHA`
    BitA,
    /// Gray with alpha. `GRAYSCALE_ALPHA`
    YA,
    RGBA,
}

impl Type {
    const fn bytes(self) -> u8 {
        use Type::*;
        match self {
            Bit | Y => 1,
            BitA | YA => 2,
            RGB => 3,
            RGBA => 4,
        }
    }
}

/// Decode a PAM image into a [`DynImage`].
pub fn decode(x: impl AsRef<[u8]>) -> Result<DynImage<Vec<u8>>> {
    let mut x = x.as_ref();
    crate::decode::magic(&mut x);
    decode_wo_magic(x)
}

/// Decode a magicless PAM image.
pub fn decode_wo_magic(mut x: &[u8]) -> Result<DynImage<Vec<u8>>> {
    let header = decode_pam_header(&mut x)?;
    let mut alloc = Vec::with_capacity(
        header.tupltype.bytes() as usize
            * header.width.get() as usize
            * header.height.get() as usize,
    );
    let n = unsafe { decode_inner(x, alloc.as_mut_ptr(), header)? };
    unsafe { alloc.set_len(n) };
    Ok(match header.tupltype {
        Type::Bit => unsafe { DynImage::Y(Image::new(header.width, header.height, alloc)) },
        Type::Y => unsafe { DynImage::Y(Image::new(header.width, header.height, alloc)) },
        Type::BitA => unsafe { DynImage::Ya(Image::new(header.width, header.height, alloc)) },
        Type::YA => unsafe { DynImage::Ya(Image::new(header.width, header.height, alloc)) },
        Type::RGB => unsafe { DynImage::Rgb(Image::new(header.width, header.height, alloc)) },
        Type::RGBA => unsafe { DynImage::Rgba(Image::new(header.width, header.height, alloc)) },
    })
}

/// Decodes this pam image's body, placing it in the raw pointer.
/// # Safety
///
/// buffer must have [`size`] bytes of space.
pub unsafe fn decode_inner(x: &[u8], mut into: *mut u8, header: PAMHeader) -> Result<usize> {
    let n = header.tupltype.bytes() as usize
        * header.width.get() as usize
        * header.height.get() as usize;
    match header.tupltype {
        Type::Bit => x
            .iter()
            .map(|&x| x.saturating_mul(0xff))
            .take(n)
            .for_each(|x| into.push(x)),
        Type::BitA => x
            .iter()
            .array_chunks::<2>()
            .take(header.width.get() as usize * header.height.get() as usize)
            .map(|[&x, &a]| [x.saturating_mul(0xff), a])
            .for_each(|x| into.put(x)),
        Type::Y | Type::YA | Type::RGB | Type::RGBA => {
            if x.len() < n {
                return Err(Error::MissingData);
            }
            into.copy_from(x.as_ptr(), n);
        }
    }
    Ok(n)
}

/// expects no magic
pub fn decode_pam_header(x: &mut &[u8]) -> Result<PAMHeader> {
    macro_rules! test {
        ($for:literal else $e:ident) => {
            if x.rd().ok_or(Error::$e)? != *$for {
                return Err(Error::$e);
            };
        };
    }
    test![b"WIDTH " else MissingWidth];
    let width = NonZeroU32::new(read_til(x)?).ok_or(Error::ZeroWidth)?;
    test![b"HEIGHT " else MissingHeight];
    let height = NonZeroU32::new(read_til(x)?).ok_or(Error::ZeroHeight)?;
    width.checked_mul(height).ok_or(Error::TooLarge)?;
    test![b"DEPTH " else MissingDepth];
    let depth = read_til::<u8>(x)?;
    test![b"MAXVAL " else MissingMax];
    let max = read_til::<u8>(x)?;
    test![b"TUPLTYPE " else MissingTupltype];
    let end = x
        .iter()
        .position(|&x| x == b'\n')
        .ok_or(Error::MissingTupltype)?;
    let tupltype = match &x[..end] {
        b"BLACKANDWHITE" => Type::Bit,
        b"BLACKANDWHITE_ALPHA" => Type::BitA,
        b"GRAYSCALE" => Type::Y,
        b"GRAYSCALE_ALPHA" => Type::YA,
        b"RGB" => Type::RGB,
        b"RGB_ALPHA" => Type::RGBA,
        _ => return Err(Error::MissingTupltype),
    };
    *x = &x[end..];
    test![b"\nENDHDR\n" else MissingData];
    Ok(PAMHeader {
        width,
        height,
        depth,
        max,
        tupltype,
    })
}

#[doc = include_str!("est.md")]
pub const fn size<T>(x: &[T]) -> usize {
    92 + x.len()
}

#[test]
fn test_bit() {
    assert_eq!(
        PAMBit::encode_bitmap(
            Image::<_, 1>::build(20, 15).buf(
                include_bytes!("../tdata/fimg.imgbuf")
                    .iter()
                    .map(|&x| x <= 128)
                    .collect::<Vec<_>>(),
            ),
        ),
        include_bytes!("../tdata/fimg.pam")
    );

    assert_eq!(
        &**decode(include_bytes!("../tdata/fimg.pam"))
            .unwrap()
            .buffer(),
        include_bytes!("../tdata/fimg.imgbuf")
    );
}

#[test]
fn test_y() {
    assert_eq!(
        PAM::encode(
            Image::<_, 1>::build(20, 15).buf(&include_bytes!("../tdata/fimg-gray.imgbuf")[..])
        ),
        include_bytes!("../tdata/fimg-gray.pam")
    );
    assert_eq!(
        &**decode(include_bytes!("../tdata/fimg-gray.pam"))
            .unwrap()
            .buffer(),
        include_bytes!("../tdata/fimg-gray.imgbuf")
    );
}

#[test]
fn test_ya() {
    assert_eq!(
        PAM::encode(
            Image::<_, 2>::build(20, 15)
                .buf(&include_bytes!("../tdata/fimg-transparent.imgbuf")[..])
        ),
        include_bytes!("../tdata/fimg-transparent.pam")
    );
    assert_eq!(
        &**decode(include_bytes!("../tdata/fimg-transparent.pam"))
            .unwrap()
            .buffer(),
        include_bytes!("../tdata/fimg-transparent.imgbuf")
    );
}

#[test]
fn test_rgb() {
    assert_eq!(
        PAM::encode(
            Image::<_, 3>::build(20, 15).buf(&include_bytes!("../tdata/fimg-rainbow.imgbuf")[..])
        ),
        include_bytes!("../tdata/fimg-rainbow.pam")
    );
    assert_eq!(
        &**decode(include_bytes!("../tdata/fimg-rainbow.pam"))
            .unwrap()
            .buffer(),
        include_bytes!("../tdata/fimg-rainbow.imgbuf")
    );
}

#[test]
fn test_rgba() {
    assert_eq!(
        PAM::encode(
            Image::<_, 4>::build(20, 15)
                .buf(&include_bytes!("../tdata/fimg-rainbow-transparent.imgbuf")[..])
        ),
        include_bytes!("../tdata/fimg-rainbow-transparent.pam")
    );
    assert_eq!(
        &**decode(include_bytes!("../tdata/fimg-rainbow-transparent.pam"))
            .unwrap()
            .buffer(),
        include_bytes!("../tdata/fimg-rainbow-transparent.imgbuf")
    );
}
