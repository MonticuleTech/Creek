#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::env;

fn main() {
    let cwd = env::current_dir().unwrap_or_default();
    println!("Current Working Directory: {:?}", cwd);

    // Try multiple locations
    let possible_paths = vec![
        "../.env",
        ".env",
        "/Users/mayiran/Projects/creek/.env" 
    ];

    let mut loaded = false;
    for p in possible_paths {
        let path = Path::new(p);
        if path.exists() {
            println!("Found .env at: {:?}", path.canonicalize());
            if dotenv::from_path(path).is_ok() {
                println!("Successfully loaded .env from: {:?}", p);
                loaded = true;
                break;
            } else {
                println!("Failed to parse .env at: {:?}", p);
            }
        } else {
            println!(".env NOT found at: {:?}", p);
        }
    }

    if !loaded {
        println!("Trying standard dotenv lookup...");
        dotenv::dotenv().ok();
    }
    
    // Logger initialized by tauri-plugin-log in lib.rs
    
    // Verify immediate availability
    match env::var("OPENAI_API_KEY") {
        Ok(val) => println!("OPENAI_API_KEY is set (length: {})", val.len()),
        Err(_) => println!("OPENAI_API_KEY is NOT SET in main.rs"),
    }
    
    creek_lib::run()
}
