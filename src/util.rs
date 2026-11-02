use std::hash::Hash;
use std::marker::PhantomData;

use indexmap::IndexMap;
use serde::{
    de::{IgnoredAny, Visitor},
    Deserialize, Deserializer,
};

#[allow(clippy::trivially_copy_pass_by_ref)] // needs to match signature for use in serde attribute
#[inline]
pub const fn is_false(v: &bool) -> bool {
    !(*v)
}

/// Every entry the object's typed fields did not consume, verbatim. For an
/// object whose only flattened field is this map, that is exactly the vocabulary
/// the model does not know: `x-` extensions, and everything OpenAPI 3.1 added
/// (`webhooks`, `jsonSchemaDialect`, a `summary` on Info, …). Carrying it
/// unchanged is what lets a 3.1 document round-trip through a 3.0 model.
pub(crate) fn deserialize_extensions<'de, D>(
    deserializer: D,
) -> Result<IndexMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(PredicateVisitor(|_: &String| true, PhantomData))
}

/// Only `x-` extensions. For objects with a second flattened field that
/// consumes keywords after this map has run (a Paths or Responses map, a
/// parameter's schema/content, a link's operation), retaining everything would
/// duplicate those keywords on serialization.
pub(crate) fn deserialize_x_extensions<'de, D>(
    deserializer: D,
) -> Result<IndexMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(PredicateVisitor(
        |key: &String| key.starts_with("x-"),
        PhantomData,
    ))
}

/// Keywords the typed Schema model consumes (SchemaData and SchemaKind). A key
/// outside this list is JSON Schema vocabulary the model does not type — `const`,
/// `$defs`, `prefixItems`, `examples`, `if`/`then`/`else`, `$ref` beside other
/// keywords — and is retained verbatim.
const SCHEMA_KEYWORDS: &[&str] = &[
    "nullable",
    "readOnly",
    "writeOnly",
    "deprecated",
    "externalDocs",
    "example",
    "title",
    "description",
    "discriminator",
    "default",
    "type",
    "pattern",
    "multipleOf",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minimum",
    "maximum",
    "properties",
    "required",
    "additionalProperties",
    "minProperties",
    "maxProperties",
    "items",
    "minItems",
    "maxItems",
    "uniqueItems",
    "enum",
    "format",
    "minLength",
    "maxLength",
    "oneOf",
    "allOf",
    "anyOf",
    "not",
];

/// The OpenAPI 3.1 forms of modeled keywords that the 3.0 model cannot hold:
/// a `type` array, a numeric exclusive bound, a boolean `items`. The typed side
/// reads them as absent (see `schema.rs`); the map keeps them, so they
/// serialize back exactly and nothing is lost.
fn is_31_form(key: &str, value: &serde_json::Value) -> bool {
    match key {
        "type" => value.is_array(),
        "exclusiveMinimum" | "exclusiveMaximum" => value.is_number(),
        "items" => value.is_boolean(),
        _ => false,
    }
}

/// Schema Objects: retain what the typed model does not consume, plus the 3.1
/// forms it cannot hold.
pub(crate) fn deserialize_schema_extensions<'de, D>(
    deserializer: D,
) -> Result<IndexMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    fn retain(key: &String, value: &serde_json::Value) -> bool {
        !SCHEMA_KEYWORDS.contains(&key.as_str()) || is_31_form(key, value)
    }
    deserializer.deserialize_map(ValuePredicateVisitor(retain))
}

/// Used to deserialize IndexMap<K, V> that are flattened within other structs.
/// This only adds keys that satisfy the given predicate.
pub(crate) struct PredicateVisitor<F, K, V>(pub F, pub PhantomData<(K, V)>);

impl<'de, F, K, V> Visitor<'de> for PredicateVisitor<F, K, V>
where
    F: Fn(&K) -> bool,
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
{
    type Value = IndexMap<K, V>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map whose fields obey a predicate")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut ret = Self::Value::default();

        loop {
            match map.next_key::<K>() {
                Err(_) => (),
                Ok(None) => break,
                Ok(Some(key)) if self.0(&key) => {
                    let _ = ret.insert(key, map.next_value()?);
                }
                Ok(Some(_)) => {
                    let _ = map.next_value::<IgnoredAny>()?;
                }
            }
        }

        Ok(ret)
    }
}

/// Like [PredicateVisitor], but the predicate also sees the value: needed where
/// the same key is kept or left to the typed fields depending on its shape.
pub(crate) struct ValuePredicateVisitor<F>(pub F);

impl<'de, F> Visitor<'de> for ValuePredicateVisitor<F>
where
    F: Fn(&String, &serde_json::Value) -> bool,
{
    type Value = IndexMap<String, serde_json::Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map whose entries obey a predicate")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut ret = Self::Value::default();

        loop {
            match map.next_key::<String>() {
                Err(_) => (),
                Ok(None) => break,
                Ok(Some(key)) => {
                    let value: serde_json::Value = map.next_value()?;
                    if self.0(&key, &value) {
                        let _ = ret.insert(key, value);
                    }
                }
            }
        }

        Ok(ret)
    }
}
