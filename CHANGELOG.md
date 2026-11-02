# Changelog

## Unreleased (next major: field types change)

- Read OpenAPI 3.1 documents, typed like 3.0: `type` is a `TypeSet` (one name or a set,
  `null` included); `exclusiveMinimum`/`exclusiveMaximum` are an `ExclusiveBound` (the 3.0
  flag or the 3.1 value); `items` is a `BoolOrSchema`; `SchemaData` gains the JSON Schema
  2020-12 keywords (`$ref` beside other keywords, `$id`, `$anchor`, `$comment`, `const`,
  `examples`, `$defs`, `if`/`then`/`else`, `patternProperties`, `dependentSchemas`,
  `dependentRequired`, `unevaluatedProperties`/`unevaluatedItems`, `contentMediaType`,
  `contentEncoding`); `ArrayType` and `AnySchema` gain `prefixItems`, `contains`,
  `minContains`, `maxContains`; `OpenAPI` gains `webhooks` and `json_schema_dialect`,
  `Info` `summary`, `License` `identifier`, `Components` `path_items`
- Breaking: `AnySchema::typ` is `Option<TypeSet>` (was `Option<String>`); the
  `exclusive_minimum`/`exclusive_maximum` fields of `NumberType`, `IntegerType` and
  `AnySchema` are `Option<ExclusiveBound>` (were `bool` / `Option<bool>`), the 3.1 value kept as the
  JSON number it was written as; the `items`
  fields are `Option<BoolOrSchema>` (were `Option<ReferenceOr<Box<Schema>>>`); public
  structs gained fields (construct them with `..Default::default()`)
- Anything the model does not type is retained verbatim in `extensions` and serialized
  back unchanged, so a document of either version round-trips
- Add the `mutualTLS` security scheme and `extensions` to the `OpenIDConnect` scheme
- `paths` is optional, as in 3.1 (a webhooks-only document)
- A `$ref` with sibling keywords deserializes as the schema carrying it (JSON Schema
  2020-12 semantics) where the item type can hold it, instead of a bare reference
  with the siblings dropped

## 2.2.0 (2025-06-02)

- Fix regression in `ReferenceOr` parsing to preserve order and priority

## 2.1.0 (2025-05-22) (yanked)

- Improve deserialization error messages from `ReferenceOr`
- Fix OAuth structure field visibility

## 2.0.0 (2023-12-08)

- Add additional documentation

## 2.0.0-rc.1 (2023-10-21)

- Fix handling of null values in `enum` for string, number, integer, and boolean

## 2.0.0-rc.0 (2023-10-21)

- Bump `indexmap` dependency to 2.0.0
- Add `enum` field to boolean schema
- Add `callbacks` to `Operation` type (out of conformance with spec)
- Fix handling of subschemas with typed Schema objects
- Fixes to SecuritySchema handling
