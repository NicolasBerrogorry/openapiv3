//! OpenAPI 3.1 documents read through the 3.0 model without loss.
//!
//! The model types what 3.0 can say. Everything 3.1 adds, and the 3.1 forms of
//! keywords the model cannot hold, are retained verbatim in each object's
//! `extensions` and serialized back, so a document round-trips. These tests pin
//! that contract: the typed view stays honest (absent, never wrong), and the
//! JSON that comes out is the JSON that went in.

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
                .filter(|(k, v)| !(*k == "style" && matches!(v, Value::String(s) if s == "simple" || s == "form")))
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
    // 3.1 vocabulary at the top level survives beside the typed fields.
    assert!(doc.extensions.contains_key("webhooks"));

    let original: Value = serde_json::from_str(raw).unwrap();
    let back = serde_json::to_value(&doc).unwrap();
    assert_same(&back, &original);
}

#[test]
fn type_array_is_retained_and_the_typed_view_reads_no_single_type() {
    let input = json!({ "type": ["string", "null"], "maxLength": 5, "description": "maybe" });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Any(any) => {
            assert_eq!(any.typ, None);
            assert_eq!(any.max_length, Some(5));
        }
        other => panic!("expected the untyped kind, got {other:?}"),
    }
    assert_eq!(parsed.schema_data.extensions["type"], json!(["string", "null"]));
    round_trips(input);
}

#[test]
fn numeric_exclusive_bounds_are_retained_and_read_as_no_bound() {
    let input = json!({ "type": "number", "exclusiveMinimum": 5, "maximum": 10 });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Number(number)) => {
            assert!(!number.exclusive_minimum);
            assert_eq!(number.minimum, None);
            assert_eq!(number.maximum, Some(10.0));
        }
        other => panic!("expected a number, got {other:?}"),
    }
    assert_eq!(parsed.schema_data.extensions["exclusiveMinimum"], json!(5));
    round_trips(input);
    round_trips(json!({ "type": "integer", "exclusiveMaximum": 3 }));
}

#[test]
fn boolean_items_is_retained() {
    let input = json!({ "type": "array", "items": false, "prefixItems": [{ "type": "string" }] });
    let parsed = schema(input.clone());
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Array(array)) => assert!(array.items.is_none()),
        other => panic!("expected an array, got {other:?}"),
    }
    round_trips(input);
}

#[test]
fn json_schema_2020_12_vocabulary_is_retained_verbatim() {
    round_trips(json!({
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
    }));
}

#[test]
fn a_ref_with_siblings_is_a_schema_carrying_the_ref() {
    // 2020-12 semantics: $ref is one keyword among others. The siblings a
    // consumer depends on (writeOnly, description) stay with it.
    let input = json!({ "$ref": "#/components/schemas/Pet", "writeOnly": true, "description": "owner" });
    let parsed: ReferenceOr<Schema> = serde_json::from_value(input.clone()).unwrap();
    let ReferenceOr::Item(item) = &parsed else { panic!("expected an item, got {parsed:?}") };
    assert!(item.schema_data.write_only);
    assert_eq!(item.schema_data.extensions["$ref"], json!("#/components/schemas/Pet"));
    let back = serde_json::to_value(&parsed).unwrap();
    assert_eq!(canonical(&back), canonical(&input));
}

#[test]
fn a_bare_ref_is_still_a_reference() {
    let parsed: ReferenceOr<Schema> = serde_json::from_value(json!({ "$ref": "#/components/schemas/Pet" })).unwrap();
    assert_eq!(parsed, ReferenceOr::ref_("#/components/schemas/Pet"));
}

#[test]
fn a_ref_with_siblings_on_a_non_schema_object_stays_a_reference() {
    // A Parameter cannot carry $ref as a keyword; the 3.0 reading (siblings dropped) holds.
    let parsed: ReferenceOr<Parameter> =
        serde_json::from_value(json!({ "$ref": "#/components/parameters/Limit", "description": "x" })).unwrap();
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
    let doc: OpenAPI = serde_json::from_value(input.clone()).expect("a paths-less 3.1 document parses");
    assert!(doc.paths.paths.is_empty());
    assert_eq!(doc.info.extensions["summary"], json!("only webhooks"));
    assert!(doc.extensions.contains_key("jsonSchemaDialect"));
    let back = serde_json::to_value(&doc).unwrap();
    assert_eq!(canonical(&back), canonical(&input));
}

#[test]
fn the_3_0_dialect_is_untouched() {
    let parsed = schema(json!({ "type": "string", "nullable": true, "enum": ["a", null] }));
    assert!(parsed.schema_data.nullable);
    match &parsed.schema_kind {
        SchemaKind::Type(Type::String(s)) => assert_eq!(s.enumeration, vec![Some("a".to_string()), None]),
        other => panic!("expected a string, got {other:?}"),
    }
    let parsed = schema(json!({ "type": "number", "minimum": 1, "exclusiveMinimum": true }));
    match &parsed.schema_kind {
        SchemaKind::Type(Type::Number(n)) => {
            assert!(n.exclusive_minimum);
            assert_eq!(n.minimum, Some(1.0));
        }
        other => panic!("expected a number, got {other:?}"),
    }
    // Vendor extensions keep working everywhere.
    round_trips(json!({ "type": "object", "x-internal": true, "properties": { "a": { "type": "string", "x-note": 1 } } }));
}
