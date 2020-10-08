//! CryptoAPI key providers.
use std::io;
use std::ptr;
use winapi::shared::minwindef as winapi;
use winapi::um::winbase;
use winapi::um::wincrypt;

use Inner;
use crypt_key::CryptKey;

/// A CryptoAPI handle to a provider of a key.
pub struct CryptProv(wincrypt::HCRYPTPROV);

impl Drop for CryptProv {
    fn drop(&mut self) {
        unsafe {
            wincrypt::CryptReleaseContext(self.0, 0);
        }
    }
}

inner!(CryptProv, wincrypt::HCRYPTPROV);

impl CryptProv {
    /// Imports a key into this provider.
    pub fn import<'a>(&'a mut self) -> ImportOptions<'a> {
        ImportOptions {
            prov: self,
            flags: 0,
        }
    }
}

/// A builder for `CryptProv`s.
pub struct AcquireOptions {
    container: Option<Vec<u16>>,
    provider: Option<Vec<u16>>,
    flags: winapi::DWORD,
}

impl AcquireOptions {
    /// Returns a new builder with default settings.
    pub fn new() -> AcquireOptions {
        AcquireOptions {
            container: None,
            provider: None,
            flags: 0,
        }
    }

    /// Sets the name for this key container.
    ///
    /// This should not be set if `verify_context` is set.
    pub fn container(&mut self, container: &str) -> &mut AcquireOptions {
        self.container = Some(container.encode_utf16().chain(Some(0)).collect());
        self
    }

    /// Sets the name of the CSP to be used.
    pub fn provider(&mut self, provider: &str) -> &mut AcquireOptions {
        self.provider = Some(provider.encode_utf16().chain(Some(0)).collect());
        self
    }

    /// If set, private keys will not be accessible or persisted.
    pub fn verify_context(&mut self, verify_context: bool) -> &mut AcquireOptions {
        self.flag(wincrypt::CRYPT_VERIFYCONTEXT, verify_context)
    }

    /// If set, the container will be created.
    pub fn new_keyset(&mut self, new_keyset: bool) -> &mut AcquireOptions {
        self.flag(wincrypt::CRYPT_NEWKEYSET, new_keyset)
    }

    /// If set, the container will be stored as a machine rather than user keys.
    pub fn machine_keyset(&mut self, machine_keyset: bool) -> &mut AcquireOptions {
        self.flag(wincrypt::CRYPT_MACHINE_KEYSET, machine_keyset)
    }

    /// If set, an error will be returned if user intervention is required
    /// rather than displaying a dialog.
    pub fn silent(&mut self, silent: bool) -> &mut AcquireOptions {
        self.flag(wincrypt::CRYPT_SILENT, silent)
    }

    fn flag(&mut self, flag: winapi::DWORD, on: bool) -> &mut AcquireOptions {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }

        self
    }

    /// Acquires a container.
    pub fn acquire(&self, type_: ProviderType) -> io::Result<CryptProv> {
        unsafe {
            let container = self.container.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null());
            let provider = self.provider.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null());

            let mut prov = 0;
            let res = wincrypt::CryptAcquireContextW(&mut prov,
                                                     container as *mut _,
                                                     provider as *mut _,
                                                     type_.0,
                                                     self.flags);
            if res == winapi::TRUE {
                Ok(CryptProv(prov))
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }
}

/// An identifier of the type of cryptography provider to be used with a
/// container.
#[derive(Copy, Clone)]
pub struct ProviderType(winapi::DWORD);

#[allow(missing_docs)]
impl ProviderType {
    pub fn rsa_full() -> ProviderType {
        ProviderType(wincrypt::PROV_RSA_FULL)
    }

    pub fn rsa_aes() -> ProviderType {
        ProviderType(wincrypt::PROV_RSA_AES)
    }

    pub fn rsa_sig() -> ProviderType {
        ProviderType(wincrypt::PROV_RSA_SIG)
    }

    pub fn rsa_schannel() -> ProviderType {
        ProviderType(wincrypt::PROV_RSA_SCHANNEL)
    }

    pub fn dss() -> ProviderType {
        ProviderType(wincrypt::PROV_DSS)
    }

    pub fn dss_dh() -> ProviderType {
        ProviderType(wincrypt::PROV_DSS_DH)
    }

    pub fn dh_schannel() -> ProviderType {
        ProviderType(wincrypt::PROV_DH_SCHANNEL)
    }

    pub fn fortezza() -> ProviderType {
        ProviderType(wincrypt::PROV_FORTEZZA)
    }

    pub fn ms_exchange() -> ProviderType {
        ProviderType(wincrypt::PROV_MS_EXCHANGE)
    }

    pub fn ssl() -> ProviderType {
        ProviderType(wincrypt::PROV_SSL)
    }

    pub fn as_raw(&self) -> winapi::DWORD {
        self.0
    }
}

/// A builder for key imports.
pub struct ImportOptions<'a> {
    prov: &'a mut CryptProv,
    flags: winapi::DWORD,
}

impl<'a> ImportOptions<'a> {
    /// Imports a DER-encoded private key.
    pub fn import(&mut self, der: &[u8]) -> io::Result<CryptKey> {
        unsafe {
            assert!(der.len() <= winapi::DWORD::max_value() as usize);
            let mut buf = ptr::null_mut();
            let mut len = 0;
            let res = wincrypt::CryptDecodeObjectEx(wincrypt::X509_ASN_ENCODING |
                                                    wincrypt::PKCS_7_ASN_ENCODING,
                                                    wincrypt::PKCS_RSA_PRIVATE_KEY,
                                                    der.as_ptr(),
                                                    der.len() as winapi::DWORD,
                                                    wincrypt::CRYPT_DECODE_ALLOC_FLAG,
                                                    ptr::null_mut(),
                                                    &mut buf as *mut _ as winapi::LPVOID,
                                                    &mut len);
            if res == winapi::FALSE {
                return Err(io::Error::last_os_error());
            }

            let mut key = 0;
            let res = wincrypt::CryptImportKey(self.prov.0, buf, len, 0, self.flags, &mut key);
            winbase::LocalFree(buf as *mut _);

            if res == winapi::TRUE {
                Ok(CryptKey::from_inner(key))
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rsa_key() {
        let key = include_bytes!("../test/key.key");

        let mut context = AcquireOptions::new()
            .verify_context(true)
            .acquire(ProviderType::rsa_full())
            .unwrap();
        context.import()
            .import(key)
            .unwrap();
    }
}
