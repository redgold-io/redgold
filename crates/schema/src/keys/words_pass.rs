use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordsPass {
    pub words: String,
    pub passphrase: Option<String>,
}

impl WordsPass {

    pub fn new(words: impl Into<String>, passphrase: Option<String>) -> Self {
        Self {
            words: words.into(),
            passphrase,
        }
    }

}