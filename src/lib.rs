use crossbeam_channel::Sender;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;

pub fn start_password_reader(file_path: PathBuf, send_password: Sender<String>) -> JoinHandle<()> {
    thread::Builder::new()
        .name("password-reader".to_string())
        .spawn(move || {
            let file = File::open(file_path).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                match send_password.send(line.unwrap()) {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        })
        .unwrap()
}
