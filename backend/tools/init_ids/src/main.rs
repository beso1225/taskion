use dotenvy::dotenv;
use std::env;

fn main() {
    dotenv().ok();

    let token = env::var("NOTION_TOKEN").expect("NOTION_TOKEN not set");

    println!("Notion Token: {}", &token[..8]);
}