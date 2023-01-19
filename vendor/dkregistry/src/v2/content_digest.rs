/// Implements types and methods for content verification
use sha2::{self, Digest};
use std::str;

/// DigestAlgorithm declares the supported algorithms
#[derive(Display, Clone, Debug)]
pub enum DigestAlgorithm {
    Sha256(sha2::Sha256),
}

impl std::str::FromStr for DigestAlgorithm {
    type Err = ContentDigestError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "sha256" => Ok(DigestAlgorithm::Sha256(sha2::Sha256::new())),
            _ => Err(ContentDigestError::AlgorithmUnknown(name.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContentDigestError {
    #[error("digest {0} does not have algorithm prefix")]
    BadDigest(String),
    #[error("unknown algorithm: {0}")]
    AlgorithmUnknown(String),
    #[error("verification failed: expected '{expected}', got '{got}'")]
    Verify { expected: String, got: String },
}

/// ContentDigest stores a digest and its DigestAlgorithm
#[derive(Clone, Debug)]
pub struct ContentDigest {
    digest: String,
    algorithm: DigestAlgorithm,
}

impl ContentDigest {
    /// try_new attempts to parse the digest string and create a ContentDigest instance from it
    ///
    /// Success depends on
    /// - the string having an "algorithm:" prefix
    /// - the algorithm being supported by DigestAlgorithm
    pub fn try_new(digest: &str) -> std::result::Result<Self, ContentDigestError> {
        let digest_split = digest.split(':').collect::<Vec<&str>>();

        if digest_split.len() != 2 {
            return Err(ContentDigestError::BadDigest(digest.to_string()));
        }

        let algorithm = std::str::FromStr::from_str(digest_split[0])?;
        Ok(ContentDigest {
            digest: digest.to_string(),
            algorithm,
        })
    }

    pub fn update(&mut self, input: &[u8]) {
        self.algorithm.update(input)
    }

    pub fn verify(self) -> std::result::Result<(), ContentDigestError> {
        let digest = self.algorithm.digest();
        if digest != self.digest {
            return Err(ContentDigestError::Verify {
                expected: self.digest,
                got: digest,
            });
        }
        Ok(())
    }
}

impl std::fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.digest)
    }
}

impl DigestAlgorithm {
    fn update(&mut self, input: &[u8]) {
        match self {
            DigestAlgorithm::Sha256(hash) => {
                hash.update(input);
            }
        }
    }

    fn digest(self) -> String {
        let (algo, digest) = match self {
            DigestAlgorithm::Sha256(hash) => ("sha256", hash.finalize()),
        };
        format!("{}:{:x}", algo, &digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2;

    type Fallible<T> = Result<T, crate::Error>;

    #[test]
    fn try_new_succeeds_with_correct_digest() -> Fallible<()> {
        for correct_digest in
            &["sha256:0000000000000000000000000000000000000000000000000000000000000000"]
        {
            ContentDigest::try_new(correct_digest)?;
        }

        Ok(())
    }

    #[test]
    fn try_new_fails_with_incorrect_digest() {
        for incorrect_digest in &[
            "invalid",
            "invalid:",
            "invalid:0000000000000000000000000000000000000000000000000000000000000000",
        ] {
            if ContentDigest::try_new(incorrect_digest).is_ok() {
                panic!(
                    "expected try_new to fail for incorrect digest {}",
                    incorrect_digest
                );
            }
        }
    }

    #[test]
    fn verify_succeeds_with_same_content() -> Fallible<()> {
        let blob: &[u8] = b"somecontent";
        let mut content_digest = ContentDigest::try_new(
            "sha256:d5a3477d91583e65a7aba6f6db7a53e2de739bc7bf8f4a08f0df0457b637f1fb",
        )?;
        content_digest.update(blob);
        content_digest.verify().map_err(Into::into)
    }

    #[test]
    fn verify_chunked_succeeds_with_same_content() -> Fallible<()> {
        let mut content_digest = ContentDigest::try_new(
            "sha256:d5a3477d91583e65a7aba6f6db7a53e2de739bc7bf8f4a08f0df0457b637f1fb",
        )?;
        content_digest.update(b"some");
        content_digest.update(b"content");
        content_digest.verify().map_err(Into::into)
    }

    #[test]
    fn verify_fails_with_different_content() -> Fallible<()> {
        let blob: &[u8] = b"somecontent";
        let different_blob: &[u8] = b"someothercontent";

        let mut expected_digest = DigestAlgorithm::Sha256(sha2::Sha256::new());
        expected_digest.update(different_blob);
        let expected_digest = expected_digest.digest();

        let mut content_digest = ContentDigest::try_new(&expected_digest)?;
        content_digest.update(blob);
        if content_digest.verify().is_ok() {
            panic!("expected try_verify to fail for a different blob");
        }
        Ok(())
    }
}
