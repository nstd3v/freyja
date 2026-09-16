#[cfg(test)]
mod tests {
    use freyja_core::spec::Spec;
    use std::{path::PathBuf, str::FromStr};

    const EXAMPLE_SPEC_PATH: &str = "../../examples/freyja.toml";

    fn read_example_spec() -> Spec {
        let path = PathBuf::from_str(EXAMPLE_SPEC_PATH).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        toml::from_str(&content).unwrap()
    }

    #[test]
    fn test_parsing() {
        let spec = read_example_spec();

        assert_eq!(spec.version, "1");
        assert_eq!(spec.title, "freyja example build spec");
    }
}
