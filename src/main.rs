use std::env;

use zip_pw_finder::password_finder;

fn main() {
    let zip_path = env::args().nth(1).unwrap();
    println!("{zip_path}");
    let dictionary_path = "passwords/xato-net-10-million-passwords.txt";
    let workers = 3;

    match password_finder(&zip_path, dictionary_path, workers) {
        Some(password_found) => println!("Password found: {password_found}"),
        None => println!("No password found :("),
    }
}
