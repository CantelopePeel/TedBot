#[macro_use]
extern crate tantivy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};
use tantivy::tokenizer::*;
use tantivy::ReloadPolicy;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use tantivy::IndexWriter;
use std::process::Command;
use std::str;

fn bert_doc(doc: &str, question: &str) -> () {
    let script_str = include_str!("../bert_test.py");
    let mut cmd = Command::new("python3");
    cmd.arg("bert_test.py")
        .arg(doc)
        .arg(question);
    println!("{}", str::from_utf8(&cmd.output().unwrap().stdout).unwrap());
}

fn read_document(file: &Path, schema: &Schema, index_writer: &IndexWriter) {
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    
    if let Ok(lines) = read_lines(file) {
        for line_res in lines {
            if let Ok(line) = line_res {
                //println!("{}", line);
                index_writer.add_document(doc!(
                    title => "",
                    body => line.to_string(),
                ));
            }
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
    let file = File::open(filename)?;
    return Ok(io::BufReader::new(file).lines());
}

fn setup_index(schema: &Schema) -> Index {
    let index = Index::create_in_ram(schema.clone());
    let tokenizer = SimpleTokenizer
        .filter(LowerCaser)
        .filter(StopWordFilter::remove(vec![
            // TODO: Read in a stopword list (sklearn has one, so does nltk I think).
            "the".to_string(),
            "and".to_string(),
        ]));

    index.tokenizers().register("stoppy", tokenizer);
    return index;
}

fn setup_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    
    let text_field_indexing = TextFieldIndexing::default()
        .set_tokenizer("stoppy")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default()
        .set_indexing_options(text_field_indexing)
        .set_stored();

    schema_builder.add_text_field("title", text_options);
    
    let text_field_indexing = TextFieldIndexing::default()
        .set_tokenizer("stoppy")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default()
        .set_indexing_options(text_field_indexing)
        .set_stored();
    schema_builder.add_text_field("body", text_options);

    let schema = schema_builder.build();
    return schema;
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let paths = fs::read_dir(&args[1]).unwrap();

    let schema = setup_schema();
    let index = setup_index(&schema);
    let mut index_writer = index.writer(50_000_000).expect("Unable to make index writer!");
    for path in paths {
        let path_buf = path.unwrap().path();
        println!("File: {}", path_buf.display());
        read_document(&path_buf, &schema, &index_writer);
    }


    index_writer.commit();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().expect("Could not build reader!");

    let searcher = reader.searcher();
    
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    let query_parser = QueryParser::for_index(&index, vec![title, body]);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let query_line = line.unwrap(); 
        println!("Query: {}", &query_line);
        let query = query_parser.parse_query(&query_line).expect("Failed to parse query!");

        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).expect("Query failed!");

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).expect("Unable to retrieve document!");

            let doc_body = retrieved_doc.get_first(body).unwrap().text().unwrap();
            println!("Doc: {}\n{}", schema.to_json(&retrieved_doc), score);
            bert_doc(doc_body, &query_line);
        }
    }

}
