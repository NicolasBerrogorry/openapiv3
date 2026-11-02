//! The 3.1 document-level additions, typed.

use openapiv3::*;
use serde_json::json;

fn document(input: serde_json::Value) -> OpenAPI {
    serde_json::from_value(input).expect("a document parses")
}

#[test]
fn document_fields_of_3_1_are_typed_and_round_trip() {
    let input = json!({
        "openapi": "3.1.0",
        "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema",
        "info": {
            "title": "t", "version": "1", "summary": "a short one",
            "license": { "name": "Apache 2.0", "identifier": "Apache-2.0" }
        },
        "webhooks": {
            "newPet": { "post": { "operationId": "newPet", "responses": { "200": { "description": "ok" } } } },
            "shared": { "$ref": "#/components/pathItems/Shared" }
        },
        "components": {
            "pathItems": { "Shared": { "get": { "responses": { "200": { "description": "ok" } } } } }
        }
    });
    let doc = document(input.clone());

    assert_eq!(
        doc.json_schema_dialect.as_deref(),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(doc.info.summary.as_deref(), Some("a short one"));
    assert_eq!(
        doc.info.license.as_ref().unwrap().identifier.as_deref(),
        Some("Apache-2.0")
    );
    assert_eq!(doc.webhooks.len(), 2);
    match &doc.webhooks["newPet"] {
        ReferenceOr::Item(item) => assert_eq!(
            item.post.as_ref().unwrap().operation_id.as_deref(),
            Some("newPet")
        ),
        other => panic!("expected a path item, got {other:?}"),
    }
    assert_eq!(
        doc.webhooks["shared"],
        ReferenceOr::ref_("#/components/pathItems/Shared")
    );
    let path_items = &doc.components.as_ref().unwrap().path_items;
    assert!(matches!(&path_items["Shared"], ReferenceOr::Item(item) if item.get.is_some()));
    for object in [&doc.extensions, &doc.info.extensions] {
        assert!(object.is_empty(), "nothing left untyped: {object:?}");
    }

    let back = serde_json::to_value(&doc).unwrap();
    assert_eq!(back["webhooks"], input["webhooks"]);
    assert_eq!(
        back["components"]["pathItems"],
        input["components"]["pathItems"]
    );
    assert_eq!(back["info"]["summary"], input["info"]["summary"]);
    assert_eq!(back["jsonSchemaDialect"], input["jsonSchemaDialect"]);
}

#[test]
fn a_3_0_document_has_none_of_them() {
    let doc = document(
        json!({ "openapi": "3.0.3", "info": { "title": "t", "version": "1" }, "paths": {} }),
    );
    assert!(doc.webhooks.is_empty());
    assert_eq!(doc.json_schema_dialect, None);
    assert_eq!(doc.info.summary, None);
    let back = serde_json::to_value(&doc).unwrap();
    assert!(back.get("webhooks").is_none());
    assert!(back.get("jsonSchemaDialect").is_none());
}

#[test]
fn the_petstore_3_1_document_is_typed() {
    let doc: OpenAPI = serde_json::from_str(include_str!("../fixtures/petstore31.json")).unwrap();
    assert!(!doc.webhooks.is_empty());
    let pet = match &doc.components.as_ref().unwrap().schemas["Pet"] {
        ReferenceOr::Item(schema) => schema,
        other => panic!("expected the Pet schema inline, got {other:?}"),
    };
    let properties = match &pet.schema_kind {
        SchemaKind::Type(Type::Object(object)) => &object.properties,
        SchemaKind::Any(any) => &any.properties,
        other => panic!("expected an object schema, got {other:?}"),
    };
    match &properties["availableInstances"] {
        ReferenceOr::Item(available) => {
            match &available.schema_kind {
                SchemaKind::Type(Type::Integer(integer)) => {
                    assert_eq!(
                        integer.exclusive_minimum,
                        Some(ExclusiveBound::Value(1.into()))
                    );
                    assert_eq!(
                        integer.exclusive_maximum,
                        Some(ExclusiveBound::Value(10.into()))
                    );
                }
                other => panic!("expected an integer, got {other:?}"),
            }
            assert_eq!(available.schema_data.examples.len(), 1);
        }
        other => panic!("expected an inline property, got {other:?}"),
    }
}
