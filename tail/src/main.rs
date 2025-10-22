use std::{
    cmp::min,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, default_value_t = 10)]
    n: u64,

    filename: String,
}

const READ_BUFFER_SIZE: usize = 4096;

/* Buffered approach:
 * 1. Seek to the end of the file.
 * 2. Crawl backwards in chunks of `READ_BUFFER_SIZE` bytes.
 * 3. In each chunk, search for newline characters (`\n`).
 * 4. Accumulate the relevant text into a final buffer that will contain the last N lines.
 */
fn get_last_n_lines(filename: &String, n: u64) -> String {
    let mut file = match File::open(filename) {
        Err(why) => panic!("Failed to open {}: {}", &filename, why),
        Ok(file) => file,
    };

    file.seek(SeekFrom::End(0)).unwrap();

    let mut file_size = file.stream_position().unwrap();
    let mut tailed_lines = 0;
    let mut buf = [0; READ_BUFFER_SIZE];
    let mut tailed_output = String::new();
    while tailed_lines < n && file_size > 0 {
        let read_size = min(file_size, READ_BUFFER_SIZE as u64);
        // get next portion of data to read (crawl backwards)
        file.seek(SeekFrom::Current(-(read_size as i64))).unwrap();

        // adjust buf to the desired amount of data to read (max READ_BUFFER_SIZE)
        // size of this slice automatically lets 'read_exact' how many bytes to read
        let buf_slice = &mut buf[..read_size as usize];
        match file.read_exact(buf_slice) {
            Err(e) => {
                let pos = file.stream_position();
                panic!("Couldn't read data from position {pos:?}, err: {e}");
            }
            _ => (),
        }

        // Try to find newlines and mark amount of data that we would want to read from this buf slice
        // Sometimes it can be lass than its max size as we may already found all sentences in this batch
        let mut text_slice = &buf_slice[..];
        for (pos, ch) in buf_slice.iter().rev().enumerate() {
            if matches!(ch, b'\n') {
                tailed_lines += 1;

                if tailed_lines > n {
                    // subtract '-1' as we don't want to account for newline char which belongs just to string above us (11th)
                    text_slice = &buf_slice[(read_size as usize - pos - 1)..];
                    break;
                }
            }
        }

        // NOTE: Seems that we can also pass &text_slice here - no difference?
        if let Ok(str) = str::from_utf8(text_slice) {
            // TODO: It doesn't seem to be efficient...
            tailed_output = format!("{}{}", str, tailed_output);
        }

        // get backwards once again as file pointer was moved by previous 'read_exact' call
        file_size = file.seek(SeekFrom::Current(-(read_size as i64))).unwrap();
    }

    // TODO: This method was returning 'tailed_output' even without any explicit instructions
    // WHY?
    tailed_output
}

fn main() {
    let args = Args::parse();

    let tailed_output = get_last_n_lines(&args.filename, args.n);
    print!("{tailed_output:?}");
}
