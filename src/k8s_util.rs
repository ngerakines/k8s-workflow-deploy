use std::collections::BTreeMap;

pub(crate) fn annotation_true(annotations: &BTreeMap<String, String>, search: &str) -> bool {
    annotations
        .get(search)
        .map(|value| value.to_lowercase().starts_with('t'))
        .unwrap_or(false)
}
