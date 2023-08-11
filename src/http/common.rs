use std::{fmt::Display, hash::Hash};

use crate::error::Error;

#[derive(Debug, Clone, Default)]
pub struct RequestHeader {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub fields: HeaderFields,
}

impl RequestHeader {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        super::parse::parse_request_header(data)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseHeader {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub fields: HeaderFields,
}

impl ResponseHeader {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        super::parse::parse_response_header(data)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseTrailer {
    pub fields: HeaderFields,
}

impl ResponseTrailer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        super::parse::parse_response_trailer(data)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HeaderFields {
    inner: Vec<(FieldName, FieldValue)>,
}

impl HeaderFields {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn as_slice(&self) -> &[(FieldName, FieldValue)] {
        &self.inner
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (FieldName, FieldValue)> {
        self.inner.iter()
    }

    pub fn contains_key<K: Into<FieldName>>(&self, key: K) -> bool {
        let key = key.into();
        self.inner.iter().any(|(k, _v)| k == &key)
    }

    pub fn get<K: Into<FieldName>>(&self, key: K) -> Option<&FieldValue> {
        let key = key.into();
        self.inner
            .iter()
            .find_map(|(k, v)| if k == &key { Some(v) } else { None })
    }

    pub fn get_all<K: Into<FieldName>>(&self, key: K) -> impl Iterator<Item = &FieldValue> {
        let key = key.into();
        self.inner
            .iter()
            .filter_map(move |(k, v)| if k == &key { Some(v) } else { None })
    }

    pub fn insert<K: Into<FieldName>, V: Into<FieldValue>>(&mut self, key: K, value: V) {
        let key = key.into();

        if let Some(position) = self.inner.iter().position(|(k, _v)| k == &key) {
            self.remove(key.clone());
            self.inner.insert(position, (key, value.into()));
        } else {
            self.append(key, value);
        }
    }

    pub fn append<K: Into<FieldName>, V: Into<FieldValue>>(&mut self, key: K, value: V) {
        self.inner.push((key.into(), value.into()));
    }

    pub fn remove<K: Into<FieldName>>(&mut self, key: K) {
        let key = key.into();
        self.inner.retain(|(k, _v)| k != &key);
    }
}

impl IntoIterator for HeaderFields {
    type Item = (FieldName, FieldValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a HeaderFields {
    type Item = &'a (FieldName, FieldValue);
    type IntoIter = std::slice::Iter<'a, (FieldName, FieldValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl Extend<(FieldName, FieldValue)> for HeaderFields {
    fn extend<T: IntoIterator<Item = (FieldName, FieldValue)>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

#[derive(Debug, Clone)]
pub struct FieldName {
    inner: String,
    normalized: String,
}

impl FieldName {
    pub fn new<T: Into<String>>(name: T) -> Self {
        let inner = name.into();
        let normalized = inner.to_ascii_lowercase();

        Self { inner, normalized }
    }

    pub fn into_inner(self) -> String {
        self.inner
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn normalized(&self) -> &str {
        &self.normalized
    }
}

impl Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner)
    }
}

impl From<String> for FieldName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for FieldName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&FieldName> for FieldName {
    fn from(value: &FieldName) -> Self {
        value.to_owned()
    }
}

impl PartialEq for FieldName {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for FieldName {}

impl Hash for FieldName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldValue {
    Text(String),
    Opaque(Vec<u8>),
}

impl FieldValue {
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque(_))
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            FieldValue::Text(text) => text.as_bytes(),
            FieldValue::Opaque(data) => data,
        }
    }

    pub fn to_string_lossy(&self) -> String {
        match self {
            FieldValue::Text(text) => text.to_owned(),
            FieldValue::Opaque(data) => crate::string::parse_utf8_escaped(data),
        }
    }
}

impl Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::Text(text) => f.write_str(text),
            FieldValue::Opaque(data) => {
                let text = crate::string::parse_utf8_escaped(data);
                f.write_str(&text)
            }
        }
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&[u8]> for FieldValue {
    fn from(value: &[u8]) -> Self {
        match std::str::from_utf8(value) {
            Ok(text) => Self::Text(text.to_string()),
            Err(_) => Self::Opaque(value.to_vec()),
        }
    }
}

impl From<Vec<u8>> for FieldValue {
    fn from(value: Vec<u8>) -> Self {
        match String::from_utf8(value) {
            Ok(text) => Self::Text(text),
            Err(error) => Self::Opaque(error.into_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_fields() {
        let mut fields = HeaderFields::new();

        assert!(fields.is_empty());
        assert_eq!(fields.len(), 0);

        fields.append("k1", "v1");
        fields.append("k1", "v1-1");
        fields.append("k2", "v2");

        assert!(!fields.is_empty());
        assert_eq!(fields.len(), 3);

        assert_eq!(
            fields.iter().collect::<Vec<&(FieldName, FieldValue)>>(),
            vec![
                &("k1".into(), "v1".into()),
                &("k1".into(), "v1-1".into()),
                &("k2".into(), "v2".into()),
            ]
        );

        assert!(fields.contains_key("k1"));
        assert!(fields.contains_key("K1"));
        assert!(fields.contains_key("k2"));
        assert!(fields.contains_key("K2"));
        assert!(!fields.contains_key("k3"));

        assert_eq!(fields.get("k1"), Some(&"v1".into()));
        assert_eq!(
            fields.get_all("k1").collect::<Vec<&FieldValue>>(),
            vec![&"v1".into(), &"v1-1".into()]
        );

        fields.insert("k1", "v1-2");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields.get("k1"), Some(&"v1-2".into()));

        fields.remove("k1");
        assert_eq!(fields.len(), 1);

        fields.clear();
        assert!(fields.is_empty());
    }
}
