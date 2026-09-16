use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DependencySpec {
    Oci {
        reference: String,
        tag: String,
    },
    AltRpm {
        repository: String,
        packages: Vec<String>,
    },
}
