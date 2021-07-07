/// Implements types and methods for content verification
use sha2::{self, Digest};

/// ContentDigest stores a digest and its DigestAlgorithm
#[derive(Clone, Debug, PartialEq)]
pub struct ContentDigest {
    digest: String,
    algorithm: DigestAlgorithm,
}

/// DigestAlgorithm declares the supported algorithms
#[derive(Display, Clone, Debug, PartialEq, EnumString)]
pub enum DigestAlgorithm {
    #[strum(to_string = "sha256")]
    Sha256,
}

#[derive(Debug, thiserror::Error)]
pub enum ContentDigestError {
    #[error("digest {0} does not have algorithm prefix")]
    BadDigest(String),
    #[error("unknown algorithm")]
    AlgorithmUnknown(#[from] <DigestAlgorithm as std::str::FromStr>::Err),
    #[error("verification failed: expected '{expected}', got '{got}'")]
    Verify {
        expected: ContentDigest,
        got: ContentDigest,
    },
}

impl ContentDigest {
    /// try_new attempts to parse the digest string and create a ContentDigest instance from it
    ///
    /// Success depends on
    /// - the string having a "algorithm:" prefix
    /// - the algorithm being supported by DigestAlgorithm
    pub fn try_new(digest: String) -> std::result::Result<Self, ContentDigestError> {
        let digest_split = digest.split(':').collect::<Vec<&str>>();

        if digest_split.len() != 2 {
            return Err(ContentDigestError::BadDigest(digest));
        }

        let algorithm = std::str::FromStr::from_str(digest_split[0])?;
        Ok(ContentDigest {
            digest: digest_split[1].to_string(),
            algorithm,
        })
    }

    /// try_verify hashes the input slice and compares it with the digest stored in this instance
    ///
    /// Success depends on the result of the comparison
    pub fn try_verify(&self, input: &[u8]) -> std::result::Result<(), ContentDigestError> {
        let hash = self.algorithm.hash(input);
        let layer_digest = Self::try_new(hash)?;

        if self != &layer_digest {
            return Err(ContentDigestError::Verify {
                expected: self.clone(),
                got: layer_digest.clone(),
            });
        }

        trace!("content verification succeeded for '{}'", &layer_digest);
        Ok(())
    }
}

impl std::fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.digest)
    }
}

impl DigestAlgorithm {
    fn hash(&self, input: &[u8]) -> String {
        match self {
            DigestAlgorithm::Sha256 => {
                let hash = sha2::Sha256::digest(input);
                format!("{}:{:x}", self, hash)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type Fallible<T> = Result<T, crate::Error>;

    #[test]
    fn try_new_succeeds_with_correct_digest() -> Fallible<()> {
        for correct_digest in
            &["sha256:0000000000000000000000000000000000000000000000000000000000000000"]
        {
            ContentDigest::try_new(correct_digest.to_string())?;
        }

        Ok(())
    }

    #[test]
    fn try_new_succeeds_with_incorrect_digest() {
        for incorrect_digest in &[
            "invalid",
            "invalid:",
            "invalid:0000000000000000000000000000000000000000000000000000000000000000",
        ] {
            if ContentDigest::try_new(incorrect_digest.to_string()).is_ok() {
                panic!(
                    "expected try_new to fail for incorrect digest {}",
                    incorrect_digest
                );
            }
        }
    }

    #[test]
    fn try_verify_succeeds_with_same_content() -> Fallible<()> {
        let blob: &[u8] = b"somecontent";
        let digest = DigestAlgorithm::Sha256.hash(&blob);

        ContentDigest::try_new(digest)?
            .try_verify(&blob)
            .map_err(Into::into)
    }

    #[test]
    fn try_verify_fails_with_different_content() -> Fallible<()> {
        let blob: &[u8] = b"somecontent";
        let different_blob: &[u8] = b"someothercontent";
        let digest = DigestAlgorithm::Sha256.hash(&blob);

        if ContentDigest::try_new(digest)?
            .try_verify(&different_blob)
            .is_ok()
        {
            panic!("expected try_verify to fail for a different blob");
        }
        Ok(())
    }
}
