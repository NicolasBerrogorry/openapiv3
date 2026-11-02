# Changelog

## Unreleased

- Read OpenAPI 3.1 documents without loss: 3.1-only vocabulary and the 3.1 forms of
  modeled keywords are retained verbatim in `extensions` and serialized back unchanged
- Add the `mutualTLS` security scheme and `extensions` to the `OpenIDConnect` scheme
- `paths` is optional, as in 3.1 (a webhooks-only document)
- A `$ref` with sibling keywords deserializes as the item carrying `$ref` (JSON Schema
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
