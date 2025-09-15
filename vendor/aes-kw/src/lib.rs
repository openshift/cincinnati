#![no_std]
#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;

pub use error::{Error, Result};

use aes::cipher::{
    generic_array::GenericArray,
    typenum::{Unsigned, U16, U24, U32},
    Block, BlockBackend, BlockCipher, BlockClosure, BlockDecrypt, BlockEncrypt, BlockSizeUser,
    KeyInit,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Size of an AES "semiblock" in bytes.
///
/// From NIST SP 800-38F § 4.1:
///
/// > semiblock: given a block cipher, a bit string whose length is half of the
/// > block size.
pub const SEMIBLOCK_SIZE: usize = 8;

/// Maximum length of the AES-KWP input data (2^32 bytes).
pub const KWP_MAX_LEN: usize = u32::MAX as usize;

/// Size of an AES-KW and AES-KWP initialization vector in bytes.
pub const IV_LEN: usize = SEMIBLOCK_SIZE;

/// Default Initial Value for AES-KW as defined in RFC3394 § 2.2.3.1.
///
/// <https://datatracker.ietf.org/doc/html/rfc3394#section-2.2.3.1>
///
/// ```text
/// The default initial value (IV) is defined to be the hexadecimal
/// constant:
///
///     A[0] = IV = A6A6A6A6A6A6A6A6
///
/// The use of a constant as the IV supports a strong integrity check on
/// the key data during the period that it is wrapped.  If unwrapping
/// produces A[0] = A6A6A6A6A6A6A6A6, then the chance that the key data
/// is corrupt is 2^-64.  If unwrapping produces A[0] any other value,
/// then the unwrap must return an error and not return any key data.
/// ```
pub const IV: [u8; IV_LEN] = [0xA6, 0xA6, 0xA6, 0xA6, 0xA6, 0xA6, 0xA6, 0xA6];

/// Alternative Initial Value constant prefix for AES-KWP as defined in
/// RFC3394 § 3.
///
/// <https://datatracker.ietf.org/doc/html/rfc5649#section-3>
///
/// ```text
/// The Alternative Initial Value (AIV) required by this specification is
//  a 32-bit constant concatenated to a 32-bit MLI.  The constant is (in
//  hexadecimal) A65959A6 and occupies the high-order half of the AIV.
/// ```
pub const KWP_IV_PREFIX: [u8; IV_LEN / 2] = [0xA6, 0x59, 0x59, 0xA6];

/// A Key-Encrypting-Key (KEK) that can be used to wrap and unwrap other
/// keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kek<Aes>
where
    Aes: KeyInit + BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt + BlockDecrypt,
{
    /// Initialized cipher
    cipher: Aes,
}

/// AES-128 KEK
pub type KekAes128 = Kek<aes::Aes128>;

/// AES-192 KEK
pub type KekAes192 = Kek<aes::Aes192>;

/// AES-256 KEK
pub type KekAes256 = Kek<aes::Aes256>;

impl From<GenericArray<u8, U16>> for KekAes128 {
    fn from(kek: GenericArray<u8, U16>) -> Self {
        Kek::new(&kek)
    }
}

impl From<GenericArray<u8, U24>> for KekAes192 {
    fn from(kek: GenericArray<u8, U24>) -> Self {
        Kek::new(&kek)
    }
}

impl From<GenericArray<u8, U32>> for KekAes256 {
    fn from(kek: GenericArray<u8, U32>) -> Self {
        Kek::new(&kek)
    }
}

impl From<[u8; 16]> for KekAes128 {
    fn from(kek: [u8; 16]) -> Self {
        Kek::new(&kek.into())
    }
}

impl From<[u8; 24]> for KekAes192 {
    fn from(kek: [u8; 24]) -> Self {
        Kek::new(&kek.into())
    }
}

impl From<[u8; 32]> for KekAes256 {
    fn from(kek: [u8; 32]) -> Self {
        Kek::new(&kek.into())
    }
}

impl<Aes> TryFrom<&[u8]> for Kek<Aes>
where
    Aes: KeyInit + BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt + BlockDecrypt,
{
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        if value.len() == Aes::KeySize::to_usize() {
            Ok(Kek::new(GenericArray::from_slice(value)))
        } else {
            Err(Error::InvalidKekSize { size: value.len() })
        }
    }
}

impl<Aes> Kek<Aes>
where
    Aes: KeyInit + BlockCipher + BlockSizeUser<BlockSize = U16> + BlockEncrypt + BlockDecrypt,
{
    /// Constructs a new Kek based on the appropriate raw key material.
    pub fn new(key: &GenericArray<u8, Aes::KeySize>) -> Self {
        let cipher = Aes::new(key);
        Kek { cipher }
    }

    /// AES Key Wrap, as defined in RFC 3394.
    ///
    /// The `out` buffer will be overwritten, and must be exactly [`IV_LEN`]
    /// bytes (i.e. 8 bytes) longer than the length of `data`.
    pub fn wrap(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        if data.len() % SEMIBLOCK_SIZE != 0 {
            return Err(Error::InvalidDataSize);
        }

        if out.len() != data.len() + IV_LEN {
            return Err(Error::InvalidOutputSize {
                expected: data.len() + IV_LEN,
            });
        }

        // 0) Prepare inputs

        // number of 64 bit blocks in the input data
        let n = data.len() / 8;

        // 1) Initialize variables

        // Set A to the IV
        let block = &mut Block::<WCtx<'_>>::default();
        block[..IV_LEN].copy_from_slice(&IV);

        // 2) Calculate intermediate values
        out[IV_LEN..].copy_from_slice(data);

        self.cipher.encrypt_with_backend(WCtx { n, block, out });

        // 3) Output the results
        out[..IV_LEN].copy_from_slice(&block[..IV_LEN]);

        Ok(())
    }

    /// Computes [`Self::wrap`], allocating a [`Vec`] for the return value.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn wrap_vec(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; data.len() + IV_LEN];
        self.wrap(data, &mut out)?;
        Ok(out)
    }

    /// AES Key Unwrap, as defined in RFC 3394.
    ///
    /// The `out` buffer will be overwritten, and must be exactly [`IV_LEN`]
    /// bytes (i.e. 8 bytes) shorter than the length of `data`.
    pub fn unwrap(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        if data.len() % SEMIBLOCK_SIZE != 0 {
            return Err(Error::InvalidDataSize);
        }

        // 0) Prepare inputs

        let n = (data.len() / SEMIBLOCK_SIZE)
            .checked_sub(1)
            .ok_or(Error::InvalidDataSize)?;

        if out.len() != n * SEMIBLOCK_SIZE {
            return Err(Error::InvalidOutputSize {
                expected: n * SEMIBLOCK_SIZE,
            });
        }

        // 1) Initialize variables

        let block = &mut Block::<WInverseCtx<'_>>::default();
        block[..IV_LEN].copy_from_slice(&data[..IV_LEN]);

        //   for i = 1 to n: R[i] = C[i]
        out.copy_from_slice(&data[IV_LEN..]);

        // 2) Calculate intermediate values

        self.cipher
            .decrypt_with_backend(WInverseCtx { n, block, out });

        // 3) Output the results

        if block[..IV_LEN] == IV[..] {
            Ok(())
        } else {
            Err(Error::IntegrityCheckFailed)
        }
    }

    /// Computes [`Self::unwrap`], allocating a [`Vec`] for the return value.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn unwrap_vec(&self, data: &[u8]) -> Result<Vec<u8>> {
        let out_len = data
            .len()
            .checked_sub(IV_LEN)
            .ok_or(Error::InvalidDataSize)?;

        let mut out = vec![0u8; out_len];
        self.unwrap(data, &mut out)?;
        Ok(out)
    }

    /// AES Key Wrap with Padding, as defined in RFC 5649.
    ///
    ///
    /// The `out` buffer will be overwritten, and must be the smallest
    /// multiple of [`SEMIBLOCK_SIZE`] (i.e. 8) which is at least [`IV_LEN`]
    /// bytes (i.e. 8 bytes) longer than the length of `data`.
    pub fn wrap_with_padding(&self, data: &[u8], out: &mut [u8]) -> Result<()> {
        if data.len() > KWP_MAX_LEN {
            return Err(Error::InvalidDataSize);
        }

        // 0) Prepare inputs

        // number of 64 bit blocks in the input data (padded)
        let n = (data.len() + SEMIBLOCK_SIZE - 1) / SEMIBLOCK_SIZE;

        if out.len() != n * SEMIBLOCK_SIZE + IV_LEN {
            return Err(Error::InvalidOutputSize {
                expected: n * SEMIBLOCK_SIZE + IV_LEN,
            });
        }

        // 32-bit MLI equal to the number of bytes in the input data, big endian
        let mli = (data.len() as u32).to_be_bytes();

        // 2) Wrapping

        // 2.1) Initialize variables

        // Set A to the AIV
        let block = &mut Block::<WCtx<'_>>::default();
        block[..IV_LEN / 2].copy_from_slice(&KWP_IV_PREFIX);
        block[IV_LEN / 2..IV_LEN].copy_from_slice(&mli);

        // If n is 1, the plaintext is encrypted as a single AES block
        if n == 1 {
            // 1) Append padding

            // GenericArrays should be zero by default, but zeroize again to be sure
            for i in data.len()..n * SEMIBLOCK_SIZE {
                block[IV_LEN + i] = 0;
            }

            block[IV_LEN..IV_LEN + data.len()].copy_from_slice(data);

            self.cipher.encrypt_block(block);
            out.copy_from_slice(block);
        } else {
            // 1) Append padding

            // Don't trust the caller to provide a zeroized out buffer, zeroize again to be sure
            for i in data.len()..n * SEMIBLOCK_SIZE {
                out[IV_LEN + i] = 0;
            }

            // 2.2) Calculate intermediate values
            out[IV_LEN..IV_LEN + data.len()].copy_from_slice(data);

            self.cipher.encrypt_with_backend(WCtx { n, block, out });

            // 2.3) Output the results
            out[..IV_LEN].copy_from_slice(&block[..IV_LEN]);
        }

        Ok(())
    }

    /// Computes [`Self::wrap`], allocating a [`Vec`] for the return value.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn wrap_with_padding_vec(&self, data: &[u8]) -> Result<Vec<u8>> {
        let n = (data.len() + SEMIBLOCK_SIZE - 1) / SEMIBLOCK_SIZE;
        let mut out = vec![0u8; n * SEMIBLOCK_SIZE + IV_LEN];
        self.wrap_with_padding(data, &mut out)?;
        Ok(out)
    }

    /// AES Key Wrap with Padding, as defined in RFC 5649.
    ///
    /// The `out` buffer will be overwritten, and must be exactly [`IV_LEN`]
    /// bytes (i.e. 8 bytes) shorter than the length of `data`.
    /// This method returns a slice of `out`, truncated to the appropriate
    /// length by removing the padding.
    pub fn unwrap_with_padding<'a>(&self, data: &[u8], out: &'a mut [u8]) -> Result<&'a [u8]> {
        if data.len() % SEMIBLOCK_SIZE != 0 {
            return Err(Error::InvalidDataSize);
        }

        // 0) Prepare inputs

        let n = (data.len() / SEMIBLOCK_SIZE)
            .checked_sub(1)
            .ok_or(Error::InvalidDataSize)?;

        if out.len() != n * SEMIBLOCK_SIZE {
            return Err(Error::InvalidOutputSize {
                expected: n * SEMIBLOCK_SIZE,
            });
        }

        // 1) Key unwrapping

        // 1.1) Initialize variables

        let block = &mut Block::<WInverseCtx<'_>>::default();

        // If n is 1, the plaintext is encrypted as a single AES block
        if n == 1 {
            block.copy_from_slice(data);

            self.cipher.decrypt_block(block);
            out.copy_from_slice(&block[IV_LEN..]);
        } else {
            block[..IV_LEN].copy_from_slice(&data[..IV_LEN]);

            //   for i = 1 to n: R[i] = C[i]
            out.copy_from_slice(&data[IV_LEN..]);

            // 1.2) Calculate intermediate values

            self.cipher
                .decrypt_with_backend(WInverseCtx { n, block, out });
        }

        // 2) AIV verification

        // Checks as defined in RFC5649 § 3

        if block[..IV_LEN / 2] != KWP_IV_PREFIX {
            return Err(Error::IntegrityCheckFailed);
        }

        let mli = u32::from_be_bytes(block[IV_LEN / 2..IV_LEN].try_into().unwrap()) as usize;
        if !(SEMIBLOCK_SIZE * (n - 1) < mli && mli <= SEMIBLOCK_SIZE * n) {
            return Err(Error::IntegrityCheckFailed);
        }

        let b = SEMIBLOCK_SIZE * n - mli;
        if !out.iter().rev().take(b).all(|&x| x == 0) {
            return Err(Error::IntegrityCheckFailed);
        }

        // 3) Output the results

        Ok(&out[..mli])
    }

    /// Computes [`Self::unwrap`], allocating a [`Vec`] for the return value.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn unwrap_with_padding_vec(&self, data: &[u8]) -> Result<Vec<u8>> {
        let out_len = data
            .len()
            .checked_sub(IV_LEN)
            .ok_or(Error::InvalidDataSize)?;

        let mut out = vec![0u8; out_len];
        let out_len = self.unwrap_with_padding(data, &mut out)?.len();
        out.truncate(out_len);
        Ok(out)
    }
}

struct WCtx<'a> {
    n: usize,
    block: &'a mut Block<Self>,
    out: &'a mut [u8],
}

impl<'a> BlockSizeUser for WCtx<'a> {
    type BlockSize = U16;
}

/// Very similar to the W(S) function defined by NIST in SP 800-38F,
/// Section 6.1
impl<'a> BlockClosure for WCtx<'a> {
    #[inline(always)]
    fn call<B: BlockBackend<BlockSize = Self::BlockSize>>(self, backend: &mut B) {
        for j in 0..=5 {
            for (i, chunk) in self.out.chunks_mut(SEMIBLOCK_SIZE).skip(1).enumerate() {
                // A | R[i]
                self.block[IV_LEN..].copy_from_slice(chunk);
                // B = AES(K, ..)
                backend.proc_block(self.block.into());

                // A = MSB(64, B) ^ t
                let t = (self.n * j + (i + 1)) as u64;
                for (ai, ti) in self.block[..IV_LEN].iter_mut().zip(&t.to_be_bytes()) {
                    *ai ^= ti;
                }

                // R[i] = LSB(64, B)
                chunk.copy_from_slice(&self.block[IV_LEN..]);
            }
        }
    }
}

struct WInverseCtx<'a> {
    n: usize,
    block: &'a mut Block<Self>,
    out: &'a mut [u8],
}

impl<'a> BlockSizeUser for WInverseCtx<'a> {
    type BlockSize = U16;
}

/// Very similar to the W^-1(S) function defined by NIST in SP 800-38F,
/// Section 6.1
impl<'a> BlockClosure for WInverseCtx<'a> {
    #[inline(always)]
    fn call<B: BlockBackend<BlockSize = Self::BlockSize>>(self, backend: &mut B) {
        for j in (0..=5).rev() {
            for (i, chunk) in self.out.chunks_mut(SEMIBLOCK_SIZE).enumerate().rev() {
                // A ^ t
                let t = (self.n * j + (i + 1)) as u64;
                for (ai, ti) in self.block[..IV_LEN].iter_mut().zip(&t.to_be_bytes()) {
                    *ai ^= ti;
                }

                // (A ^ t) | R[i]
                self.block[IV_LEN..].copy_from_slice(chunk);

                // B = AES-1(K, ..)
                backend.proc_block(self.block.into());

                // A = MSB(64, B)
                // already set

                // R[i] = LSB(64, B)
                chunk.copy_from_slice(&self.block[IV_LEN..]);
            }
        }
    }
}
