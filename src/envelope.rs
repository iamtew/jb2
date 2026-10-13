use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct Envelope {
    #[serde(rename = "exitCode")]
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl Envelope {
    pub fn ok_msg(message: impl Into<String>, data: Value) -> Self {
        Self {
            exit_code: 0,
            message: Some(message.into()),
            data: Some(data),
        }
    }

    pub fn ok_data(data: Value) -> Self {
        Self {
            exit_code: 0,
            message: None,
            data: Some(data),
        }
    }

    pub fn ok_only(message: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            message: Some(message.into()),
            data: None,
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            message: Some(message.into()),
            data: None,
        }
    }

    pub fn fail_data(message: impl Into<String>, data: Value) -> Self {
        Self {
            exit_code: 1,
            message: Some(message.into()),
            data: Some(data),
        }
    }
}
