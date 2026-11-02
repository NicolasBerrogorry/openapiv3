//! OpenAPI 3.1 documents read without loss.
//!
//! What 3.1 adds is typed like what 3.0 has: a `type` set, a numeric exclusive
//! bound, a boolean `items`, the JSON Schema 2020-12 keywords, `webhooks`.
//! What the model does not type is retained verbatim in each object's
//! `extensions`. Either way the JSON that comes out is the JSON that went in;
//! these tests pin that.

use openapiv3::*;
use serde_json::{json, Value};

/// Serialization elides values the model treats as defaults (`false`, empty
/// collections), always writes a parameter's `style` even when the document
/// left the spec default implicit, and writes every number through f64, so
/// equality is checked on that canonical form of both sides. Nothing else is
/// normalized.
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !matches!(v, Value::Bool(false)))
                .filter(|(_, v)| !matches!(v, Value::Array(a) if a.is_empty()))
                .filter(|(_, v)| !matches!(v, Value::Object(o) if o.is_empty()))
                .filter(|(k, v)| {
                    !(*k == "style"
                        && matches!(v, Value::String(s) if s == "simple" || s == "form"))
                })
                .map(|(k, v)| (k.clone(), canonical(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        Value::Number(n) => json!(n.as_f64().expect("a JSON number reads as f64")),
        other => other.clone(),
    }
}

/// The JSON-pointer-ish path of the first difference between two values, for
/// a failure message that names the spot instead of printing two documents.
fn first_diff(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    None => return Some(format!("{path}/{k}: only on the left: {v}")),
                    Some(w) => {
                        if let Some(d) = first_diff(v, w, &format!("{path}/{k}")) {
                            return Some(d);
                        }
                    }
                }
            }
            y.keys()
                .find(|k| !x.contains_key(*k))
                .map(|k| format!("{path}/{k}: only on the right: {}", y[k]))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: {} items vs {}", x.len(), y.len()));
            }
            x.iter()
                .zip(y)
                .enumerate()
                .find_map(|(i, (v, w))| first_diff(v, w, &format!("{path}/{i}")))
        }
        _ => (a != b).then(|| format!("{path}: {a} vs {b}")),
    }
}

fn assert_same(back: &Value, original: &Value) {
    if let Some(diff) = first_diff(&canonical(back), &canonical(original), "") {
        panic!("round trip differs (serialized vs original) at {diff}");
    }
}

fn schema(input: Value) -> Schema {
    serde_json::from_value(input).expect("a 3.1 schema object parses")
}

fn round_trips(input: Value) {
    let parsed = schema(input.clone());
    let back = serde_json::to_value(&parsed).expect("serializes");
    assert_same(&back, &input);
}

#[test]
fn petstore_3_1_parses_and_round_trips() {
    let raw = include_str!("../fixtures/petstore31.json");
    let doc: OpenAPI = serde_json::from_str(raw).expect("the 3.1 petstore parses");
    assert_eq!(doc.openapi, "3.1.0");
    assert_eq!(doc.paths.paths.len(), 2);
    assert!(!doc.webhooks.is_empty());

    let original: Value = serde_json::from_str(raw).unwrap();
    let back = serde_json::to_value(&doc).unwrap();
    assert_same(&back, &original);
}

#[test]
fn a_type_set_is_typed() {
    let input = json!({ "type": ["string", "null"], "maxLength": 5, "description": "maybe" });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Any(any) => {
            let types = any.typ.as_ref().unwrap();
            assert_eq!(
                *types,
                TypeSet::Multiple(vec![SchemaType::String, SchemaType::Null])
            );
            assert!(types.contains(&SchemaType::Null));
            assert_eq!(types.single(), Some(&SchemaType::String));
            assert_eq!(any.max_length, Some(5));
        }
        other => panic!("expected the untyped kind, got {other:?}"),
    }
    assert!(!parsed.schema_data.extensions.contains_key("type"));
    round_trips(input);
    // A single 3.1 `null`, and a name outside the set, both read as written.
    match &schema(json!({ "type": "null" })).schema_kind {
        SchemaKind::Any(any) => assert_eq!(any.typ, Some(TypeSet::Single(SchemaType::Null))),
        other => panic!("expected the untyped kind, got {other:?}"),
    }
    match &schema(json!({ "type": "file" })).schema_kind {
        SchemaKind::Any(any) => assert_eq!(
            any.typ,
            Some(TypeSet::Single(SchemaType::Other("file".into())))
        ),
        other => panic!("expected the untyped kind, got {other:?}"),
    }
    round_trips(json!({ "type": "file" }));
}

#[test]
fn a_numeric_exclusive_bound_is_typed() {
    let input = json!({ "type": "number", "exclusiveMinimum": 5, "maximum": 10 });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Number(number)) => {
            assert_eq!(
                number.exclusive_minimum,
                Some(ExclusiveBound::Value(5.into()))
            );
            assert!(number.exclusive_minimum.as_ref().unwrap().is_exclusive());
            assert_eq!(number.exclusive_maximum, None);
            assert_eq!(number.minimum, None);
            assert_eq!(number.maximum, Some(10.0));
        }
        other => panic!("expected a number, got {other:?}"),
    }
    assert!(!parsed
        .schema_data
        .extensions
        .contains_key("exclusiveMinimum"));
    round_trips(input);

    let input = json!({ "type": "integer", "exclusiveMaximum": 3 });
    match &schema(input.clone()).schema_kind {
        SchemaKind::Type(Type::Integer(integer)) => {
            assert_eq!(
                integer.exclusive_maximum,
                Some(ExclusiveBound::Value(3.into()))
            )
        }
        other => panic!("expected an integer, got {other:?}"),
    }
    round_trips(input);
    // An integer with a fractional exclusive bound is not an integer schema.
    match &schema(json!({ "type": "integer", "exclusiveMaximum": 3.5 })).schema_kind {
        SchemaKind::Any(any) => {
            assert_eq!(
                any.exclusive_maximum,
                Some(ExclusiveBound::Value(
                    serde_json::Number::from_f64(3.5).unwrap()
                ))
            )
        }
        other => panic!("expected the untyped kind, got {other:?}"),
    }
}

#[test]
fn a_boolean_items_and_prefix_items_are_typed() {
    let input = json!({ "type": "array", "items": false, "prefixItems": [{ "type": "string" }] });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Array(array)) => {
            assert_eq!(array.items, Some(BoolOrSchema::Bool(false)));
            assert_eq!(array.prefix_items.len(), 1);
            assert!(matches!(
                &array.prefix_items[0],
                ReferenceOr::Item(item) if matches!(item.schema_kind, SchemaKind::Type(Type::String(_)))
            ));
        }
        other => panic!("expected an array, got {other:?}"),
    }
    round_trips(input);

    let input = json!({ "type": "array", "items": { "type": "integer" }, "contains": { "const": 1 }, "minContains": 1 });
    match &schema(input.clone()).schema_kind {
        SchemaKind::Type(Type::Array(array)) => {
            assert!(matches!(array.items, Some(BoolOrSchema::Schema(_))));
            assert!(array.contains.is_some());
            assert_eq!(array.min_contains, Some(1));
        }
        other => panic!("expected an array, got {other:?}"),
    }
    round_trips(input);
}

#[test]
fn json_schema_2020_12_vocabulary_is_typed_and_round_trips() {
    let input = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.com/thing",
        "type": "object",
        "const": { "fixed": true },
        "examples": [{ "a": 1 }],
        "$defs": { "Inner": { "type": "string", "const": "x" } },
        "patternProperties": { "^x-": { "type": "string" } },
        "unevaluatedProperties": false,
        "if": { "required": ["a"] },
        "then": { "required": ["b"] },
        "contentMediaType": "application/json",
        "dependentRequired": { "a": ["b"] }
    });
    let parsed = schema(input.clone());
    let data = &parsed.schema_data;
    assert_eq!(data.id.as_deref(), Some("https://example.com/thing"));
    assert_eq!(data.const_value, Some(json!({ "fixed": true })));
    assert_eq!(data.examples, vec![json!({ "a": 1 })]);
    assert!(
        matches!(&data.defs["Inner"], ReferenceOr::Item(inner) if inner.schema_data.const_value == Some(json!("x")))
    );
    assert_eq!(data.pattern_properties.len(), 1);
    assert_eq!(data.unevaluated_properties, Some(BoolOrSchema::Bool(false)));
    assert!(data.if_schema.is_some());
    assert!(data.then_schema.is_some());
    assert!(data.else_schema.is_none());
    assert_eq!(data.content_media_type.as_deref(), Some("application/json"));
    assert_eq!(data.dependent_required["a"], vec!["b".to_string()]);
    // `$schema` is not modeled and stays an extension.
    assert_eq!(
        data.extensions["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
    round_trips(input);
}

#[test]
fn a_ref_with_siblings_is_a_schema_carrying_the_ref() {
    // 2020-12 semantics: $ref is one keyword among others. The siblings a
    // consumer depends on (writeOnly, description) stay with it.
    let input =
        json!({ "$ref": "#/components/schemas/Pet", "writeOnly": true, "description": "owner" });
    let parsed: ReferenceOr<Schema> = serde_json::from_value(input.clone()).unwrap();
    let ReferenceOr::Item(item) = &parsed else {
        panic!("expected an item, got {parsed:?}")
    };
    assert!(item.schema_data.write_only);
    assert_eq!(
        item.schema_data.reference.as_deref(),
        Some("#/components/schemas/Pet")
    );
    let back = serde_json::to_value(&parsed).unwrap();
    assert_eq!(canonical(&back), canonical(&input));
}

#[test]
fn a_bare_ref_is_still_a_reference() {
    let parsed: ReferenceOr<Schema> =
        serde_json::from_value(json!({ "$ref": "#/components/schemas/Pet" })).unwrap();
    assert_eq!(parsed, ReferenceOr::ref_("#/components/schemas/Pet"));
}

#[test]
fn a_ref_with_siblings_on_a_non_schema_object_stays_a_reference() {
    // A Parameter cannot carry $ref as a keyword; the 3.0 reading (siblings dropped) holds.
    let parsed: ReferenceOr<Parameter> = serde_json::from_value(
        json!({ "$ref": "#/components/parameters/Limit", "description": "x" }),
    )
    .unwrap();
    assert_eq!(parsed, ReferenceOr::ref_("#/components/parameters/Limit"));
}

#[test]
fn a_3_1_document_may_omit_paths_and_keeps_its_envelope() {
    let input = json!({
        "openapi": "3.1.0",
        "info": { "title": "Hooks", "version": "1", "summary": "only webhooks" },
        "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema",
        "webhooks": { "newPet": { "post": { "responses": { "200": { "description": "ok" } } } } }
    });
    let doc: OpenAPI =
        serde_json::from_value(input.clone()).expect("a paths-less 3.1 document parses");
    assert!(doc.paths.paths.is_empty());
    assert_eq!(doc.info.summary.as_deref(), Some("only webhooks"));
    assert_eq!(
        doc.json_schema_dialect.as_deref(),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert!(matches!(&doc.webhooks["newPet"], ReferenceOr::Item(item) if item.post.is_some()));
    let back = serde_json::to_value(&doc).unwrap();
    assert_eq!(canonical(&back), canonical(&input));
}

#[test]
fn the_3_0_dialect_is_untouched() {
    let parsed = schema(json!({ "type": "string", "nullable": true, "enum": ["a", null] }));
    assert!(parsed.schema_data.nullable);
    match &parsed.schema_kind {
        SchemaKind::Type(Type::String(s)) => {
            assert_eq!(s.enumeration, vec![Some("a".to_string()), None])
        }
        other => panic!("expected a string, got {other:?}"),
    }
    let parsed = schema(json!({ "type": "number", "minimum": 1, "exclusiveMinimum": true }));
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Number(n)) => {
            assert_eq!(n.exclusive_minimum, Some(ExclusiveBound::Flag(true)));
            assert_eq!(n.minimum, Some(1.0));
        }
        other => panic!("expected a number, got {other:?}"),
    }
    // Vendor extensions keep working everywhere.
    round_trips(
        json!({ "type": "object", "x-internal": true, "properties": { "a": { "type": "string", "x-note": 1 } } }),
    );
}
