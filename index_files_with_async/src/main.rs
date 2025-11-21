use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::{fs, thread};
use tokio::sync::Mutex;

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

async fn index_files(
    job_queue: Arc<Mutex<Vec<PathBuf>>>,
    tx_end: tokio::sync::mpsc::Sender<IndexResults>,
) {
    println!(
        "Start indexing files at thread id {:?}",
        thread::current().id()
    );

    loop {
        let Some(filename) = job_queue.lock().await.pop() else {
            break; // shut down
        };

        println!("Processing {:?}", filename);
        match index_file(&filename) {
            Ok(word_pos) => {
                tx_end
                    .send(IndexResults {
                        filename,
                        index: word_pos,
                    })
                    .await
                    .unwrap();
            }
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                println!("Skip {:?} - Invalid UTF8", filename)
            }
            Err(e) => {
                println!("Skip {:?} - {e}", filename)
            }
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
    let shared_files = Arc::new(Mutex::new(files));

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let threads_num = match std::thread::available_parallelism() {
        Ok(available_threads) => available_threads.get(),
        Err(e) => {
            println!("Failed to get available num of threads: {e}");
            12
        }
    }
    .sub(1)
    .max(1);

    let mut join_handles = Vec::new();
    for _ in 0..threads_num {
        join_handles.push(tokio::spawn(index_files(shared_files.clone(), tx.clone())));
    }
    drop(tx); // important to drop. Otherwise, this tx will be active, and rx end will stuck

    join_handles.push(tokio::spawn(collect_files_index(
        rx,
        args.indexed_files_output_path, // Q: can't be passed by reference, why?
    )));

    // wait for all the tasks to finish
    println!("Just before joining the tasks");
    for handle in join_handles {
        handle.await.unwrap();
        // Q: What is the difference between join and await??
        // "join" and similar methods allow to wait for multiple handlers at the same time
        // we can still use await as we are inside async block anyways
    }
    println!("Right after joining the tasks");

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
