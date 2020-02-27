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
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::iter::Iterator;
use tantivy::IndexWriter;
use std::process::Command;
use std::process::Child;
use std::process::Stdio;
use std::io::BufReader;
use std::io::Result;
use std::convert::TryInto;
use serde_json::{Value};
extern crate colored;
use colored::*;
extern crate stopwords;
use stopwords::{NLTK, Language, Stopwords};

fn start_py_shell_process() -> Child {
    let mut py_sh_process = Command::new("python3").arg("-i").arg("-")
                                 .stdin(Stdio::piped())
                                 .stdout(Stdio::piped())
                                 .stderr(Stdio::piped())
                                 .spawn().unwrap();
    return py_sh_process;
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
    let mut stopword_vec: Vec<String> = Vec::new();
    for stopword in NLTK::stopwords(Language::English).unwrap().iter() {
        stopword_vec.push(stopword.to_string());
    }
    let tokenizer = SimpleTokenizer
        .filter(LowerCaser)
        .filter(StopWordFilter::remove(
            stopword_vec
        ));

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


fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let paths = fs::read_dir(&args[1]).unwrap();

    let py_script_content = include_str!("../scripts/bert_test.py");

    // Start Python shell.
    let mut py_sh_process = start_py_shell_process();

    
    // Setup document index and ingest documents.
    let schema = setup_schema();
    let index = setup_index(&schema);
    let mut index_writer = index.writer(50_000_000).expect("Unable to make index writer!");
    for path in paths {
        let path_buf = path.unwrap().path();
        println!("File: {}", path_buf.display());
        read_document(&path_buf, &schema, &index_writer);
    }

    index_writer.commit().unwrap();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().expect("Could not build reader!");

    let searcher = reader.searcher();
    
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    let query_parser = QueryParser::for_index(&index, vec![title, body]);
   
    {
        let mut py_reader = BufReader::new(py_sh_process.stdout.as_mut().unwrap());
        let py_writer = py_sh_process.stdin.as_mut().unwrap();
        
        writeln!(py_writer, "{}", py_script_content)?;
        py_writer.flush().unwrap(); 
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let query_line = line.unwrap(); 
            let query = query_parser.parse_query(&query_line).expect("Failed to parse query!");
            println!("Query: {} {:?}", &query_line, query);

            let top_docs = searcher.search(&query, &TopDocs::with_limit(5)).expect("Query failed!");
            let mut answers: Vec<Value> = Vec::new();
            println!("{}", "Documents".blue());
            for (score, doc_address) in top_docs {
                println!("{}", "---".blue());
                let retrieved_doc = searcher.doc(doc_address).expect("Unable to retrieve document!");

                let doc_body = retrieved_doc.get_first(body).unwrap().text().unwrap();
                println!("Doc: {}\n{}", schema.to_json(&retrieved_doc), score);
                
                py_writer.write(format!("predict(\"{}\", \"{}\")\n", doc_body, query_line).as_bytes()).unwrap();
                // py_writer.flush().unwrap();
                let mut answer_line = String::new();
                let answer_len = py_reader.read_line(&mut answer_line).unwrap();

                answer_line.truncate(answer_len - 1);
                let answer_val: Value = serde_json::from_str(&answer_line).unwrap();
                answers.push(answer_val);
                //bert_doc(doc_body, &query_line);
            }
            println!("{}", "---".blue());

            answers.sort_by(|a0, a1| a1["confidence"].as_f64().unwrap().partial_cmp(&a0["confidence"].as_f64().unwrap()).unwrap());
            println!("{}", "Answers".bright_red());
            for answer_val in answers {
                println!("{}", "---".red());
                let doc_vec = answer_val["document"].as_array().unwrap();
                let ans_start: usize = answer_val["start"].as_u64().unwrap().try_into().unwrap();
                let ans_end: usize = answer_val["end"].as_u64().unwrap().try_into().unwrap();
                for i in 0..doc_vec.len() {
                    let word = doc_vec[i].as_str().unwrap();
                    if i >= ans_start && i <= ans_end {
                        print!("{} ", word.green().bold());
                    } else {
                        print!("{} ", word);
                    }
                }
                println!();
                println!("{} {:.3}", "Confidence:".yellow(), answer_val["confidence"].as_f64().unwrap()); 
            }
            println!("{}", "---".red());
        }
        
        writeln!(py_writer, "exit()")?;
    }

    println!("Exiting!");
    py_sh_process.kill()?;
    py_sh_process.wait()?;

    return Ok(());
}
