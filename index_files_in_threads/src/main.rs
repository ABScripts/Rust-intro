use clap::Parser;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Parser)]
#[command(version, about = "Count word occurences in files")]
struct Args {
    path: PathBuf,
}

#[derive(Debug)]
struct IndexResults {
    filename: PathBuf,
    index: HashMap<String, Vec<usize>>,
}

struct ThreadedFileIndexer {
    worker_threads: Vec<JoinHandle<()>>,
    rx: Receiver<IndexResults>, // this Receiver will be used by user of the threaded file indexer
                                // to retrieve results of processing
                                // tx: Sender<HashMap<String, Vec<usize>>>, // Sender will be cloned to each of the worker threads and will be used to send results
}

impl ThreadedFileIndexer {
    fn new(path: &PathBuf) -> io::Result<ThreadedFileIndexer> {
        let mut files = Vec::new();
        collect_files(path, &mut files)?;

        let threads_num = num_cpus::get() - 1; // account for the main thread which is already running
        eprintln!("Will spawn {} additional threads", threads_num);

        let shared_job_queue = Arc::new(Mutex::new(files));
        let (tx, rx) = mpsc::channel();
        // this tx will be dropped as we are done with initializing ThreadedFileIndexer object

        let mut threads = Vec::new();
        for i in 0..threads_num {
            let job_queue = shared_job_queue.clone();
            let tx = tx.clone();
            threads.push(std::thread::spawn(move || {
                let thread_id = i;
                eprintln!("Spawned thread {thread_id}");
                loop {
                    let Some(filename) = job_queue.lock().unwrap().pop() else {
                        break; // shut down this thread if "job_queue" is empty
                    };

                    match index_file(&filename) {
                        Ok(word_pos) => {
                            tx.send(IndexResults {
                                filename,
                                index: word_pos,
                            })
                            .unwrap_or_else(|e| {
                                println!("Failed to send results from {thread_id}: {e}")
                            });
                        }
                        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                            println!("Skip {:?} - Invalid UTF8", filename)
                        }
                        Err(e) => {
                            println!("Skip {:?} - {e}", filename)
                        }
                    }
                }
                // at this point our tx end will be dropped
                // as all threads shut down their tx ends, main thread will get its rx end closed automatically (it will receive notification)
            }));
        }

        return Ok(ThreadedFileIndexer {
            worker_threads: threads,
            rx,
        });
    }

    fn collect_results(self) -> io::Result<HashMap<String, HashMap<String, Vec<usize>>>> {
        let mut index: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();
        for result in self.rx.iter() {
            let filename = result.filename;
            let word_pos = result.index;
            println!("Received {:?}", filename);

            for (word, positions) in word_pos {
                let entry = index.entry(word.to_string()).or_insert_with(HashMap::new);

                let filepath = fs::canonicalize(filename.to_path_buf())?
                    .to_string_lossy()
                    .to_string();

                entry
                    .entry(filepath)
                    .or_insert_with(Vec::new)
                    .extend(positions);
            }
        }

        Ok(index)
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let start_time = Instant::now();

    let threaded_file_indexer = ThreadedFileIndexer::new(&args.path)?;
    let index = threaded_file_indexer.collect_results()?;

    eprintln!("Done");

    let duration = start_time.elapsed();
    println!("Indexing results: {:?}", index);
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
