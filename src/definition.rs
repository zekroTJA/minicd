use regex::Regex;
use serde::Deserialize;
use std::{collections::HashMap, fmt, str::FromStr};

#[derive(thiserror::Error, Debug)]
#[error("invalid reference format")]
pub struct RefParseError;

#[derive(Deserialize)]
pub struct Definition {
    pub name: String,
    pub jobs: HashMap<String, Job>,
}

#[derive(Deserialize)]
pub struct Job {
    #[serde(with = "serde_yaml::with::singleton_map")]
    pub on: Option<Ref>,
    pub notify: Option<Vec<Notify>>,
    pub shell: Option<ValueOrList<String>>,
    #[serde(rename = "await")]
    pub await_result: Option<bool>,
    pub run: String,
}

#[derive(Deserialize)]
pub struct Notify {
    pub to: Vec<NotifyTarget>,
    pub on: Option<Vec<Event>>,
}

#[derive(Deserialize)]
pub enum Event {
    #[serde(rename = "success")]
    Success,

    #[serde(rename = "failure")]
    Failure,

    #[serde(rename = "finish")]
    Finish,

    #[serde(rename = "start")]
    Start,

    #[serde(rename = "all")]
    All,
}

#[derive(Deserialize, Clone, Debug)]
pub enum Ref {
    #[serde(rename = "branch")]
    Branch(String),

    #[serde(rename = "tag")]
    Tag(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ValueOrList<T> {
    Value(T),
    List(Vec<T>),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum NotifyTarget {
    #[serde(rename = "email")]
    EMail { address: String },
    #[serde(rename = "webhook")]
    WebHook {
        url: String,
        method: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
}

pub enum JobState {
    Start,
    Success,
    Failure,
}

impl Definition {
    pub fn parse(r: &[u8]) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_slice(r)
    }
}

impl Job {
    pub fn get_notify(&self, event: JobState) -> Option<Vec<&Notify>> {
        self.notify.as_ref().map(|notifies| {
            notifies
                .iter()
                .filter(|notify| {
                    notify
                        .on
                        .as_ref()
                        .is_some_and(|events| events.iter().any(|e| e.matches(&event)))
                })
                .collect()
        })
    }
}

impl Event {
    pub fn matches(&self, e: &JobState) -> bool {
        match self {
            Event::All => true,
            Event::Success if matches!(e, JobState::Success) => true,
            Event::Failure if matches!(e, JobState::Failure) => true,
            Event::Start if matches!(e, JobState::Start) => true,
            Event::Finish if matches!(e, JobState::Success) || matches!(e, JobState::Failure) => {
                true
            }
            _ => false,
        }
    }
}

impl FromStr for Ref {
    type Err = RefParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 6 {
            return Err(RefParseError);
        }

        let Some((typ, rf)) = s[5..].split_once('/') else {
            return Err(RefParseError);
        };

        match typ {
            "heads" => Ok(Self::Branch(rf.into())),
            "tags" => Ok(Self::Tag(rf.into())),
            _ => Err(RefParseError),
        }
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Branch(v) => write!(f, "refs/heads/{v}"),
            Self::Tag(v) => write!(f, "refs/tags/{v}"),
        }
    }
}

impl Ref {
    pub fn matches(&self, rf: &Ref) -> bool {
        match (self, rf) {
            (Ref::Branch(rx), Ref::Branch(ref rf)) => rx_match(rx, rf),
            (Ref::Tag(rx), Ref::Tag(ref rf)) => rx_match(rx, rf),
            _ => false,
        }
    }
}

fn rx_match(rx: &str, rf: &str) -> bool {
    let Ok(rx) = Regex::new(rx) else {
        return false;
    };

    rx.is_match(rf)
}
