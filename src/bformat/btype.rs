use std::collections::HashMap;

pub enum BType {
    Bytes(Vec<u8>),
    Number(i128),
    List(Vec<Box<BType>>),
    Map(HashMap<String, Box<BType>>),
}

impl BType {
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            BType::Bytes(bytes) => {
                let string_result = String::from_utf8(bytes.clone());
                if string_result.is_ok() {
                    serde_json::Value::String(string_result.unwrap())
                } else {
                    serde_json::Value::String(hex::encode(bytes))
                }
            }
            BType::Number(number) => {
                serde_json::Value::Number(serde_json::Number::from_i128(*number).unwrap())
            }
            BType::List(list) => {
                let mut converted_list = Vec::new();
                for btype in list {
                    converted_list.push(btype.to_json_value());
                }
                serde_json::Value::Array(converted_list)
            }
            BType::Map(map) => {
                let mut converted_map = serde_json::Map::new();
                for (key, btype) in map {
                    converted_map.insert(key.clone(), btype.to_json_value());
                }
                serde_json::Value::Object(converted_map)
            }
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            BType::Bytes(bytes) => String::from_utf8(bytes.clone()).unwrap(),
            BType::Number(number) => format!("{}", number),
            BType::List(list) => {
                let mut string = "[".to_owned();
                for item in list {
                    string.push_str(item.to_string().as_str());
                    string.push_str(",");
                }
                string.pop();
                string.push_str("]");
                string
            }
            BType::Map(map) => {
                let mut string = "{".to_owned();
                for (key, value) in map {
                    string.push_str(key.as_str());
                    string.push_str(":");
                    string.push_str(value.to_string().as_str());
                    string.push_str(",");
                }
                string.pop();
                string.push_str("}");
                string
            }
        }
    }

    pub fn as_bytes(&self) -> Option<&Vec<u8>> {
        match self {
            BType::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&i128> {
        match self {
            BType::Number(number) => Some(number),
            _ => None,
        }
    }

    // pub fn as_list(&self) -> Option<&Vec<Box<BType>>> {
    //     match self {
    //         BType::List(list) => Some(list),
    //         _ => None,
    //     }
    // }

    pub fn as_map(&self) -> Option<&HashMap<String, Box<BType>>> {
        match self {
            BType::Map(map) => Some(map),
            _ => None,
        }
    }
}
