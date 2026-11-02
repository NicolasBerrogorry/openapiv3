# OpenAPI v3 ![example workflow](https://github.com/glademiller/openapiv3/actions/workflows/rust.yml/badge.svg)


This crate provides data structures that represent the [OpenAPI v3.0.x specification](https://github.com/OAI/OpenAPI-Specification/blob/master/versions/3.0.3.md).

[OpenAPI v3.1](https://github.com/OAI/OpenAPI-Specification/blob/master/versions/3.1.0.md) documents are read as well, and what 3.1 adds is typed like what 3.0 has: a `type` set (`TypeSet`, with `null` as a type), a numeric exclusive bound (`ExclusiveBound`, beside the 3.0 flag), a boolean `items` (`BoolOrSchema`), the JSON Schema 2020-12 keywords on `SchemaData` and `ArrayType` (`const`, `examples`, `$defs`, `prefixItems`, `if`/`then`/`else`, `patternProperties`, `dependentSchemas`, `dependentRequired`, `unevaluatedProperties`, `contains`, `contentMediaType`, … and a `$ref` carried beside other keywords), `mutualTLS`, and on the document `webhooks`, `jsonSchemaDialect`, `info.summary`, `license.identifier` and `components.pathItems`. A 3.0 document reads exactly as before. Anything the model does not type is retained verbatim in the `extensions` map of the object that carries it and serialized back unchanged, so a document of either version round-trips. See `tests/openapi31.rs`.

```rust
let schema: openapiv3::Schema = serde_json::from_value(serde_json::json!({
    "type": ["string", "null"], "const": "a"
}))?;
if let openapiv3::SchemaKind::Any(any) = &schema.schema_kind {
    assert_eq!(any.typ.as_ref().unwrap().single(), Some(&openapiv3::SchemaType::String));
}
assert_eq!(schema.schema_data.const_value, Some(serde_json::json!("a")));
```

## Example

```rust
use serde_json;
use openapiv3::OpenAPI;

fn main() {
    let data = include_str!("openapi.json");
    let openapi: OpenAPI = serde_json::from_str(data).expect("Could not deserialize input");
    println!("{:?}", openapi);
}
```

## Goals
* Provide a deserialization for the specification that maps cleanly to Rust enums etc.

## Non Goals
* Deserialization and subsequent re-serialization are 100% the same.
    * Some defaults show-up when serializing that may not have existed in the input.

## Issues
Schemas without a type will end up as any data type as per the specification and can have any parameters of any schema type. Some Open API documents don't include the type parameter it would be nice to try to derive the type but the crate as of right now meets my needs.

## Similar Crates
* [openapi](https://crates.io/crates/openapi)

## License

This crate is licensed under either of

    Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
    MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.