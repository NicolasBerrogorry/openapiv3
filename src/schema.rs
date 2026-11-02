use std::str::FromStr;

use crate::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaData {
    #[serde(default, skip_serializing_if = "is_false")]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub write_only: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<Discriminator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// A `$ref` carried beside other keywords: in JSON Schema 2020-12 (OpenAPI
    /// 3.1) one keyword among the others, the siblings kept. A bare `$ref` is a
    /// [ReferenceOr::Reference], never a schema.
    #[serde(rename = "$ref", default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "$id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "$anchor", default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(rename = "$comment", default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// The `const` keyword: the one value the instance may be.
    #[serde(rename = "const", default, skip_serializing_if = "Option::is_none")]
    pub const_value: Option<serde_json::Value>,
    /// The `examples` keyword (3.1); the singular `example` is above.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<serde_json::Value>,
    /// The `$defs` keyword: schemas the schema defines for its own `$ref`s.
    #[serde(rename = "$defs", default, skip_serializing_if = "IndexMap::is_empty")]
    pub defs: IndexMap<String, ReferenceOr<Schema>>,
    #[serde(rename = "if", default, skip_serializing_if = "Option::is_none")]
    pub if_schema: Option<Box<ReferenceOr<Schema>>>,
    #[serde(rename = "then", default, skip_serializing_if = "Option::is_none")]
    pub then_schema: Option<Box<ReferenceOr<Schema>>>,
    #[serde(rename = "else", default, skip_serializing_if = "Option::is_none")]
    pub else_schema: Option<Box<ReferenceOr<Schema>>>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub pattern_properties: IndexMap<String, ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dependent_schemas: IndexMap<String, ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dependent_required: IndexMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unevaluated_properties: Option<BoolOrSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unevaluated_items: Option<BoolOrSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    /// Inline extensions to this object.
    #[serde(
        flatten,
        deserialize_with = "crate::util::deserialize_schema_extensions"
    )]
    pub extensions: IndexMap<String, serde_json::Value>,
}

/// A JSON Schema type name. OpenAPI 3.1 (JSON Schema 2020-12) adds `null`; a
/// name outside the set is kept as it was written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaType {
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
    Null,
    Other(String),
}

impl SchemaType {
    pub fn from_name(name: &str) -> Self {
        match name {
            "boolean" => Self::Boolean,
            "integer" => Self::Integer,
            "number" => Self::Number,
            "string" => Self::String,
            "array" => Self::Array,
            "object" => Self::Object,
            "null" => Self::Null,
            other => Self::Other(other.to_string()),
        }
    }

    /// The name as the `type` keyword writes it.
    pub fn name(&self) -> &str {
        match self {
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::Null => "null",
            Self::Other(name) => name,
        }
    }
}

impl Serialize for SchemaType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for SchemaType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|name| Self::from_name(&name))
    }
}

/// The `type` keyword in either form: one name, as 3.0 requires, or the set of
/// names 3.1 allows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeSet {
    Single(SchemaType),
    Multiple(Vec<SchemaType>),
}

impl TypeSet {
    /// Whether the keyword is exactly this one name.
    pub fn is(&self, schema_type: SchemaType) -> bool {
        matches!(self, Self::Single(t) if *t == schema_type)
    }

    /// Whether the keyword names this type.
    pub fn contains(&self, schema_type: &SchemaType) -> bool {
        match self {
            Self::Single(t) => t == schema_type,
            Self::Multiple(types) => types.contains(schema_type),
        }
    }

    /// Every type the keyword names, in document order.
    pub fn iter(&self) -> impl Iterator<Item = &SchemaType> {
        match self {
            Self::Single(t) => std::slice::from_ref(t).iter(),
            Self::Multiple(types) => types.iter(),
        }
    }

    /// The one type the keyword names besides `null`, if there is exactly one:
    /// `"string"` and `["string", "null"]` both read as `String`.
    pub fn single(&self) -> Option<&SchemaType> {
        let mut non_null = self.iter().filter(|t| **t != SchemaType::Null);
        match (non_null.next(), non_null.next()) {
            (Some(t), None) => Some(t),
            _ => None,
        }
    }
}

/// An exclusive bound in either form: in 3.0 a flag that makes `minimum` or
/// `maximum` exclusive, in 3.1 the bound itself, kept as written.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ExclusiveBound {
    Flag(bool),
    Value(serde_json::Number),
}

impl ExclusiveBound {
    /// Whether the bound excludes its value: a set flag, or a value.
    pub fn is_exclusive(&self) -> bool {
        !matches!(self, Self::Flag(false))
    }
}

/// A schema, or (JSON Schema 2020-12) a boolean schema: `true` admits
/// everything, `false` nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BoolOrSchema {
    Schema(Box<ReferenceOr<Schema>>),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    #[serde(flatten)]
    pub schema_data: SchemaData,
    #[serde(flatten)]
    pub schema_kind: SchemaKind,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum SchemaKind {
    Type(Type),
    OneOf {
        #[serde(rename = "oneOf")]
        one_of: Vec<ReferenceOr<Schema>>,
    },
    AllOf {
        #[serde(rename = "allOf")]
        all_of: Vec<ReferenceOr<Schema>>,
    },
    AnyOf {
        #[serde(rename = "anyOf")]
        any_of: Vec<ReferenceOr<Schema>>,
    },
    Not {
        not: Box<ReferenceOr<Schema>>,
    },
    Any(AnySchema),
}

// Custom Deserialize implementation that is similar to the logic for an
// untagged enum with awareness of all the fields we might expect to see. This
// is necessary to ensure that relevant fields aren't ignored e.g. when mixing
// a object with a oneOf/anyOf/allOf. Note that serde's deny_unknown_fields
// doesn't help us here due to its interactions with our abundant use of
// flatten.
impl<'de> Deserialize<'de> for SchemaKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawAnySchema {
            #[serde(rename = "type", default)]
            typ: Option<TypeSet>,
            #[serde(default)]
            pattern: Option<String>,
            #[serde(default)]
            multiple_of: Option<serde_json::Number>,
            #[serde(default)]
            exclusive_minimum: Option<ExclusiveBound>,
            #[serde(default)]
            exclusive_maximum: Option<ExclusiveBound>,
            #[serde(default)]
            minimum: Option<serde_json::Number>,
            #[serde(default)]
            maximum: Option<serde_json::Number>,
            #[serde(default)]
            properties: Option<IndexMap<String, ReferenceOr<Box<Schema>>>>,
            #[serde(default)]
            required: Option<Vec<String>>,
            #[serde(default)]
            additional_properties: Option<AdditionalProperties>,
            #[serde(default)]
            min_properties: Option<usize>,
            #[serde(default)]
            max_properties: Option<usize>,
            #[serde(default)]
            items: Option<BoolOrSchema>,
            #[serde(default)]
            prefix_items: Option<Vec<ReferenceOr<Schema>>>,
            #[serde(default)]
            contains: Option<Box<ReferenceOr<Schema>>>,
            #[serde(default)]
            min_contains: Option<usize>,
            #[serde(default)]
            max_contains: Option<usize>,
            #[serde(default)]
            min_items: Option<usize>,
            #[serde(default)]
            max_items: Option<usize>,
            #[serde(default)]
            unique_items: Option<bool>,
            #[serde(rename = "enum", default)]
            enumeration: Option<Vec<serde_json::Value>>,
            #[serde(default)]
            format: Option<String>,
            #[serde(default)]
            min_length: Option<usize>,
            #[serde(default)]
            max_length: Option<usize>,
            #[serde(default)]
            one_of: Option<Vec<ReferenceOr<Schema>>>,
            #[serde(default)]
            all_of: Option<Vec<ReferenceOr<Schema>>>,
            #[serde(default)]
            any_of: Option<Vec<ReferenceOr<Schema>>>,
            #[serde(default)]
            not: Option<Box<ReferenceOr<Schema>>>,
        }

        let any = RawAnySchema::deserialize(deserializer)?;
        match any {
            // String
            RawAnySchema {
                typ: Some(typ),
                pattern,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration,
                format,
                min_length,
                max_length,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::String)
                && enumerated_values_valid(&enumeration, serde_json::Value::is_string) =>
            {
                Ok(Self::Type(Type::String(StringType {
                    format: format.into(),
                    pattern,
                    enumeration: enumerated_values_transform(enumeration, |v| {
                        v.as_str().map(String::from)
                    }),
                    min_length,
                    max_length,
                })))
            }

            // Number
            RawAnySchema {
                typ: Some(typ),
                pattern: None,
                multiple_of,
                exclusive_minimum,
                exclusive_maximum,
                minimum,
                maximum,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration,
                format,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::Number)
                && enumerated_values_valid(&enumeration, serde_json::Value::is_number) =>
            {
                Ok(Self::Type(Type::Number(NumberType {
                    format: format.into(),
                    multiple_of: multiple_of.map(|v| v.as_f64().unwrap()),
                    exclusive_minimum,
                    exclusive_maximum,
                    minimum: minimum.map(|v| v.as_f64().unwrap()),
                    maximum: maximum.map(|v| v.as_f64().unwrap()),
                    enumeration: enumerated_values_transform(
                        enumeration,
                        serde_json::Value::as_f64,
                    ),
                })))
            }

            // Integer
            RawAnySchema {
                typ: Some(typ),
                pattern: None,
                multiple_of,
                exclusive_minimum,
                exclusive_maximum,
                minimum,
                maximum,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration,
                format,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::Integer)
                && enumerated_values_valid(&enumeration, serde_json::Value::is_i64)
                && none_or_int(&multiple_of)
                && none_or_int(&minimum)
                && none_or_int(&maximum)
                && none_or_int_bound(&exclusive_minimum)
                && none_or_int_bound(&exclusive_maximum) =>
            {
                Ok(Self::Type(Type::Integer(IntegerType {
                    format: format.into(),
                    multiple_of: multiple_of.map(|v| v.as_i64().unwrap()),
                    exclusive_minimum,
                    exclusive_maximum,
                    minimum: minimum.map(|v| v.as_i64().unwrap()),
                    maximum: maximum.map(|v| v.as_i64().unwrap()),
                    enumeration: enumerated_values_transform(
                        enumeration,
                        serde_json::Value::as_i64,
                    ),
                })))
            }

            // Boolean
            RawAnySchema {
                typ: Some(typ),
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::Boolean)
                && enumerated_values_valid(&enumeration, serde_json::Value::is_boolean) =>
            {
                Ok(Self::Type(Type::Boolean(BooleanType {
                    enumeration: enumerated_values_transform(
                        enumeration,
                        serde_json::Value::as_bool,
                    ),
                })))
            }

            // Object
            RawAnySchema {
                typ: Some(typ),
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties,
                required,
                additional_properties,
                min_properties,
                max_properties,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::Object) => Ok(Self::Type(Type::Object(ObjectType {
                properties: properties.unwrap_or_default(),
                required: required.unwrap_or_default(),
                additional_properties,
                min_properties,
                max_properties,
            }))),

            // Array
            RawAnySchema {
                typ: Some(typ),
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items,
                prefix_items,
                contains,
                min_contains,
                max_contains,
                min_items,
                max_items,
                unique_items,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: None,
            } if typ.is(SchemaType::Array) => Ok(Self::Type(Type::Array(ArrayType {
                items,
                prefix_items: prefix_items.unwrap_or_default(),
                contains,
                min_contains,
                max_contains,
                min_items,
                max_items,
                unique_items: unique_items.unwrap_or_default(),
            }))),

            // OneOf
            RawAnySchema {
                typ: None,
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: Some(one_of),
                all_of: None,
                any_of: None,
                not: None,
            } => Ok(Self::OneOf { one_of }),

            // AllOf
            RawAnySchema {
                typ: None,
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: Some(all_of),
                any_of: None,
                not: None,
            } => Ok(Self::AllOf { all_of }),

            // AnyOf
            RawAnySchema {
                typ: None,
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: Some(any_of),
                not: None,
            } => Ok(Self::AnyOf { any_of }),

            // Not
            RawAnySchema {
                typ: None,
                pattern: None,
                multiple_of: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
                minimum: None,
                maximum: None,
                properties: None,
                required: None,
                additional_properties: None,
                min_properties: None,
                max_properties: None,
                items: None,
                prefix_items: None,
                contains: None,
                min_contains: None,
                max_contains: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                enumeration: None,
                format: None,
                min_length: None,
                max_length: None,
                one_of: None,
                all_of: None,
                any_of: None,
                not: Some(not),
            } => Ok(Self::Not { not }),

            // Any
            RawAnySchema {
                typ,
                pattern,
                multiple_of,
                exclusive_minimum,
                exclusive_maximum,
                minimum,
                maximum,
                properties,
                required,
                additional_properties,
                min_properties,
                max_properties,
                items,
                prefix_items,
                contains,
                min_contains,
                max_contains,
                min_items,
                max_items,
                unique_items,
                enumeration,
                format,
                min_length,
                max_length,
                one_of,
                all_of,
                any_of,
                not,
            } => Ok(Self::Any(AnySchema {
                typ,
                pattern,
                multiple_of: multiple_of.map(|n| n.as_f64().unwrap()),
                exclusive_minimum,
                exclusive_maximum,
                minimum: minimum.map(|n| n.as_f64().unwrap()),
                maximum: maximum.map(|n| n.as_f64().unwrap()),
                properties: properties.unwrap_or_default(),
                required: required.unwrap_or_default(),
                additional_properties,
                min_properties,
                max_properties,
                items,
                prefix_items: prefix_items.unwrap_or_default(),
                contains,
                min_contains,
                max_contains,
                min_items,
                max_items,
                unique_items,
                enumeration: enumeration.unwrap_or_default(),
                format,
                min_length,
                max_length,
                one_of: one_of.unwrap_or_default(),
                all_of: all_of.unwrap_or_default(),
                any_of: any_of.unwrap_or_default(),
                not,
            })),
        }
    }
}

fn none_or_int_bound(value: &Option<ExclusiveBound>) -> bool {
    match value {
        Some(ExclusiveBound::Value(x)) => x.is_i64(),
        _ => true,
    }
}

fn none_or_int(value: &Option<serde_json::Number>) -> bool {
    match value {
        None => true,
        Some(x) => x.is_i64(),
    }
}

fn enumerated_values_transform<T, F>(
    enumeration: Option<Vec<serde_json::Value>>,
    transform: F,
) -> Vec<Option<T>>
where
    F: Fn(&serde_json::Value) -> Option<T>,
{
    match enumeration {
        Some(values) => values
            .iter()
            .map(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(transform(v).unwrap())
                }
            })
            .collect::<Vec<_>>(),
        None => Default::default(),
    }
}

fn enumerated_values_valid<F>(enumeration: &Option<Vec<serde_json::Value>>, check: F) -> bool
where
    F: Fn(&serde_json::Value) -> bool,
{
    match enumeration {
        Some(values) => values.iter().all(|value| value.is_null() || check(value)),
        None => true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Type {
    String(StringType),
    Number(NumberType),
    Integer(IntegerType),
    Object(ObjectType),
    Array(ArrayType),
    Boolean(BooleanType),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Any(bool),
    Schema(Box<ReferenceOr<Schema>>),
}

/// Catch-all for any combination of properties that doesn't correspond to one
/// of the predefined subsets.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnySchema {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub typ: Option<TypeSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<ExclusiveBound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<ExclusiveBound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, ReferenceOr<Box<Schema>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<BoolOrSchema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_items: Vec<ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<Box<ReferenceOr<Schema>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_contains: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_contains: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_of: Vec<ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<ReferenceOr<Schema>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StringType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<StringFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NumberType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<NumberFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<ExclusiveBound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<ExclusiveBound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntegerType {
    #[serde(default, skip_serializing_if = "VariantOrUnknownOrEmpty::is_empty")]
    pub format: VariantOrUnknownOrEmpty<IntegerFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<ExclusiveBound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<ExclusiveBound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectType {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, ReferenceOr<Box<Schema>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArrayType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<BoolOrSchema>,
    /// The schemas of the leading items (3.1); `items` then covers the rest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_items: Vec<ReferenceOr<Schema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<Box<ReferenceOr<Schema>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_contains: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_contains: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique_items: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BooleanType {
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enumeration: Vec<Option<bool>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    Float,
    Double,
}

impl FromStr for NumberFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "float" => Ok(Self::Float),
            "double" => Ok(Self::Double),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntegerFormat {
    Int32,
    Int64,
}

impl FromStr for IntegerFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "int32" => Ok(Self::Int32),
            "int64" => Ok(Self::Int64),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    Date,
    DateTime,
    Password,
    Byte,
    Binary,
}

impl FromStr for StringFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "date" => Ok(Self::Date),
            "date-time" => Ok(Self::DateTime),
            "password" => Ok(Self::Password),
            "byte" => Ok(Self::Byte),
            "binary" => Ok(Self::Binary),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        AnySchema, Schema, SchemaData, SchemaKind, StringType, Type, VariantOrUnknownOrEmpty,
    };

    #[test]
    fn test_schema_with_extensions() {
        let schema = serde_json::from_str::<Schema>(
            r#"{
                "type": "boolean",
                "x-foo": "bar"
            }"#,
        )
        .unwrap();

        assert_eq!(
            schema.schema_data.extensions.get("x-foo"),
            Some(&json!("bar"))
        );
    }

    #[test]
    fn test_any() {
        let value = json! { {} };
        serde_json::from_value::<AnySchema>(value).unwrap();
    }

    #[test]
    fn test_not() {
        let value = json! {
            {
                "not": {}
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(matches!(schema.schema_kind, SchemaKind::Not { not: _ }));
    }

    #[test]
    fn test_null() {
        let value = json! {
            {
                "nullable": true,
                "enum": [ null ],
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(matches!(
            &schema.schema_data,
            SchemaData { nullable: true, .. }
        ));
        assert!(matches!(
            &schema.schema_kind,
            SchemaKind::Any(AnySchema { enumeration, .. }) if enumeration[0] == json!(null)));
    }

    #[test]
    fn test_object_and_one_of() {
        let value = json! {
            {
                "type": "object",
                "nullable": true,
                "description": "xyz",
                "properties": {
                    "a": {},
                    "b": {},
                    "c": {}
                },
                "oneOf": [
                    { "required": ["a"] },
                    { "required": ["b"] },
                    { "required": ["c"] }
                ],
                "x-foo": "bar"
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(schema.schema_data.nullable);
        assert_eq!(schema.schema_data.extensions.get("x-foo").unwrap(), "bar");

        match schema.schema_kind {
            SchemaKind::Any(AnySchema {
                typ,
                properties,
                one_of,
                ..
            }) => {
                assert_eq!(
                    typ.unwrap(),
                    crate::TypeSet::Single(crate::SchemaType::Object)
                );
                assert_eq!(properties.len(), 3);
                assert_eq!(one_of.len(), 3);
            }
            _ => panic!("incorrect kind {:#?}", schema),
        }
    }

    #[test]
    fn test_enum_with_null() {
        let value = json! {
            {
                "type": "string",
                "nullable": true,
                "enum": [ null, "howdy" ]
            }
        };

        let schema = serde_json::from_value::<Schema>(value).unwrap();
        assert!(schema.schema_data.nullable);

        match schema.schema_kind {
            SchemaKind::Type(Type::String(StringType {
                format: VariantOrUnknownOrEmpty::Empty,
                pattern: None,
                enumeration,
                min_length: None,
                max_length: None,
            })) => {
                assert_eq!(enumeration, vec![None, Some("howdy".to_string())]);
            }
            _ => panic!("incorrect kind {:#?}", schema),
        }
    }
}
