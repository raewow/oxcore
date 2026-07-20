//! Wire types for the REST login service.
//!
//! These mirror `Battlenet.JSON.Login` (TrinityCore `src/server/proto/Login/Login.proto`),
//! which the client consumes as JSON rather than binary protobuf. Field names are therefore
//! load-bearing and must match the proto exactly — including the odd-looking `public_B` and
//! `server_evidence_M2`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize)]
pub enum FormType {
    #[serde(rename = "LOGIN_FORM")]
    LoginForm,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormInput {
    pub input_id: &'static str,
    #[serde(rename = "type")]
    pub input_type: &'static str,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormInputs {
    #[serde(rename = "type")]
    pub form_type: FormType,
    pub inputs: Vec<FormInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srp_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormInputValue {
    pub input_id: String,
    pub value: String,
}

/// What the client POSTs to `/bnetserver/login/` and `/bnetserver/login/srp/`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    pub platform_id: String,
    #[serde(default)]
    pub program_id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub inputs: Vec<FormInputValue>,
}

impl LoginForm {
    /// Look up a submitted input by id. Absent and empty are treated alike — the client sends
    /// empty strings for fields the user left blank.
    pub fn get(&self, input_id: &str) -> Option<&str> {
        self.inputs
            .iter()
            .find(|i| i.input_id == input_id)
            .map(|i| i.value.as_str())
            .filter(|v| !v.is_empty())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum AuthenticationState {
    #[serde(rename = "LOGIN")]
    Login,
    #[serde(rename = "DONE")]
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[allow(non_snake_case)] // field names are fixed by the proto/JSON contract
pub struct SrpLoginChallenge {
    pub version: u32,
    pub iterations: u32,
    pub modulus: String,
    pub generator: String,
    pub hash_function: String,
    pub username: String,
    pub salt: String,
    pub public_B: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[allow(non_snake_case)] // field names are fixed by the proto/JSON contract
pub struct LoginResult {
    pub authentication_state: Option<AuthenticationState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_ticket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_evidence_M2: Option<String>,
}

impl LoginResult {
    pub fn done(login_ticket: String, server_evidence_m2: String) -> Self {
        Self {
            authentication_state: Some(AuthenticationState::Done),
            login_ticket: Some(login_ticket),
            server_evidence_M2: Some(server_evidence_m2),
            ..Default::default()
        }
    }

    /// A rejected login. The client re-shows the form with `error_message` attached.
    pub fn rejected(error_code: &str, error_message: &str) -> Self {
        Self {
            authentication_state: Some(AuthenticationState::Login),
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginRefreshResult {
    pub login_ticket_expiry: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_expired: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameAccountInfo {
    pub display_name: String,
    pub expansion: u32,
    pub is_suspended: bool,
    pub is_banned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameAccountList {
    pub game_accounts: Vec<GameAccountInfo>,
}

/// The login form description served by `GET /bnetserver/login/`.
pub fn login_form(srp_url: Option<String>) -> FormInputs {
    FormInputs {
        form_type: FormType::LoginForm,
        inputs: vec![
            FormInput {
                input_id: "account_name",
                input_type: "text",
                label: "E-mail",
                max_length: Some(320),
            },
            FormInput {
                input_id: "password",
                input_type: "password",
                label: "Password",
                max_length: Some(128),
            },
            FormInput {
                input_id: "log_in_submit",
                input_type: "submit",
                label: "Log In",
                max_length: None,
            },
        ],
        srp_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_form_serializes_with_proto_field_names() {
        let json = serde_json::to_value(login_form(None)).unwrap();
        assert_eq!(json["type"], "LOGIN_FORM");
        assert_eq!(json["inputs"][0]["input_id"], "account_name");
        assert_eq!(json["inputs"][0]["type"], "text");
        assert_eq!(json["inputs"][0]["max_length"], 320);
        // srp_url is optional and must be omitted rather than null when unset.
        assert!(json.get("srp_url").is_none());
    }

    #[test]
    fn submit_input_has_no_max_length() {
        let json = serde_json::to_value(login_form(None)).unwrap();
        assert!(json["inputs"][2].get("max_length").is_none());
    }

    #[test]
    fn login_result_omits_unset_optionals() {
        let json = serde_json::to_value(LoginResult::done("OX-abc".into(), "ff".into())).unwrap();
        assert_eq!(json["authentication_state"], "DONE");
        assert_eq!(json["login_ticket"], "OX-abc");
        assert_eq!(json["server_evidence_M2"], "ff");
        assert!(json.get("error_code").is_none());
    }

    #[test]
    fn form_get_treats_empty_values_as_absent() {
        let form: LoginForm = serde_json::from_str(
            r#"{"platform_id":"Win","program_id":"WoW","version":"1",
                "inputs":[{"input_id":"account_name","value":""},
                          {"input_id":"password","value":"hunter2"}]}"#,
        )
        .unwrap();

        assert_eq!(form.get("account_name"), None);
        assert_eq!(form.get("password"), Some("hunter2"));
        assert_eq!(form.get("missing"), None);
    }
}
