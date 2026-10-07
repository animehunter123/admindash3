use serde::*;
use serde_json::*;
use anyhow::Result;


use serde_json::Value;

fn main() {
    let source_json_file = "/tmp/data_buttons.json";

    let fsdata = std::fs::read_to_string(source_json_file)
        .expect("couldnt read file data_buttons.json");

    let mut deserializer = serde_json::Deserializer::from_str(&fsdata);
    let rawjson: Result<Value, _> = serde_path_to_error::deserialize(&mut deserializer);

    match rawjson {
        Ok(_) => println!("Parsed successfully"),
        Err(err) => {
            // 1. Convert the path to an owned String right away to release the borrow on `err`
            let path_string = err.path().to_string();
            
            // 2. Now it is completely safe to consume `err` with `.into_inner()`
            let json_err = err.into_inner();
            let line_num = json_err.line();
            let col_num = json_err.column();

            eprintln!("❌ JSON Error in file: {}", source_json_file);
            eprintln!("   Path to error: .{}", path_string);
            eprintln!("   Message      : {} at line {}, column {}", json_err, line_num, col_num);
            eprintln!("------------------------------------------------------------");

            // Print the actual offending line visually
            if line_num > 0 {
                if let Some(line_text) = fsdata.lines().nth(line_num - 1) {
                    eprintln!("{:4} | {}", line_num, line_text);
                    // Print a caret pointer pointing exactly to the column
                    let padding = " ".repeat(col_num.saturating_sub(1));
                    eprintln!("     | {}^--- error here", padding);
                }
            }
        }
    }
}

