extern crate tantivy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::query::Query;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::DocAddress;
use tantivy::Score;
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
use std::process::ChildStdin;
use std::process::Stdio;
use std::io::BufReader;
use std::io::Result;
use std::convert::TryInto;
use serde_json::{Value};
extern crate colored;
use colored::*;
extern crate stopwords;
use stopwords::{SkLearn, Language, Stopwords};
use std::{thread, time};
extern crate rand;
use rand::Rng;

fn start_py_shell_process() -> Child {
    let py_sh_process = Command::new("./venv/bin/python").arg("-i").arg("-")
                                 .stdin(Stdio::piped())
                                 .stdout(Stdio::piped())
                                 .stderr(Stdio::null())
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
    for stopword in SkLearn::stopwords(Language::English).unwrap().iter() {
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

#[allow(dead_code)]
struct UserInfo {
    name: String,
    student_type: String,
    year: String,
}

fn ted_dialog(dialog: &str) {
    let mut waits = (400, 100, 50);
    if cfg!(debug_assertions) {
        waits = (0, 0, 0);
    }

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
    for _ in 0..4 {
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
    ted_dialog(&format!("Okay, we are ready to start. When you are done just say \"quit\" or \"(oo)\"."));
    ted_dialog(&format!("What questions do you have for me today?"));
    let user_info: UserInfo = UserInfo { 
        name: name,
        student_type: student_type,
        year: year,
    };

    return user_info;
}

fn generate_query(query_str: &str, index: &Index) -> Box<dyn Query> {
    let schema = index.schema();
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    
    let query_parser = QueryParser::for_index(&index, vec![title, body]);
    let query = query_parser.parse_query(query_str).expect("Failed to parse query!");
    return query;
}

fn query_index_for_docs(query: Box<dyn Query>, index: &Index) -> Vec<(Score, DocAddress)> {
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().expect("Could not build reader!");

    let searcher = reader.searcher();
    
    // println!("Query: \n{:#?}", query);
    let top_docs = searcher.search(&query, &TopDocs::with_limit(5)).expect("Query failed!");
    return top_docs;
}

fn collect_predicted_answers(query_str: &str, docs: &Vec<(Score, DocAddress)>, index: &Index, py_reader: &mut std::io::BufReader<&mut std::process::ChildStdout>, py_writer: &mut ChildStdin) -> Vec<(DocAddress, Value)> {
    let schema = index.schema();
    let body = schema.get_field("body").unwrap();
    
    let mut answers: Vec<(DocAddress, Value)> = Vec::new();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().expect("Could not build reader!");
    
    let searcher = reader.searcher();

    for (score, doc_address) in docs {
        let retrieved_doc = searcher.doc(*doc_address).expect("Unable to retrieve document!");

        let doc_bodies = retrieved_doc.get_all(body);

        let mut doc_text = String::new();
        for body_val in doc_bodies {
            doc_text += body_val.text().unwrap();
            doc_text += "\n";
        }
        doc_text = doc_text.trim().to_string();

        if cfg!(debug_assertions) {
            println!("Doc: {}\n{}", schema.to_json(&retrieved_doc), score);
            println!("Doc Text: \n{}", doc_text);
            //println!("Score: {}", query.explain(&searcher, doc_address).unwrap().to_pretty_json());
        }
        
        py_writer.write(format!("predict(\"\"\"{}\"\"\", \"\"\"{}\"\"\")\n", doc_text, query_str).as_bytes()).unwrap();
        
        let mut answer_line = String::new();
        let answer_len = py_reader.read_line(&mut answer_line).unwrap();

        answer_line.truncate(answer_len - 1);
        let answer_val: Value = serde_json::from_str(&answer_line).unwrap();
        
        if answer_val["confidence"].as_f64().unwrap() >= 0.1 {
            answers.push((*doc_address, answer_val));
        }
    }
    answers.sort_by(|a0, a1| a1.1["confidence"].as_f64().unwrap().partial_cmp(&a0.1["confidence"].as_f64().unwrap()).unwrap());
    return answers;   
}

fn display_answer(answer_val: &(DocAddress, Value), index: &Index) {
    let doc_vec = answer_val.1["document"].as_array().unwrap();
    let ans_start: usize = answer_val.1["start"].as_u64().unwrap().try_into().unwrap();
    let ans_end: usize = answer_val.1["end"].as_u64().unwrap().try_into().unwrap();
   
    let schema = index.schema();
    let link = schema.get_field("link").unwrap();
    let title = schema.get_field("title").unwrap();

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().expect("Could not build reader!");

    let searcher = reader.searcher();
    let retrieved_doc = searcher.doc(answer_val.0).expect("Unable to retrieve document!");
    let source_link = retrieved_doc.get_first(link).unwrap();
    let doc_titles = retrieved_doc.get_all(title);
 
    for i in 0..doc_vec.len() {
        let word = doc_vec[i].as_str().unwrap();
        if i >= ans_start && i <= ans_end {
            print!("{} ", word.green().bold());
        } else {
            print!("{} ", word);
        }
    }
    println!();
    
    let mut doc_title_crumb = String::new();
    for i in 0..doc_titles.len() {
        doc_title_crumb += &format!("[ {} ]", doc_titles[i].text().unwrap());
        if i < (doc_titles.len() - 1) {
            doc_title_crumb += " -> ";
        }
    }
    
    println!("[ Document: {} ]", doc_title_crumb);
    println!("[ Source: [ {} ] ]", source_link.text().unwrap().blue().underline());
    
    #[cfg(debug_assertions)]
    println!("{:?}\n{} {:.3}",  answer_val.1,  "Confidence:".yellow(), answer_val.1["confidence"].as_f64().unwrap()); 
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
        
        #[cfg(debug_assertions)]
        println!("File: {}", path_buf.display());
        
        read_document_json(&path_buf, &schema, &index_writer);
    }

    index_writer.commit().unwrap();
    
   
    {
        let mut py_reader = BufReader::new(py_sh_process.stdout.as_mut().unwrap());
        let py_writer = py_sh_process.stdin.as_mut().unwrap();
        
        writeln!(py_writer, "{}", py_script_content)?;
        py_writer.flush().unwrap(); 
    
        let mut load_line = String::new();
        py_reader.read_line(&mut load_line).unwrap();
        
        let user_info = introduce();
        
        let mut ready_line = String::new();
        py_reader.read_line(&mut ready_line).unwrap();

        loop {
            let query_line = user_dialog(&user_info.name);
            if query_line == "quit" || query_line == "(oo)" {
                break;
            }

            let query = generate_query(&query_line, &index); 
            let docs = query_index_for_docs(query, &index);

            ted_dialog("Give me a few moments to consider your question.");

            if docs.len() == 0 {
                ted_dialog("I don't think I have knowledge of what you have requesting. Try rewording your query.");
                continue;
            }

            let answers = collect_predicted_answers(&query_line, &docs, &index, &mut py_reader, py_writer);
            if answers.len() == 0 {
                ted_dialog("I am not confident I could give you the correct answer to your query. Try rephrasing it or giving more context.");
            } else {
                ted_dialog("Here is an answer I found to your query:");
                display_answer(&answers[0], &index);
                ted_dialog("Was this information helpful? (y/n/_)");
                let helpful = user_dialog(&user_info.name);
                if helpful == "y" {
                    ted_dialog("Thanks! I am so glad I could help!");
                } else if helpful == "n" {
                    if answers.len() > 1 {
                        ted_dialog("Okay. Here are several more results I found:");
                        for i in  1..answers.len() {
                            println!("Answer #{}:", i);
                            display_answer(&answers[i], &index);
                        }
                        ted_dialog("Which of these answers was most relevant to your query? (#)");
                        user_dialog(&user_info.name);
                    } else {
                        ted_dialog("I am sorry I could not be more helpful.");
                    }
                }
                ted_dialog("What other questions do you have for me?");
            }
        }
        
        writeln!(py_writer, "exit()")?;
    }
    
    ted_dialog("Goodbye!");
    machine_dialog("Exiting!");
    py_sh_process.kill()?;
    py_sh_process.wait()?;

    return Ok(());
}
