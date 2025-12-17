use super::json_helpers::{get_number_field, get_string_field, insert_number, insert_string};
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SdpOfferMsg {
    pub call_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub sdp: String,
}

impl SdpOfferMsg {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let call_id = get_string_field(obj, "call_id")?;
        let from_user_id = get_string_field(obj, "from_user_id")?;
        let to_user_id = get_string_field(obj, "to_user_id")?;
        let sdp = get_string_field(obj, "sdp")?;
        Ok(SdpOfferMsg {
            call_id,
            from_user_id,
            to_user_id,
            sdp,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = HashMap::new();
        insert_string(&mut obj, "call_id", self.call_id.clone());
        insert_string(&mut obj, "from_user_id", self.from_user_id.clone());
        insert_string(&mut obj, "sdp", self.sdp.clone());
        JsonValue::Object(obj)
    }
}

#[derive(Debug, Clone)]
pub struct SdpAnswerMsg {
    pub call_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub sdp: String,
}

impl SdpAnswerMsg {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let call_id = get_string_field(obj, "call_id")?;
        let from_user_id = get_string_field(obj, "from_user_id")?;
        let to_user_id = get_string_field(obj, "to_user_id")?;
        let sdp = get_string_field(obj, "sdp")?;
        Ok(SdpAnswerMsg {
            call_id,
            from_user_id,
            to_user_id,
            sdp,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = HashMap::new();
        insert_string(&mut obj, "call_id", self.call_id.clone());
        insert_string(&mut obj, "from_user_id", self.from_user_id.clone());
        insert_string(&mut obj, "sdp", self.sdp.clone());
        JsonValue::Object(obj)
    }
}

#[derive(Debug, Clone)]
pub struct IceCandidateMsg {
    pub call_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub candidate: String,
    pub sdp_mid: String,
    pub sdp_mline_index: u32,
}

impl IceCandidateMsg {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let call_id = get_string_field(obj, "call_id")?;
        let from_user_id = get_string_field(obj, "from_user_id")?;
        let to_user_id = get_string_field(obj, "to_user_id")?;
        let candidate = get_string_field(obj, "candidate")?;
        let sdp_mid = get_string_field(obj, "sdp_mid")?;
        let sdp_mline_index = get_number_field(obj, "sdp_mline_index")? as u32;
        Ok(IceCandidateMsg {
            call_id,
            from_user_id,
            to_user_id,
            candidate,
            sdp_mid,
            sdp_mline_index,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = HashMap::new();
        insert_string(&mut obj, "call_id", self.call_id.clone());
        insert_string(&mut obj, "from_user_id", self.from_user_id.clone());
        insert_string(&mut obj, "candidate", self.candidate.clone());
        insert_string(&mut obj, "sdp_mid", self.sdp_mid.clone());
        insert_number(&mut obj, "sdp_mline_index", self.sdp_mline_index as f64);
        JsonValue::Object(obj)
    }
}

#[derive(Debug, Clone)]
pub struct HangupMsg {
    pub call_id: String,
}

impl HangupMsg {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let call_id = get_string_field(obj, "call_id")?;
        Ok(HangupMsg { call_id })
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = HashMap::new();
        insert_string(&mut obj, "call_id", self.call_id.clone());
        JsonValue::Object(obj)
    }
}
