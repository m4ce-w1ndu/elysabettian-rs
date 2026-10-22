use ely_core::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    String(String),
    Array(Vec<Value>),
}
