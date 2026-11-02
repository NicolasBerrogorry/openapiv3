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
    // JSON Schema 2020-12 (OpenAPI 3.1)
    "$ref",
    "$id",
    "$anchor",
    "$comment",
    "const",
    "examples",
    "$defs",
    "if",
    "then",
    "else",
    "patternProperties",
    "dependentSchemas",
    "dependentRequired",
    "unevaluatedProperties",
    "unevaluatedItems",
    "contentMediaType",
    "contentEncoding",
    "prefixItems",
    "contains",
    "minContains",
    "maxContains",
];

/// Schema Objects: retain what the typed model does not consume.
pub(crate) fn deserialize_schema_extensions<'de, D>(
    deserializer: D,
) -> Result<IndexMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(PredicateVisitor(
        |key: &String| !SCHEMA_KEYWORDS.contains(&key.as_str()),
        PhantomData,
    ))
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
