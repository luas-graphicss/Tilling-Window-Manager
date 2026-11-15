use sha2::{Digest, Sha256};

use super::Artifact;

pub fn sha256(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

pub fn listing(artifacts: &[Artifact]) -> String {
    artifacts
        .iter()
        .map(|artifact| format!("{}  {}\n", sha256(&artifact.contents), artifact.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_matches_the_published_test_vector() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn listing_has_the_layout_sha256sum_verifies() {
        let artifacts = [
            Artifact {
                name: "tilling_0.1.0_amd64.deb".to_owned(),
                contents: b"abc".to_vec(),
            },
            Artifact {
                name: "tilling-0.1.0.cdx.json".to_owned(),
                contents: Vec::new(),
            },
        ];

        assert_eq!(
            listing(&artifacts),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  tilling_0.1.0_amd64.deb\n\
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  tilling-0.1.0.cdx.json\n"
        );
    }
}
