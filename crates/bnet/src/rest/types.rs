//! Wire types for the REST login service.
//!
//! The client consumes these as JSON rather than binary protobuf. Field names are therefore
//! load-bearing and must match the proto exactly — including the odd-looking `public_B` and
//! `server_evidence_M2`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize)]
pub enum FormType {
    #[serde(rename = "LOGIN_FORM")]
    LoginForm,
}

// Field order in these two structs is load-bearing by imitation, not by protocol: the
// login browser expects members alphabetically by name (matching DataContractJsonSerializer
// output). Keeping serde's declaration order in step with that makes our form byte-identical to the
// one the 1.14 login browser is known to accept, so key order is one fewer variable if it starts
// rejecting the form again.
#[derive(Debug, Clone, Serialize)]
pub struct FormInput {
    pub input_id: &'static str,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(rename = "type")]
    pub input_type: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormInputs {
    pub inputs: Vec<FormInput>,
    /// The bundled login browser requires this unset optional to be present as JSON `null`.
    /// the field to be present even though it has no prompt text.
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srp_url: Option<String>,
    #[serde(rename = "type")]
    pub form_type: Option<FormType>,
}

impl FormInputs {
    /// The empty form nested inside every `LogonResult` — no inputs, and a null `type`
    /// because its `FormInputs.Type` is a default-constructed string.
    pub fn empty() -> Self {
        Self {
            inputs: Vec::new(),
            prompt: None,
            srp_url: None,
            form_type: None,
        }
    }
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

impl From<oxcore_shared::crypto::srp6v2::Challenge> for SrpLoginChallenge {
    fn from(c: oxcore_shared::crypto::srp6v2::Challenge) -> Self {
        Self {
            version: c.version,
            iterations: c.iterations,
            modulus: c.modulus,
            generator: c.generator,
            hash_function: c.hash_function,
            username: c.username,
            salt: c.salt,
            public_B: c.public_b,
        }
    }
}

/// A pre-SRP failure (bad request, unknown account) returned from the challenge endpoint. The
/// client renders `error_message` back on the login form.
pub fn login_error(error_code: &str, error_message: &str) -> axum::Json<LoginResult> {
    axum::Json(LoginResult::rejected(error_code, error_message))
}

#[derive(Debug, Clone, Serialize)]
#[allow(non_snake_case)] // field names are fixed by the proto/JSON contract
pub struct LoginResult {
    pub authentication_state: Option<AuthenticationState>,
    /// Always present and always empty. The `LogonResult.AuthenticatorForm` is initialised to a
    /// fresh `FormInputs`, so `DataContractJsonSerializer` emits a nested object rather than
    /// omitting the member; the login browser is fussy enough about this response's shape that we
    /// mirror it exactly rather than find out the hard way.
    pub authenticator_form: FormInputs,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub login_ticket: Option<String>,
    /// Only the SRP path produces evidence; the legacy login path has no such member, so it is omitted when
    /// unset rather than sent as null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_evidence_M2: Option<String>,
    pub support_error_code: Option<String>,
}

impl Default for LoginResult {
    fn default() -> Self {
        Self {
            authentication_state: None,
            authenticator_form: FormInputs::empty(),
            error_code: None,
            error_message: None,
            login_ticket: None,
            server_evidence_M2: None,
            support_error_code: None,
        }
    }
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

    pub fn done_without_evidence(login_ticket: String) -> Self {
        Self {
            authentication_state: Some(AuthenticationState::Done),
            login_ticket: Some(login_ticket),
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
pub fn login_form() -> FormInputs {
    FormInputs {
        form_type: Some(FormType::LoginForm),
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
                max_length: Some(16),
            },
            FormInput {
                input_id: "log_in_submit",
                input_type: "submit",
                label: "Log In",
                max_length: Some(0),
            },
        ],
        prompt: None,
        // The 1.14.2 browser uses the legacy plain-password login form. Including
        // `srp_url` makes it reject the form before it renders credentials.
        srp_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_form_serializes_with_proto_field_names() {
        let json = serde_json::to_value(login_form()).unwrap();
        assert_eq!(json["type"], "LOGIN_FORM");
        assert_eq!(json["inputs"][0]["input_id"], "account_name");
        assert_eq!(json["inputs"][0]["type"], "text");
        assert_eq!(json["inputs"][0]["max_length"], 320);
        // srp_url is optional and must be omitted rather than null when unset.
        assert!(json.get("srp_url").is_none());
    }

    #[test]
    fn legacy_form_includes_hermes_optional_defaults() {
        let json = serde_json::to_value(login_form()).unwrap();
        assert!(json["prompt"].is_null());
        assert_eq!(json["inputs"][2]["max_length"], 0);
    }

    #[test]
    fn login_result_matches_the_hermes_logon_result_shape() {
        let json =
            serde_json::to_string(&LoginResult::done_without_evidence("OX-abc".into())).unwrap();

        // Every member present, unset ones as null, alphabetical, with the nested empty
        // authenticator form — byte-identical to what `DataContractJsonSerializer` produces for
        // A legacy login result.
        assert_eq!(
            json,
            r#"{"authentication_state":"DONE","authenticator_form":{"inputs":[],"prompt":null,"type":null},"error_code":null,"error_message":null,"login_ticket":"OX-abc","support_error_code":null}"#
        );
    }

    #[test]
    fn srp_evidence_is_the_only_member_added_beyond_the_hermes_shape() {
        let json = serde_json::to_value(LoginResult::done("OX-abc".into(), "ff".into())).unwrap();
        assert_eq!(json["authentication_state"], "DONE");
        assert_eq!(json["login_ticket"], "OX-abc");
        assert_eq!(json["server_evidence_M2"], "ff");
        // Unset members are null rather than absent.
        assert!(json["error_code"].is_null());
        assert!(json["support_error_code"].is_null());
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
