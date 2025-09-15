use aes_kw::Error;
use aes_kw::Kek;
use hex_literal::hex;
use std::assert_eq;

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_aes_kw {
        ($name:ident, $kek_typ:ty, $kek:expr, $input:expr, $output:expr) => {
            #[test]
            fn $name() {
                let kek = Kek::<$kek_typ>::from($kek);

                let mut wrapped = [0u8; $output.len()];
                kek.wrap(&$input, &mut wrapped).unwrap();

                let mut unwrapped = [0u8; $input.len()];
                kek.unwrap(&wrapped, &mut unwrapped).unwrap();

                assert_eq!($output, wrapped, "failed wrap");
                assert_eq!($input, unwrapped, "failed unwrap");
            }
        };
    }

    test_aes_kw!(
        wrap_unwrap_128_key_128_kek,
        aes::Aes128,
        hex!("000102030405060708090A0B0C0D0E0F"),
        hex!("00112233445566778899AABBCCDDEEFF"),
        hex!("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5")
    );
    test_aes_kw!(
        wrap_unwrap_128_key_192_kek,
        aes::Aes192,
        hex!("000102030405060708090A0B0C0D0E0F1011121314151617"),
        hex!("00112233445566778899AABBCCDDEEFF"),
        hex!("96778B25AE6CA435F92B5B97C050AED2468AB8A17AD84E5D")
    );
    test_aes_kw!(
        wrap_unwrap_128_key_256_kek,
        aes::Aes256,
        hex!("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"),
        hex!("00112233445566778899AABBCCDDEEFF"),
        hex!("64E8C3F9CE0F5BA263E9777905818A2A93C8191E7D6E8AE7")
    );
    test_aes_kw!(
        wrap_unwrap_192_key_192_kek,
        aes::Aes192,
        hex!("000102030405060708090A0B0C0D0E0F1011121314151617"),
        hex!("00112233445566778899AABBCCDDEEFF0001020304050607"),
        hex!("031D33264E15D33268F24EC260743EDCE1C6C7DDEE725A936BA814915C6762D2")
    );
    test_aes_kw!(
        wrap_unwrap_192_key_256_kek,
        aes::Aes256,
        hex!("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"),
        hex!("00112233445566778899AABBCCDDEEFF0001020304050607"),
        hex!("A8F9BC1612C68B3FF6E6F4FBE30E71E4769C8B80A32CB8958CD5D17D6B254DA1")
    );
    test_aes_kw!(
        wrap_unwrap_256_key_256_kek,
        aes::Aes256,
        hex!("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"),
        hex!("00112233445566778899AABBCCDDEEFF000102030405060708090A0B0C0D0E0F"),
        hex!("28C9F404C4B810F4CBCCB35CFB87F8263F5786E2D80ED326CBC7F0E71A99F43BFB988B9B7A02DD21")
    );

    #[test]
    fn error_invalid_data_size() {
        let kek = hex!("000102030405060708090A0B0C0D0E0F");
        let input = hex!("00112233445566778899AABBCCDDEE");
        let output = hex!("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CF");

        let kek = Kek::<aes::Aes128>::from(kek);

        let mut wrapped = [0u8; 24];
        let result = kek.wrap(&input, &mut wrapped);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidDataSize));

        let mut unwrapped = [0u8; 16];
        let result = kek.unwrap(&output, &mut unwrapped);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidDataSize));

        let mut unwrapped = [0u8; 0];
        let result = kek.unwrap(&[], &mut unwrapped);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidDataSize));
    }

    #[test]
    fn error_invalid_kek_size() {
        let kek = hex!("000102030405060708090A0B0C0D0E");

        let result = Kek::<aes::Aes128>::try_from(&kek[..]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidKekSize { size: 15 }
        ));
    }

    #[test]
    fn error_invalid_output_size() {
        let kek = hex!("000102030405060708090A0B0C0D0E0F");
        let input = hex!("00112233445566778899AABBCCDDEEFF");
        let output = hex!("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5");

        let kek = Kek::<aes::Aes128>::from(kek);

        let mut wrapped = [0u8; 23];
        let result = kek.wrap(&input, &mut wrapped);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidOutputSize { expected: 24 }
        ));

        let mut unwrapped = [0u8; 15];
        let result = kek.unwrap(&output, &mut unwrapped);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidOutputSize { expected: 16 }
        ));
    }

    #[test]
    fn error_integrity_check_failed() {
        let kek = hex!("000102030405060708090A0B0C0D0E0F");
        let output = hex!("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE6");

        let kek = Kek::<aes::Aes128>::from(kek);

        let mut unwrapped = [0u8; 16];
        let result = kek.unwrap(&output, &mut unwrapped);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::IntegrityCheckFailed));
    }
}
