
use serde_json;
use serde::*;


#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub name: String,
    pub description: String,
    pub actions: Vec<Action>,
}

impl Slide {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            actions: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Action {
    pub text: String,
    pub target_slide: String,
}
