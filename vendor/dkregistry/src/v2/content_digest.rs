use crate::errors::Result;
/// Implements types and methods for content verification
use sha2::{self, Digest};

/// ContentDigest stores a digest and its DigestAlgorithm
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContentDigest {
    digest: String,
    algorithm: DigestAlgorithm,
}

/// DigestAlgorithm declares the supported algorithms
#[derive(Display, Clone, Debug, PartialEq, EnumString)]
enum DigestAlgorithm {
    #[strum(to_string = "sha256")]
    Sha256,
}

impl ContentDigest {
    /// try_new attempts to parse the digest string and create a ContentDigest instance from it
    ///
    /// Success depends on
    /// - the string having a "algorithm:" prefix
    /// - the algorithm being supported by DigestAlgorithm
    pub fn try_new(digest: String) -> Result<Self> {
        let digest_split = digest.split(':').collect::<Vec<&str>>();

        if digest_split.len() != 2 {
            return Err(format!("digest '{}' does not have an algorithm prefix", digest).into());
        }

        let algorithm =
            std::str::FromStr::from_str(digest_split[0]).map_err(|e| format!("{}", e))?;
        Ok(ContentDigest {
            digest: digest_split[1].to_string(),
            algorithm,
        })
    }

    /// try_verify hashes the input slice and compares it with the digest stored in this instance
    ///
    /// Success depends on the result of the comparison
    pub fn try_verify(&self, input: &[u8]) -> Result<()> {
        let hash = self.algorithm.hash(input);
        let layer_digest = Self::try_new(hash)?;

        if self != &layer_digest {
            return Err(format!(
                "content verification failed. expected '{}', got '{}'",
                self.to_owned(),
                layer_digest.to_owned()
            )
            .into());
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
    type Fallible<T> = Result<T>;

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
    fn try_new_succeeds_with_incorrect_digest() -> Fallible<()> {
        for incorrect_digest in &[
            "invalid",
            "invalid:",
            "invalid:0000000000000000000000000000000000000000000000000000000000000000",
        ] {
            if ContentDigest::try_new(incorrect_digest.to_string()).is_ok() {
                return Err(format!(
                    "expected try_new to fail for incorrect digest {}",
                    incorrect_digest
                )
                .into());
            }
        }

        Ok(())
    }

    #[test]
    fn try_verify_succeeds_with_same_content() -> Fallible<()> {
        let blob: &[u8] = b"somecontent";
        let digest = DigestAlgorithm::Sha256.hash(&blob);

        ContentDigest::try_new(digest)?.try_verify(&blob)
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
            return Err("expected try_verify to fail for a different blob".into());
        }

        Ok(())
    }
}
