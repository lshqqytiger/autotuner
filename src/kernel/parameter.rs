use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

#[derive(Serialize, Deserialize)]
pub struct Restriction {
    even_number: bool,
    range: Range,
    condition: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Value {
    Integer(i32, (i32, i32)),
    Switch(bool),
}

#[derive(Serialize, Deserialize)]
pub struct Parameter {
    restriction: Restriction,
    value: Value,
}
