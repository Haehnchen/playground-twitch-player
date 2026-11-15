use super::{TwitchAuthDeviceCode, TwitchAuthToken};
use crate::twitch::json::{invalid_response, owned_string, parse, string, uint};
use gtk4::glib;
use serde_json::Value;

fn parse_json_object(json: &[u8]) -> Result<Value, glib::Error> {
    let value = parse(json)?;
    if !value.is_object() {
        return Err(invalid_response("Twitch returned invalid JSON"));
    }
    Ok(value)
}

pub(super) fn parse_auth_error_message(json: &[u8]) -> String {
    parse_json_object(json)
        .ok()
        .map(|value| {
            string(&value["message"])
                .filter(|s| !s.is_empty())
                .or_else(|| string(&value["error"]))
                .unwrap_or("")
                .to_owned()
        })
        .unwrap_or_default()
}

// Reject incomplete authentication responses.
pub(super) fn parse_device_code_response(
    json: &[u8],
) -> Result<*mut TwitchAuthDeviceCode, glib::Error> {
    let value = parse_json_object(json)?;
    if ["device_code", "user_code", "verification_uri"]
        .iter()
        .any(|key| string(&value[key]).is_none())
    {
        return Err(invalid_response("Twitch did not return a device code"));
    }
    let interval = uint(&value["interval"]);
    Ok(Box::into_raw(Box::new(TwitchAuthDeviceCode {
        device_code: owned_string(&value["device_code"]),
        user_code: owned_string(&value["user_code"]),
        verification_uri: owned_string(&value["verification_uri"]),
        expires_in: uint(&value["expires_in"]),
        interval: if interval == 0 { 5 } else { interval },
    })))
}

pub(super) fn parse_token_response(json: &[u8]) -> Result<*mut TwitchAuthToken, glib::Error> {
    let value = parse_json_object(json)?;
    if string(&value["access_token"]).is_none_or(str::is_empty) {
        return Err(invalid_response("Twitch did not return an access token"));
    }
    Ok(Box::into_raw(Box::new(TwitchAuthToken {
        access_token: owned_string(&value["access_token"]),
        refresh_token: owned_string(&value["refresh_token"]),
        expires_in: uint(&value["expires_in"]),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twitch::auth::{twitch_auth_device_code_free, twitch_auth_token_free};

    #[test]
    fn device_code_reads_fields_and_defaults_invalid_numbers() {
        let code = parse_device_code_response(br#"{"device_code":"device","user_code":"ABCD","verification_uri":"https://twitch.tv/activate","expires_in":600,"interval":2}"#).unwrap();
        unsafe {
            assert_eq!((*code).user_code.as_c_str(), c"ABCD");
            assert_eq!((*code).expires_in, 600);
            assert_eq!((*code).interval, 2);
            twitch_auth_device_code_free(code);
        }
        for fields in [
            "",
            ",\"expires_in\":-1,\"interval\":0",
            ",\"expires_in\":4294967296,\"interval\":\"2\"",
        ] {
            let json = format!(
                r#"{{"device_code":"d","user_code":"u","verification_uri":"url"{fields}}}"#
            );
            let code = parse_device_code_response(json.as_bytes()).unwrap();
            unsafe {
                assert_eq!((*code).expires_in, 0);
                assert_eq!((*code).interval, 5);
                twitch_auth_device_code_free(code);
            }
        }
    }

    #[test]
    fn token_reads_optional_refresh_token_and_unicode() {
        let token = parse_token_response(
            r#"{"access_token":"t\"öken","refresh_token":"refresh","expires_in":3600}"#.as_bytes(),
        )
        .unwrap();
        unsafe {
            assert_eq!(
                (*token).access_token.as_c_str().to_str().unwrap(),
                "t\"öken"
            );
            assert_eq!((*token).refresh_token.as_c_str(), c"refresh");
            assert_eq!((*token).expires_in, 3600);
            twitch_auth_token_free(token);
        }
        let token =
            parse_token_response(br#"{"access_token":"token","refresh_token":null}"#).unwrap();
        unsafe {
            assert!((*token).refresh_token.is_empty());
            assert_eq!((*token).expires_in, 0);
            twitch_auth_token_free(token);
        }
    }

    #[test]
    fn invalid_auth_responses_return_errors() {
        for json in [
            "{",
            "[]",
            "{}",
            r#"{"access_token":false}"#,
            r#"{"access_token":""}"#,
            r#"{"access_token":"\u0000hidden"}"#,
        ] {
            assert!(parse_token_response(json.as_bytes()).is_err());
        }
        assert!(parse_device_code_response(br#"{"device_code":"d","user_code":null}"#).is_err());
    }

    #[test]
    fn auth_errors_prefer_message_then_error_name() {
        for (json, expected) in [
            (r#"{"message":"Denied","error":"access_denied"}"#, "Denied"),
            (
                r#"{"message":"","error":"authorization_pending"}"#,
                "authorization_pending",
            ),
            ("{", ""),
        ] {
            assert_eq!(parse_auth_error_message(json.as_bytes()), expected);
        }
    }
}
