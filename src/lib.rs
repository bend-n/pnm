//! crate for decoding/encoding the portable anymap format.
//!
//! ### a quick guide to the various functions for everyday use
//!
//! - [`decode()`]: your go-to for all PNM image decoding.
//! If you have a specific format you need to support, use its module directly.
//! Note that this function reads both plain and raw formats.
//! - [`encode()`]: this function is a little tricky.
//! It supports the "older" PNM formats, and, due to their age they do not support the alpha channels existence.
//! If possible, use [`pam::encode`] instead.
//! - [`encode_plain`]: The `PAM` format doesnt actually support read-age-by-humans, so this is still useful at times.
//! Outputs data in decimal digits.
//!
//! ### functions in action
//!
//! ```
//! let data = include_bytes!("../tdata/fimg-rainbowR.ppm");
//! let out = pnm::decode(data).unwrap();
//!
//! assert_eq!(pnm::encode(out), data);
//! ```
#![allow(incomplete_features)]
#![feature(ptr_sub_ptr, let_chains, iter_array_chunks)]
#![warn(
    clippy::missing_const_for_fn,
    clippy::suboptimal_flops,
    clippy::dbg_macro,
    clippy::use_self
)]

use fimg::{uninit, DynImage, Image};
pub mod decode;
pub(crate) mod encode;
pub mod pam;
pub mod pbm;
pub mod pgm;
pub mod ppm;
pub use pam::PAM;

/// Decode any [`pgm`], [`ppm`], [`pbm`], [`pam`] image.
pub fn decode(x: &impl AsRef<[u8]>) -> decode::Result<DynImage<Vec<u8>>> {
    let mut x = x.as_ref();
    let magic = decode::magic(&mut x).ok_or(decode::Error::MissingMagic)?;
    match magic {
        pbm::raw::MAGIC => {
            let header = decode::decode_header(&mut x, pbm::raw::MAGIC)?;
            Ok(DynImage::Y(pbm::raw::decode_body_into_u8(
                x,
                uninit::Image::new(header.width, header.height),
            )?))
        }
        pbm::plain::MAGIC => {
            let header = decode::decode_header(&mut x, pbm::plain::MAGIC)?;
            Ok(DynImage::Y(pbm::plain::decode_body_into_u8(
                x,
                uninit::Image::new(header.width, header.height),
            )?))
        }
        pgm::raw::MAGIC => Ok(DynImage::Y(pgm::raw::decode_wo_magic(x)?)),
        pgm::plain::MAGIC => Ok(DynImage::Y(pgm::plain::decode_wo_magic(x)?)),
        ppm::raw::MAGIC => Ok(DynImage::Rgb(ppm::raw::decode_wo_magic(x)?)),
        ppm::plain::MAGIC => Ok(DynImage::Rgb(ppm::plain::decode_wo_magic(x)?)),
        pam::MAGIC => pam::decode_wo_magic(x),
        _ => Err(decode::Error::BadMagic(magic)),
    }
}

/// Encodes an image to one of the [`pgm`] or [`ppm`] portable anymap formats.
///
/// Please note that this will not produce a [`pam`], use [`PAM`] for that.
pub fn encode(x: impl Encode) -> Vec<u8> {
    x.encode()
}

/// Encodes an image to one of the [`pgm`] or [`ppm`] portable anymap formats.
///
/// Please note that this will not produce a [`pam`], use [`PAM`] for that.
/// ASCII EDITION!
pub fn encode_plain(x: impl Encode) -> String {
    x.encode_plain()
}

#[doc(hidden)]
pub trait Encode {
    fn encode(self) -> Vec<u8>;
    fn encode_plain(self) -> String;
}

macro_rules! x {
    ($mod:ident) => {
        impl<T: AsRef<[u8]>> Encode for Image<T, { $mod::CHANNELS }> {
            fn encode(self) -> Vec<u8> {
                $mod::raw::encode(self)
            }
            fn encode_plain(self) -> String {
                $mod::plain::encode(self)
            }
        }
    };
    (t $mod:ident, $n:literal) => {
        impl<T: AsRef<[u8]>> Encode for Image<T, $n> {
            fn encode(self) -> Vec<u8> {
                $mod::raw::encode(<Image<Box<[u8]>, { $mod::CHANNELS }>>::from(self.as_ref()))
            }
            fn encode_plain(self) -> String {
                $mod::plain::encode(<Image<Box<[u8]>, { $mod::CHANNELS }>>::from(self.as_ref()))
            }
        }
    };
}
x![pgm];
x![t pgm, 2];
x![ppm];
x![t ppm, 4];

macro_rules! e {
    ($dyn:expr, |$image: pat_param| $do:expr) => {
        match $dyn {
            DynImage::Y($image) => $do,
            DynImage::Ya($image) => $do,
            DynImage::Rgb($image) => $do,
            DynImage::Rgba($image) => $do,
        }
    };
}
use e;
impl<T: AsRef<[u8]>> Encode for DynImage<T> {
    fn encode(self) -> Vec<u8> {
        e!(self, |x| encode(x))
    }
    fn encode_plain(self) -> String {
        e!(self, |x| encode_plain(x))
    }
}
