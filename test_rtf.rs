use std::path::Path;
use coi::parser::DocumentParser;

fn main() {
    let parser = DocumentParser::new();
    let result = parser.parse(Path::new("test-data/knowledge_base/blockchain.rtf")).unwrap();
    println!("Parsed content:\n{}", result.content);
}
