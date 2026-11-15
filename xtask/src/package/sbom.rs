use serde_json::{Value, json};

use super::workspace::{Component, Workspace};

const DISTRIBUTED_APPLICATION: &str = "tilling";
const LEGACY_SEPARATOR: char = '/';
const UNDECLARED_LICENSE: &str = "license not declared";

pub fn document(workspace: &Workspace) -> Value {
    json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": DISTRIBUTED_APPLICATION,
                "version": workspace.version,
            },
        },
        "components": workspace.components.iter().map(describe).collect::<Vec<Value>>(),
    })
}

pub fn licenses(workspace: &Workspace) -> String {
    workspace
        .components
        .iter()
        .map(|component| {
            let license = expression(component).unwrap_or_else(|| UNDECLARED_LICENSE.to_owned());
            format!("{} {}: {license}\n", component.name, component.version)
        })
        .collect()
}

fn describe(component: &Component) -> Value {
    let mut described = json!({
        "type": "library",
        "name": component.name,
        "version": component.version,
        "purl": format!("pkg:cargo/{}@{}", component.name, component.version),
    });
    if let Some(license) = expression(component) {
        described["licenses"] = json!([{ "expression": license }]);
    }
    if let Some(checksum) = &component.registry_checksum {
        described["hashes"] = json!([{ "alg": "SHA-256", "content": checksum }]);
    }
    described
}

fn expression(component: &Component) -> Option<String> {
    component
        .license
        .as_ref()
        .map(|license| license.replace(LEGACY_SEPARATOR, " OR "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::workspace::tests::workspace;

    fn component<'document>(document: &'document Value, name: &str) -> &'document Value {
        document["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["name"] == name)
            .unwrap()
    }

    #[test]
    fn document_names_the_application_and_every_component() {
        let document = document(&workspace());

        assert_eq!(document["bomFormat"], "CycloneDX");
        assert_eq!(document["specVersion"], "1.5");
        assert_eq!(document["metadata"]["component"]["name"], "tilling");
        assert_eq!(document["metadata"]["component"]["version"], "0.1.0");
        assert_eq!(document["components"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn registry_component_carries_its_package_url_license_and_hash() {
        let document = document(&workspace());
        let serde = component(&document, "serde");

        assert_eq!(serde["purl"], "pkg:cargo/serde@1.0.229");
        assert_eq!(serde["licenses"][0]["expression"], "MIT OR Apache-2.0");
        assert_eq!(serde["hashes"][0]["alg"], "SHA-256");
        assert_eq!(serde["hashes"][0]["content"], "aa11");
    }

    #[test]
    fn missing_license_and_checksum_are_left_out_instead_of_invented() {
        let document = document(&workspace());

        assert!(component(&document, "unlicensed").get("licenses").is_none());
        assert!(component(&document, "tilling").get("hashes").is_none());
    }

    #[test]
    fn license_list_marks_components_that_declare_none() {
        let mut workspace = workspace();
        workspace.components.first_mut().unwrap().license = Some("MIT/Apache-2.0".to_owned());

        assert_eq!(
            licenses(&workspace),
            "cc 1.2.0: MIT OR Apache-2.0\n\
             serde 1.0.229: MIT OR Apache-2.0\n\
             tilling 0.1.0: GPL-3.0-only\n\
             tillingctl 0.1.0: GPL-3.0-only\n\
             unlicensed 0.3.0: license not declared\n"
        );
    }
}
