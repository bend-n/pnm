//! decoding utilities
use std::num::NonZeroU32;

pub(crate) trait Read {
    fn rd<const N: usize>(&mut self) -> Option<[u8; N]>;
    fn by(&mut self) -> Option<u8> {
        Some(self.rd::<1>()?[0])
    }
}
impl<T: std::io::Read> Read for T {
    fn rd<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut buf = [0; N];
        self.read_exact(&mut buf).ok()?;
        Some(buf)
    }
}

pub(crate) trait Ten {
    fn ten() -> Self;
}
macro_rules! tenz {
    ($for:ty) => {
        impl Ten for $for {
            fn ten() -> $for {
                10
            }
        }
    };
}
tenz!(u8);
tenz!(u32);

pub(crate) trait Ck
where
    Self: Sized,
{
    fn checked_mul(self, rhs: Self) -> Option<Self>;
    fn checked_add(self, rhs: Self) -> Option<Self>;
}

macro_rules! cks {
    ($for:ty) => {
        impl Ck for $for {
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                <$for>::checked_mul(self, rhs)
            }
            fn checked_add(self, rhs: Self) -> Option<Self> {
                <$for>::checked_add(self, rhs)
            }
        }
    };
}
cks!(u8);
cks!(u32);

/// Result alias with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn read_til<
    T: Default
        + Ck
        + std::ops::Mul<T, Output = T>
        + std::ops::Add<T, Output = T>
        + From<u8>
        + Copy
        + Ten,
>(
    x: &mut &[u8],
) -> Result<T> {
    let mut n = T::default();
    while let Some(x) = x.by() {
        if x.is_ascii_whitespace() {
            return Ok(n);
        }
        if !x.is_ascii_digit() {
            return Err(Error::NotDigit(x as char));
        }
        n = n
            .checked_mul(T::ten())
            .ok_or(Error::Overflow)?
            .checked_add(T::from(x - b'0'))
            .ok_or(Error::Overflow)?;
    }
    Ok(n)
}

macro_rules! dec_fn {
    ($($f:ident)? $doc:literal) => {
        use crate::decode::{decode_header, Error, Result};

        #[doc = $doc]
        pub fn decode(x: impl AsRef<[u8]>) -> Result<Output> {
            let mut x = x.as_ref();
            let magic = crate::decode::magic(&mut x).ok_or(Error::MissingMagic)?;
            (magic == MAGIC)
                .then_some(())
                .ok_or(Error::WrongMagic {
                    got: magic,
                    should: MAGIC,
                })?;
            decode_wo_magic(x)
        }

        /// Decode without magic.
        pub fn decode_wo_magic(mut x: &[u8]) -> Result<Output> {
            let header = decode_header(&mut x, MAGIC)?;
            decode_body_into(x, Uninit::new(header.width, header.height), $(header.$f.unwrap())?)
        }
    };
}
pub(crate) use dec_fn;

/// Header for the older PNM formats. Not applicable to PAM.
#[derive(Debug, Clone, Copy)]
pub struct Header {
    /// Magic number.
    pub magic: u8,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    /// Maximum value of each byte.
    pub max: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
/// Errors that can occur on decoding.
pub enum Error {
    TooLarge,
    NotDigit(char),
    BadMagic(u8),
    WrongMagic { got: u8, should: u8 },
    MissingMagic,
    ZeroWidth,
    ZeroHeight,
    MissingWidth,
    MissingHeight,
    MissingData,
    MissingMax,
    MissingDepth,
    MissingTupltype,
    Overflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "image too big"),
            Self::NotDigit(x) => write!(f, "found {x} while decoding number"),
            Self::BadMagic(x) => write!(f, "{x} is not a valid magic number"),
            Self::WrongMagic { got, should } => {
                write!(f, "expected magic number {should} found {got}")
            }
            Self::MissingMagic => write!(f, "no magic number (likely not a pnm image)"),
            Self::ZeroWidth => write!(f, "zero width"),
            Self::ZeroHeight => write!(f, "zero height"),
            Self::MissingWidth => write!(f, "no width"),
            Self::MissingHeight => write!(f, "no height"),
            Self::MissingData => write!(f, "no data"),
            Self::MissingMax => write!(f, "no max value"),
            Self::MissingDepth => write!(f, "no depth"),
            Self::MissingTupltype => write!(f, "no tupltype"),
            Self::Overflow => write!(f, "overflow while parsing number"),
        }
    }
}
impl std::error::Error for Error {}

/// Decodes the magic number.
pub fn magic(x: &mut &[u8]) -> Option<u8> {
    (x.by()? == b'P').then_some(())?;
    let m = x.by().and_then(|x| x.checked_sub(b'0'));
    while x.first()?.is_ascii_whitespace() {
        x.by();
    }
    m
}

/// Get the older pnm formats header. Does not decode magic.
pub fn decode_header(x: &mut &[u8], magic: u8) -> Result<Header> {
    while x.first() == Some(&b'#') {
        while let Some(b) = x.by()
            && b != b'\n'
        {}
    }
    let width = NonZeroU32::new(read_til(x)?).ok_or(Error::ZeroWidth)?;
    let height = NonZeroU32::new(read_til(x)?).ok_or(Error::ZeroHeight)?;
    width.checked_mul(height).ok_or(Error::TooLarge)?;
    let max = if magic != 4 && magic != 1 {
        Some(read_til(x)?)
    } else {
        None
    };

    if magic != 4 {
        while x.first().ok_or(Error::MissingData)?.is_ascii_whitespace() {
            x.by();
        }
    }
    Ok(Header {
        magic,
        width,
        height,
        max,
    })
}
