use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Definition {
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

impl Definition {
    pub fn parse(r: &[u8]) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_slice(r)
    }

    pub fn get_notify(&self, event: Event) -> Option<Vec<&Notify>> {
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
    pub fn matches(&self, e: &Event) -> bool {
        match self {
            Event::All => true,
            Event::Success if matches!(e, Event::Success) => true,
            Event::Failure if matches!(e, Event::Failure) => true,
            Event::Start if matches!(e, Event::Start) => true,
            Event::Finish if matches!(e, Event::Success) || matches!(e, Event::Failure) => true,
            _ => false,
        }
    }
}
