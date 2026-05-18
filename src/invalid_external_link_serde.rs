//! invalid_external_link_serde.rs - serde serializer and deserializer for the
//! `InvalidExternalLink` variant of `DiagnosticKind`, which is a struct variant
//! with one field `error` that is a `url::ParseError`, and the latter doesn't implement
//! Serialize and Deserialize.

const URL_ERRORS: [&'static str; 10] = [
    "EmptyHost",
    "IdnaError",
    "InvalidPort",
    "InvalidIpv4Address",
    "InvalidIpv6Address",
    "InvalidDomainCharacter",
    "RelativeUrlWithoutBase",
    "RelativeUrlWithCannotBeABaseBase",
    "SetHostOnCannotBeABaseUrl",
    "Overflow",
];

const URL_ERRORS_COMMASEP_STR: &'static str = "EmptyHost, IdnaError, InvalidPort, InvalidIpv4Address, InvalidIpv6Address, InvalidDomainCharacter, RelativeUrlWithoutBase, RelativeUrlWithCannotBeABaseBase, SetHostOnCannotBeABaseUrl, Overflow";

// wip maybe: const-concat the array of URL_ERRORS...
// const fn const_concat_strs(xs: &[&'static str], x: &str) -> &str {
//     match xs {
//         [] => x,
//         [a] => concat!(x, ", ", a),
//         [a, ..rest] => {
//             concat!(a, ",", const_concat_strs(rest));
//         }
//     }
// }

fn str_from_url_parseerror(val: &url::ParseError) -> &'static str {
    match val {
        url::ParseError::EmptyHost => "EmptyHost",
        url::ParseError::IdnaError => "IdnaError",
        url::ParseError::InvalidPort => "InvalidPort",
        url::ParseError::InvalidIpv4Address => "InvalidIpv4Address",
        url::ParseError::InvalidIpv6Address => "InvalidIpv6Address",
        url::ParseError::InvalidDomainCharacter => "InvalidDomainCharacter",
        url::ParseError::RelativeUrlWithoutBase => "RelativeUrlWithoutBase",
        url::ParseError::RelativeUrlWithCannotBeABaseBase => "RelativeUrlWithCannotBeABaseBase",
        url::ParseError::SetHostOnCannotBeABaseUrl => "SetHostOnCannotBeABaseUrl",
        url::ParseError::Overflow => "Overflow",
        // non_exhaustive, must have catch-all
        _ => "UNKNOWN",
    }
}

fn url_parseerror_from_str(s: &str) -> Option<url::ParseError> {
    match s {
        "EmptyHost" => Some(url::ParseError::EmptyHost),
        "IdnaError" => Some(url::ParseError::IdnaError),
        "InvalidPort" => Some(url::ParseError::InvalidPort),
        "InvalidIpv4Address" => Some(url::ParseError::InvalidIpv4Address),
        "InvalidIpv6Address" => Some(url::ParseError::InvalidIpv6Address),
        "InvalidDomainCharacter" => Some(url::ParseError::InvalidDomainCharacter),
        "RelativeUrlWithoutBase" => Some(url::ParseError::RelativeUrlWithoutBase),
        "RelativeUrlWithCannotBeABaseBase" => Some(url::ParseError::RelativeUrlWithCannotBeABaseBase),
        "SetHostOnCannotBeABaseUrl" => Some(url::ParseError::SetHostOnCannotBeABaseUrl),
        "Overflow" => Some(url::ParseError::Overflow),
        _ => None,
    }
}

pub fn serialize<S>(error: &url::ParseError, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStructVariant;
    const ENUM_NAME: &str = "DiagnosticKind";
    const VARIANT_INDEX: u32 = 1;
    const VARIANT_NAME: &str = "InvalidExternalLink";
    const VARIANT_LEN: usize = 1;

    let mut s = serializer.serialize_struct_variant(
        ENUM_NAME,
        VARIANT_INDEX,
        VARIANT_NAME,
        VARIANT_LEN,
    )?;
    let value = str_from_url_parseerror(error);
    s.serialize_field("error", value)?;
    s.end()
}

fn serde_json_value_to_unexpected(val: &serde_json::Value) -> serde::de::Unexpected {
    match *val {
        serde_json::Value::Null => serde::de::Unexpected::Option,
        serde_json::Value::Bool(b) => serde::de::Unexpected::Bool(b),
        serde_json::Value::Number(ref n) => {
            let float = n.as_f64().expect("all numbers are f64s, right?");
            serde::de::Unexpected::Float(float)
        }
        serde_json::Value::String(ref s) => serde::de::Unexpected::Str(s),
        serde_json::Value::Array(ref _a) => serde::de::Unexpected::Seq,
        serde_json::Value::Object(ref _o) => serde::de::Unexpected::Map,
    }
}

macro_rules! sjv_expect_type {
    ($var:ident, string) => {{
        let serde_json::Value::String(s) = $var else {
            let unexpected = serde_json_value_to_unexpected($var);
            return Err(serde::de::Error::invalid_type(unexpected, &"string"));
        };
        s
    }};
    ($var:ident, object) => {{
        let serde_json::Value::Object(v) = $var else {
            let unexpected = serde_json_value_to_unexpected(&$var);
            return Err(serde::de::Error::invalid_type(unexpected, &"object"));
        };
        v
    }};
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<url::ParseError, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = url::ParseError;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "an object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>, {

            // trick here: deserialize the expected map, but since the value type
            // is unnamed, and url::ParseError doesn't implement Deserialize, deserialize
            // the value into a serde_json::Value, and parse it accordingly
            
            let mut final_val = None;
            while let Some(k) = map.next_key()? {
                if k != "InvalidExternalLink" {
                    return Err(serde::de::Error::unknown_field(k, &["InvalidExternalLink"]));
                }

                let v: serde_json::Value = map.next_value()?;
                let v = sjv_expect_type!(v, object);

                if v.len() != 1 {
                    return Err(serde::de::Error::invalid_length(v.len(), &"1"));
                }

                let Some(item) = v.get("error") else {
                    return Err(serde::de::Error::missing_field("error"));
                };

                let inner = sjv_expect_type!(item, string);
                let Some(url_error) = url_parseerror_from_str(inner.as_str()) else {
                    let unexp = serde::de::Unexpected::Str(inner.as_str());
                    return Err(serde::de::Error::invalid_value(unexp, &URL_ERRORS_COMMASEP_STR));
                };

                final_val = Some(url_error);
            }

            let Some(final_val) = final_val else {
                return Err(serde::de::Error::missing_field("InvalidExternalLink"));
            };
            Ok(final_val)
        }
    }
    deserializer.deserialize_map(Visitor)
}

