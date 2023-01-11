use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    /// an optional registry, generally Docker Hub if unset
    pub registry: Option<String>,

    /// an image string, possibly including a user or organization name
    pub image: String,

    /// An optional image tag (after the colon, e.g. `:1.2.3`), generally inferred
    /// to mean `:latest` if unset
    pub tag: Option<String>,

    /// An optional embedded image hash, e.g. `sha256:...`. Conflicts with `tag`.
    pub hash: Option<String>,
}

/// Determines if an ImageRef token refers to a registry hostname or not
///
/// Based on rules from https://stackoverflow.com/a/42116190
fn is_registry(token: &str) -> bool {
    token == "localhost" || token.contains('.') || token.contains(':')
}

impl ImageRef {
    /// Parses an `ImageRef` from a string.
    ///
    /// This is not fallible, however malformed image strings may return
    /// unexpected results.
    pub fn parse(s: &str) -> ImageRef {
        let parts: Vec<&str> = s.splitn(2, '/').collect();
        let (registry, mut image_full) = if parts.len() == 2 && is_registry(parts[0]) {
            // some 3rd party registry
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            // default to docker.io
            (Some("docker.io".to_string()), s.to_string())
        };

        if !image_full.chars().any(|c| c == '/') && registry.as_ref().unwrap().eq("docker.io") {
            image_full = format!("library/{}", image_full);
        }

        if let Some(at_pos) = image_full.find('@') {
            // parts length is guaranteed to be at least 1 given an empty string
            let (image, hash) = image_full.split_at(at_pos);

            ImageRef {
                registry,
                image: image.to_string(),
                hash: Some(hash[1..].to_string()),
                tag: None,
            }
        } else {
            // parts length is guaranteed to be at least 1 given an empty string
            let parts: Vec<&str> = image_full.splitn(2, ':').collect();
            let image = parts[0].to_string();
            let tag = Some(
                parts
                    .get(1)
                    .map_or_else(|| "latest".to_string(), |p| String::from(*p)),
            );

            ImageRef {
                registry,
                image,
                tag,
                hash: None,
            }
        }
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(registry) = &self.registry {
            write!(f, "{}/", registry)?;
        }

        write!(f, "{}", self.image)?;

        if let Some(tag) = &self.tag {
            write!(f, ":{}", tag)?;
        } else if let Some(hash) = &self.hash {
            write!(f, "@{}", hash)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_parse_dockerhub() {
        assert_eq!(
            ImageRef::parse("alpine:3.10"),
            ImageRef {
                registry: Some("docker.io".to_string()),
                image: "library/alpine".into(),
                tag: Some("3.10".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("library/nginx"),
            ImageRef {
                registry: Some("docker.io".into()),
                image: "library/nginx".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("fake_project/fake_image@fake_hash"),
            ImageRef {
                registry: Some("docker.io".into()),
                image: "fake_project/fake_image".into(),
                tag: None,
                hash: Some("fake_hash".into())
            }
        );

        // invalid hashes, but should still not panic
        assert_eq!(
            ImageRef::parse("fake_project/fake_image@"),
            ImageRef {
                registry: Some("docker.io".into()),
                image: "fake_project/fake_image".into(),
                tag: None,
                hash: Some("".into())
            }
        );

        assert_eq!(
            ImageRef::parse("fake_project/fake_image@sha256:"),
            ImageRef {
                registry: Some("docker.io".into()),
                image: "fake_project/fake_image".into(),
                tag: None,
                hash: Some("sha256:".into())
            }
        );
    }

    #[test]
    fn test_image_parse_registry() {
        assert_eq!(
            ImageRef::parse("quay.io/prometheus/node-exporter:v0.18.1"),
            ImageRef {
                registry: Some("quay.io".into()),
                image: "prometheus/node-exporter".into(),
                tag: Some("v0.18.1".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("gcr.io/fake_project/fake_image:fake_tag"),
            ImageRef {
                registry: Some("gcr.io".into()),
                image: "fake_project/fake_image".into(),
                tag: Some("fake_tag".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("gcr.io/fake_project/fake_image"),
            ImageRef {
                registry: Some("gcr.io".into()),
                image: "fake_project/fake_image".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("gcr.io/fake_image"),
            ImageRef {
                registry: Some("gcr.io".into()),
                image: "fake_image".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("quay.io/fake_project/fake_image@fake_hash"),
            ImageRef {
                registry: Some("quay.io".into()),
                image: "fake_project/fake_image".into(),
                tag: None,
                hash: Some("fake_hash".into())
            }
        );
    }

    #[test]
    fn test_image_parse_localhost() {
        assert_eq!(
            ImageRef::parse("localhost/foo"),
            ImageRef {
                registry: Some("localhost".into()),
                image: "foo".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("localhost/foo:bar"),
            ImageRef {
                registry: Some("localhost".into()),
                image: "foo".into(),
                tag: Some("bar".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("localhost/foo/bar"),
            ImageRef {
                registry: Some("localhost".into()),
                image: "foo/bar".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("localhost/foo/bar:baz"),
            ImageRef {
                registry: Some("localhost".into()),
                image: "foo/bar".into(),
                tag: Some("baz".into()),
                hash: None
            }
        );
    }

    #[test]
    fn test_image_parse_registry_port() {
        assert_eq!(
            ImageRef::parse("example.com:1234/foo"),
            ImageRef {
                registry: Some("example.com:1234".into()),
                image: "foo".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("example.com:1234/foo:bar"),
            ImageRef {
                registry: Some("example.com:1234".into()),
                image: "foo".into(),
                tag: Some("bar".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("example.com:1234/foo/bar"),
            ImageRef {
                registry: Some("example.com:1234".into()),
                image: "foo/bar".into(),
                tag: Some("latest".into()),
                hash: None
            }
        );

        assert_eq!(
            ImageRef::parse("example.com:1234/foo/bar:baz"),
            ImageRef {
                registry: Some("example.com:1234".into()),
                image: "foo/bar".into(),
                tag: Some("baz".into()),
                hash: None
            }
        );

        // docker hub doesn't allow it, but other registries can allow arbitrarily
        // nested images
        assert_eq!(
            ImageRef::parse("example.com:1234/foo/bar/baz:qux"),
            ImageRef {
                registry: Some("example.com:1234".into()),
                image: "foo/bar/baz".into(),
                tag: Some("qux".into()),
                hash: None
            }
        );
    }
}
