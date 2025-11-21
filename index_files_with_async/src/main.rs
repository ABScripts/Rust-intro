use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{fs, thread};

#[derive(Parser)]
#[command(version, about = "Count word occurences in files")]
struct Args {
    path: PathBuf,
    indexed_files_output_path: PathBuf,
}

#[derive(Debug)]
struct IndexResults {
    filename: PathBuf,
    index: HashMap<String, Vec<usize>>,
}

async fn index_files(filepath: PathBuf, tx_end: tokio::sync::mpsc::Sender<IndexResults>) {
    println!(
        "Start indexing files at thread id {:?}",
        thread::current().id()
    );

    println!("Processing {:?}", filepath);
    match index_file(&filepath) {
        Ok(word_pos) => {
            tx_end
                .send(IndexResults {
                    filename: filepath,
                    index: word_pos,
                })
                .await
                .unwrap();
        }
        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
            println!("Skip {:?} - Invalid UTF8", filepath)
        }
        Err(e) => {
            println!("Skip {:?} - {e}", filepath)
        }
    }
}

async fn collect_files_index(
    mut rx_end: tokio::sync::mpsc::Receiver<IndexResults>,
    out_path: PathBuf,
) {
    let mut index: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();

    println!("Start collecting results at {:?}", thread::current().id());

    while let Some(result) = rx_end.recv().await {
        let filename = result.filename;
        let word_pos = result.index;
        for (word, positions) in word_pos {
            let entry = index.entry(word.to_string()).or_insert_with(HashMap::new);

            let filepath = fs::canonicalize(filename.to_path_buf())
                .unwrap()
                .to_string_lossy()
                .to_string();

            entry
                .entry(filepath)
                .or_insert_with(Vec::new)
                .extend(positions);
        }
    }

    match serde_json::to_string(&index) {
        Ok(index_json) => match fs::write(&out_path, index_json) {
            Err(e) => eprintln!("Failed to preserve index to {:?}: {e}", out_path),
            Ok(_) => {}
        },
        Err(e) => {
            eprintln!("Failed to convert index to JSON: {e}");
            return;
        }
    };
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();
    fs::exists(&args.path)?;
    fs::exists(&args.indexed_files_output_path)?;

    let start_time = Instant::now();

    let mut files = Vec::new();
    collect_files(&args.path, &mut files)?;

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let task_collect_files_index = tokio::spawn(collect_files_index(
        rx,
        args.indexed_files_output_path, // Q: can't be passed by reference, why?
    ));

    for file in files {
        // What would happen if we have only single thread (so that tokio can't use multiple threads)?
        tokio::spawn(index_files(file, tx.clone()));
    }
    drop(tx);

    task_collect_files_index
        .await
        .expect("Failed to collect indexed files results");

    let duration = start_time.elapsed();
    println!("Done indexing files in {} seconds", duration.as_secs_f64());

    Ok(())
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            collect_files(&entry_path, files)?;
        }
    }
    Ok(())
}

fn index_file(path: &Path) -> io::Result<HashMap<String, Vec<usize>>> {
    let mut file = fs::File::open(path)?;
    let mut data = String::new();

    match file.read_to_string(&mut data) {
        Ok(_) => return Ok(get_word_positions(&data)),
        Err(e) => return Err(e),
    }
}

fn get_word_positions(string_to_index: &str) -> HashMap<String, Vec<usize>> {
    let mut indexed_string = HashMap::<String, Vec<usize>>::new();

    let mut word_byte_index = 0;
    for word in string_to_index.split(|c: char| c.is_ascii_punctuation() || c.is_ascii_whitespace())
    {
        if !word.is_empty() {
            indexed_string
                .entry(word.to_string())
                .or_default()
                .push(word_byte_index);
            word_byte_index += word.len();
        }

        // account for the delimeter just past the current word
        // OR
        // we get empty strings for each two adjacent delimeters
        // increase for the second adjacent delimeter
        word_byte_index += 1;
    }

    indexed_string
}
