use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum ReferenceOr<T> {
    Reference {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Item(T),
}

// We implement Deserialize by hand in order to provide a useful error message.
// The derived error is typically aggravating: "data did not match any variant
// of untagged enum ReferenceOr". Instead, we deserialize to an enum that
// includes a third variant that covers any data; for that variant, we parse
// again and respond with that error.
//
// The generated code for Deserialize with an untagged enum first deserializes
// into an intermediate form. Unlike serde_json::Value, for example, it uses a
// type that preserves order (it is, unfortunately, not part of the public
// interface to serde). The implementation below loses object property order
// in the failure case... but it's the failure case so we don't particularly
// care.
impl<'de, T> Deserialize<'de> for ReferenceOr<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Tried in order. A bare `{"$ref": …}` is a Reference Object. A `$ref`
        // beside other keywords is, in JSON Schema 2020-12 (OpenAPI 3.1), a
        // schema in which `$ref` is one keyword among the others, so the item
        // is tried next and keeps every sibling. An object that cannot carry
        // `$ref` as a keyword (a Parameter, a Response) falls back to the 3.0
        // reading: a reference, siblings dropped. The untagged enum buffers the
        // input in serde's order-preserving form, so object order survives.
        #[derive(Deserialize)]
        #[serde(untagged, deny_unknown_fields)]
        enum RefOrInner<TT> {
            Reference {
                #[serde(rename = "$ref")]
                reference: String,
            },
            Item(TT),
            Loose(LooseReference),
            Fail(serde_json::Value),
        }

        #[derive(Deserialize)]
        struct LooseReference {
            #[serde(rename = "$ref")]
            reference: String,
        }

        let inner = RefOrInner::<T>::deserialize(deserializer)?;

        match inner {
            RefOrInner::Reference { reference } => Ok(ReferenceOr::Reference { reference }),
            RefOrInner::Item(item) => Ok(ReferenceOr::Item(item)),
            RefOrInner::Loose(LooseReference { reference }) => {
                Ok(ReferenceOr::Reference { reference })
            }

            // We know this will fail. It's inefficient in that we try (and
            // fail) T::deserialize twice, but it allows us to produce a better
            // error message.
            RefOrInner::Fail(value) => Err(T::deserialize(value)
                .map_err(<D::Error as serde::de::Error>::custom)
                .err()
                .expect("somehow this parsed successfully the second time")),
        }
    }
}

impl<T> ReferenceOr<T> {
    pub fn ref_(r: &str) -> Self {
        ReferenceOr::Reference {
            reference: r.to_owned(),
        }
    }
    pub fn boxed_item(item: T) -> ReferenceOr<Box<T>> {
        ReferenceOr::Item(Box::new(item))
    }

    /// Converts this [ReferenceOr] to the item inside, if it exists.
    ///
    /// The return value will be [Option::Some] if this was a [ReferenceOr::Item] or [Option::None] if this was a [ReferenceOr::Reference].
    ///
    /// # Examples
    ///
    /// ```
    /// # use openapiv3::ReferenceOr;
    ///
    /// let i = ReferenceOr::Item(1);
    /// assert_eq!(i.into_item(), Some(1));
    ///
    /// let j: ReferenceOr<u8> = ReferenceOr::Reference { reference: String::new() };
    /// assert_eq!(j.into_item(), None);
    /// ```
    pub fn into_item(self) -> Option<T> {
        match self {
            ReferenceOr::Reference { .. } => None,
            ReferenceOr::Item(i) => Some(i),
        }
    }

    /// Returns a reference to to the item inside this [ReferenceOr], if it exists.
    ///
    /// The return value will be [Option::Some] if this was a [ReferenceOr::Item] or [Option::None] if this was a [ReferenceOr::Reference].
    ///
    /// # Examples
    ///
    /// ```
    /// # use openapiv3::ReferenceOr;
    ///
    /// let i = ReferenceOr::Item(1);
    /// assert_eq!(i.as_item(), Some(&1));
    ///
    /// let j: ReferenceOr<u8> = ReferenceOr::Reference { reference: String::new() };
    /// assert_eq!(j.as_item(), None);
    /// ```
    pub fn as_item(&self) -> Option<&T> {
        match self {
            ReferenceOr::Reference { .. } => None,
            ReferenceOr::Item(i) => Some(i),
        }
    }
}

impl<T> ReferenceOr<Box<T>> {
    pub fn unbox(self) -> ReferenceOr<T> {
        match self {
            ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
            ReferenceOr::Item(boxed) => ReferenceOr::Item(*boxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{ReferenceOr, Schema, SchemaKind, Type};

    #[test]
    fn test_bad_responses() {
        // Note: missing the "description" field
        let value = json!({
            "200": {
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "string"
                        }
                    }
                }
            }
        });

        match serde_json::from_value::<crate::Responses>(value) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(
                e.to_string().contains("missing field `description`"),
                "unhelpful error: {e}"
            ),
        }
    }

    #[test]
    fn test_ref_or_order() {
        let value = r#"
        {
            "type": "object",
            "properties": {
                "z": {},
                "a": {},
                "b": {}
            }
        }
        "#;

        let obj = serde_json::from_str::<ReferenceOr<Schema>>(value).unwrap();

        let ReferenceOr::Item(obj) = obj else {
            panic!()
        };

        let SchemaKind::Type(Type::Object(obj)) = obj.schema_kind else {
            panic!()
        };

        let props = obj.properties.keys().collect::<Vec<_>>();
        assert_eq!(props.as_slice(), ["z", "a", "b"]);
    }

    #[test]
    fn test_ref_and() {
        // A $ref with siblings is a schema carrying $ref as a keyword (JSON
        // Schema 2020-12, OpenAPI 3.1); the siblings are kept, not ignored.
        let value = json!({
            "description": "this is kept",
            "$ref": "#/foo/bar"
        });

        let ref_or_schema = serde_json::from_value::<ReferenceOr<Schema>>(value);

        match ref_or_schema {
            Ok(ReferenceOr::Item(schema)) => {
                assert_eq!(schema.schema_data.description.as_deref(), Some("this is kept"));
                assert_eq!(schema.schema_data.extensions["$ref"], json!("#/foo/bar"));
            }
            Ok(ReferenceOr::Reference { .. }) => panic!("parsed as a bare reference"),
            Err(e) => panic!("failed to parse: {e}"),
        }
    }

    #[test]
    fn test_bare_ref() {
        let value = json!({ "$ref": "#/foo/bar" });
        match serde_json::from_value::<ReferenceOr<Schema>>(value) {
            Ok(ReferenceOr::Reference { reference }) => assert_eq!(reference, "#/foo/bar"),
            other => panic!("expected a reference, got {other:?}"),
        }
    }
}
