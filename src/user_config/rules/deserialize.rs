use core::result::Result::Ok;
use std::path::PathBuf;

use serde::{de, export, Deserialize, Deserializer};

use crate::path::Expandable;
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    export::{fmt, PhantomData},
};
use std::str::FromStr;

pub(in crate::user_config) fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    Ok(PathBuf::from(&buf).expand_user().expand_vars())
}

pub(super) fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = ()>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = ()>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

pub(super) fn string_or_seq<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrSeq(PhantomData<fn() -> String>);

    impl<'de> Visitor<'de> for StringOrSeq {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
            formatter.write_str("string or seq")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.into()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(val) = seq.next_element()? {
                vec.push(val)
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrSeq(PhantomData))
}
