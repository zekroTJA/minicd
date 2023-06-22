use serde::Deserialize;
use std::{collections::HashMap, error::Error, fs::File, path::Path};

#[derive(Deserialize)]
#[serde(untagged)]
enum Value {
    Value(String),
    Map(HashMap<String, Value>),
}

impl Value {
    fn unwrap(&self) -> Option<String> {
        match self {
            Self::Value(v) => Some(v.clone()),
            Self::Map(_) => None,
        }
    }
}

pub struct SecretManager {
    secrets: Value,
}

impl SecretManager {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let f = File::open(dir)?;
        let secrets = serde_yaml::from_reader(f)?;
        Ok(Self { secrets })
    }

    pub fn empty() -> Self {
        Self {
            secrets: Value::Map(HashMap::with_capacity(0)),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut res = Some(&self.secrets);

        for key in key.split('.') {
            let Some(c) = res else {
                break;
            };
            let Value::Map(c) = c else {
                return None;
            };
            res = c.get(key);
        }

        res.and_then(|v| v.unwrap())
    }

    pub fn replace(&self, content: &str) -> String {
        let mut v = content;
        let mut result = String::new();

        loop {
            let Some(start) = v.find("{{") else {
                break;
            };

            let next = &v[start + 2..];
            let Some(end) = next.find("}}") else {
                break;
            };

            result.push_str(&v[..start]);

            let key = &next[..end];

            match self.get(key.trim()) {
                Some(val) => {
                    result.push_str(&val);
                }
                None => {
                    result.push_str(&v[start..start + 4 + end]);
                }
            }

            v = &v[start + 4 + end..];
        }

        result.push_str(v);

        result
    }

    pub fn to_flat_map(&self) -> HashMap<String, String> {
        let mut hashmap = HashMap::new();
        add_properties_to_hashmap("", &self.secrets, &mut hashmap);
        hashmap
    }
}

fn add_properties_to_hashmap(
    parent_key: &str,
    value: &Value,
    hashmap: &mut HashMap<String, String>,
) {
    match value {
        Value::Value(val) => {
            if !parent_key.is_empty() {
                hashmap.insert(parent_key.to_string(), val.clone());
            }
        }
        Value::Map(map) => {
            for (key, value) in map.iter() {
                let mut new_key = parent_key.to_string();
                if !new_key.is_empty() {
                    new_key.push('.');
                }
                new_key.push_str(key);
                add_properties_to_hashmap(&new_key, value, hashmap);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get() {
        let mut secrets = SecretManager::empty();
        secrets.secrets = Value::Map(HashMap::from([(
            "a".into(),
            Value::Map(HashMap::from([("b".into(), Value::Value("foo".into()))])),
        )]));

        assert_eq!(None, secrets.get("a"));
        assert_eq!(None, secrets.get("a.c"));
        assert_eq!(Some("foo".to_string()), secrets.get("a.b"));
    }

    #[test]
    fn replace() {
        let mut secrets = SecretManager::empty();
        secrets.secrets = Value::Map(HashMap::from([(
            "a".into(),
            Value::Map(HashMap::from([("b".into(), Value::Value("foo".into()))])),
        )]));

        assert_eq!(
            "hello foo this is foo world!",
            secrets.replace("hello {{a.b}} this is {{a.b}} world!")
        );

        assert_eq!(
            "hello foo this is {{a.c}} world!",
            secrets.replace("hello {{a.b}} this is {{a.c}} world!")
        );

        assert_eq!(
            "hello foo this is {{  a.c  }} world!",
            secrets.replace("hello {{ a.b  }} this is {{  a.c  }} world!")
        );

        assert_eq!("foo {{ bar", secrets.replace("foo {{ bar"));

        assert_eq!("foo foo {{ bazz", secrets.replace("foo {{ a.b }} {{ bazz"));

        assert_eq!(
            "foo foo {{{{{}}foo",
            secrets.replace("foo {{ a.b }} {{{{{}}{{ a.b }}")
        );
    }

    #[test]
    fn to_flat_map() {
        let mut secrets = SecretManager::empty();
        secrets.secrets = Value::Map(HashMap::from([(
            "a".into(),
            Value::Map(HashMap::from([
                ("b".into(), Value::Value("foo".into())),
                ("c".into(), Value::Value("bar".into())),
            ])),
        )]));

        let res = secrets.to_flat_map();

        assert_eq!(
            HashMap::from([
                ("a.b".to_string(), "foo".to_string()),
                ("a.c".to_string(), "bar".to_string())
            ]),
            res
        );
    }
}
