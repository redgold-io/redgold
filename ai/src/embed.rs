use async_openai::Client;
use async_openai::types::{CreateEmbeddingRequestArgs, CreateEmbeddingResponse, Embedding};
use redgold_schema::EasyJson;
use serde::{Deserialize, Serialize};
use crate::directory_code_reader::{AccumFileData, count_tokens, ScanConfig};

fn split_with_overlap(s: &str, substring_size: usize, overlap: usize) -> Vec<String> {
    let mut result = Vec::new();
    let chars: Vec<char> = s.chars().collect();

    for i in (0..chars.len()).step_by(substring_size - overlap) {
        let end = std::cmp::min(i + substring_size, chars.len());
        let substring: String = chars[i..end].iter().collect();
        result.push(substring);
        if end == chars.len() {
            break;
        }
    }

    result
}

#[test]
fn string_split() {
    let string = "abcdefghijklmnop";
    let substrings = split_with_overlap(string, 5, 2);
    println!("Substrings: {:?}", substrings);
}

#[derive(Serialize, Deserialize, Clone)]
struct EmbedInput {
    path: String,
    content: String,
    chunk_id: Option<usize>,
}

impl EmbedInput {
    fn tokens(&self) -> i64 {
        count_tokens(&self.content)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct EmbeddingData {
    input: EmbedInput,
    embedding_response: Embedding,
}

async fn get_git_diffs() -> Vec<(String, String)> {
    let git_diffs = vec![];
    git_diffs
}

#[ignore]
#[tokio::test]
async fn embed_test() {
    let client = Client::new();

    // An embedding is a vector (list) of floating point numbers.
    // The distance between two vectors measures their relatedness.
    // Small distances suggest high relatedness and large distances suggest low relatedness.

    let inputs = get_repository_file_embeddings();

    let mut all = vec![];

    for (i, input) in inputs.iter().enumerate() {
        println!("{}: {} {}", i, input.content.len(), input.path);

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-ada-002")
            .input(
                input.content.clone()
            )
            .build().expect("Failed to build request");

        let response = client.embeddings().create(request).await;
        let response = response.expect("response");

        if response.data.len() > 1 {
            println!("Excess Response data len {}", response.data.len());
        }
        println!("Response len {}", response.data.len());

        for data in response.data {
            let emb_data = EmbeddingData {
                input: input.clone(),
                embedding_response: data.clone()
            };
            all.push(emb_data);
            break;
        }
    }

    let out = all.json_or();
    // write output to a file:
    std::fs::write("embeddings.json", out).expect("Failed to write output");

}

fn get_repository_file_embeddings() -> Vec<EmbedInput> {
    let ac = AccumFileData::from_config(ScanConfig::new());

    let contents = ac.contents_path();

    get_embed_inputs(contents)
}

fn get_embed_inputs(contents: Vec<(String, String)>) -> Vec<EmbedInput> {
    let mut inputs = vec![];

    for (fnm, content) in contents {
        // let token_count = count_tokens(&content);
        let content_len = content.len();
        // let ratio = content_len as f64 / token_count as f64;
        // println!("ratio: {}, Tokens: {} str_len: {} {}", ratio, token_count, content_len, fnm);
        // println!("str_len: {} {}", content_len, fnm);
        // let ideal_count = 4000;
        let ideal_count = 16000;
        if content_len < ideal_count {
            let input = EmbedInput {
                path: fnm.clone(),
                content,
                chunk_id: None
            };
            inputs.push(input);
        } else {
            let fraction = (ideal_count as f64) / (content_len as f64);

            let substring_size = (content_len as f64 * fraction) as usize;
            let chunks = split_with_overlap(&content, substring_size, 100);
            for (i, chunk) in chunks.iter().enumerate() {
                let input = EmbedInput {
                    path: fnm.clone(),
                    content: chunk.clone(),
                    chunk_id: Some(i)
                };
                inputs.push(input);
            }
        }
    }
    inputs
}