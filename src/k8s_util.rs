use std::collections::BTreeMap;

pub(crate) fn annotation_true(annotations: &BTreeMap<String, String>, search: &str) -> bool {
    annotations
        .get(search)
        .map(|value| value.to_lowercase().starts_with('t'))
        .unwrap_or(false)
}

#[allow(unused)]
pub(crate) fn annotation_maybe_int(
    annotations: &BTreeMap<String, String>,
    search: &str,
) -> Option<u32> {
    annotations
        .get(search)
        .map(|value| value.parse::<u32>().map(Some).unwrap_or_else(|_| None))
        .unwrap_or(None)
}
