extern crate tantivy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
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
use std::{thread, time};
extern crate rand;
use rand::Rng;

fn start_py_shell_process() -> Child {
    let py_sh_process = Command::new("python3").arg("-i").arg("-")
                                 .stdin(Stdio::piped())
                                 .stdout(Stdio::piped())
                                 .stderr(Stdio::inherit())
                                 .spawn().unwrap();
    return py_sh_process;
}

fn read_document_json(file: &Path, schema: &Schema, index_writer: &IndexWriter) {
    if let Ok(lines) = read_lines(file) {
        for line_res in lines {
            if let Ok(line) = line_res {
                //println!("{}", line);
                let doc = schema.parse_document(&line).unwrap();
                index_writer.add_document(doc);
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
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(Stemmer::new(tantivy::tokenizer::Language::English))
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
    
    let link_options = TextOptions::default()
        .set_stored();

    schema_builder.add_text_field("title", text_options.clone());
    schema_builder.add_text_field("body", text_options.clone());
    schema_builder.add_text_field("link", link_options);

    let schema = schema_builder.build();
    return schema;
}

struct UserInfo {
    name: String,
    student_type: String,
    year: String,
}

fn ted_dialog(dialog: &str) {
    //let waits = (500, 150, 50);
    let waits = (0, 0, 0);
    let mut rng = rand::thread_rng();
    print!("{}: ", "Ted".red().bold());
    io::stdout().flush().unwrap();
    let timing0 = time::Duration::from_millis(waits.0+ rng.gen_range(0, 100));
    thread::sleep(timing0);
    for c in dialog.chars() {
        if c == ' ' {
            let timing = time::Duration::from_millis(waits.1 + rng.gen_range(0, 50));
            thread::sleep(timing);
        } else {
            let timing = time::Duration::from_millis(waits.2 + rng.gen_range(0, 50));
            thread::sleep(timing);
        }
        print!("{}", c);
        io::stdout().flush().unwrap();
    }
    thread::sleep(timing0);
    println!();
}

fn machine_dialog(dialog: &str) {
    let mut dots = String::new();
    let timing0 = time::Duration::from_millis(500);
    for i in 0..4 {
        print!("{}: {}", "Machine".yellow().bold(), dots);
        io::stdout().flush().unwrap();
        dots += ".";
        print!("\r");
        io::stdout().flush().unwrap();
        thread::sleep(timing0);
    }
    print!("{}: ", "Machine".yellow().bold());
    println!("{}", dialog);
    thread::sleep(timing0);
}

fn user_dialog(name: &str) -> String {
    print!("{}: ", name.green().bold());
    io::stdout().flush().unwrap();
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    let line = iterator.next().unwrap().unwrap().trim().to_string();
    return line;
}

fn introduce() -> UserInfo {
    ted_dialog("Hello! My name is Ted.");
    ted_dialog("I will be assisting you with your advising needs today!");
    ted_dialog("First, let me start some things up for us.");
    machine_dialog("Spooling Document Index.");
    machine_dialog("Bootstraping Knowledge Model.");
    ted_dialog("Okay, while that warms up, lets learn a bit about you.");
    ted_dialog("What is your first name?");
    let name = user_dialog("You");
    ted_dialog(&format!("Nice to meet you, {}!", name));
    ted_dialog(&format!("Are you an undergraduate or graduate student?"));
    let student_type = user_dialog(&name);
    ted_dialog(&format!("What year are you, {}?", name));
    let year = user_dialog(&name);
    ted_dialog(&format!("Okay, we are ready to start."));
    ted_dialog(&format!("What questions do you have for me today?"));
    let user_info: UserInfo = UserInfo { 
        name: name,
        student_type: student_type,
        year: year,
    };
    return user_info;
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
        read_document_json(&path_buf, &schema, &index_writer);
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

    
        let mut load_line = String::new();
        py_reader.read_line(&mut load_line).unwrap();
        println!("Loading!");
        
        let user_info = introduce();
        
        let mut ready_line = String::new();
        py_reader.read_line(&mut ready_line).unwrap();
        println!("Ready!");

        loop {
            let query_line = user_dialog(&user_info.name);
            if query_line == "quit" {
                break;
            }
            let query = query_parser.parse_query(&query_line).expect("Failed to parse query!");
            let top_docs = searcher.search(&query, &TopDocs::with_limit(5)).expect("Query failed!");
            let mut answers: Vec<Value> = Vec::new();
            println!("{}", "Documents".blue());
            for (score, doc_address) in top_docs {
                println!("{}", "---".blue());
                let retrieved_doc = searcher.doc(doc_address).expect("Unable to retrieve document!");

                let doc_bodies = retrieved_doc.get_all(body);

                let mut doc_text = String::new();
                for body_val in doc_bodies {
                    doc_text += body_val.text().unwrap();
                    doc_text += "\n";
                }
                doc_text = doc_text.trim().to_string();

                println!("Doc: {}\n{}", schema.to_json(&retrieved_doc), score);
                println!("Doc Text: {}", doc_text);
                
                py_writer.write(format!("predict(\"\"\"{}\"\"\", \"\"\"{}\"\"\")\n", doc_text, query_line).as_bytes()).unwrap();
                
                let mut answer_line = String::new();
                let answer_len = py_reader.read_line(&mut answer_line).unwrap();

                answer_line.truncate(answer_len - 1);
                let answer_val: Value = serde_json::from_str(&answer_line).unwrap();
                answers.push(answer_val);
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
