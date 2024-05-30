use clap::Parser;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct Response {
    response: String,
    context: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct ModelResponse {
    code: String,
    description: String,
    programming_language: String,
    extension: String,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whether to start a new context with the AI.
    #[arg(short, long)]
    new: bool,
    /// File to load code from
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Whether to save the model output into a file.
    #[arg(short, long)]
    save: bool,
    question: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    model: String,
    prompt: String,
    stream: bool,
    context: Option<Vec<i64>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let q = args.question.join(" ");

    let cfg_dir = Path::join(
        dirs::config_dir()
            .expect("Unable to get XDG_HOME")
            .as_path(),
        Path::new("kysy"),
    );
    let last_ctx_path = Path::join(&cfg_dir, Path::new("context.json"));
    if !cfg_dir.exists() {
        std::fs::create_dir(cfg_dir).expect("Unable to create .config/kysy");
    }
    let mut ctx = None;
    if last_ctx_path.exists() {
        // Use context.
        if !args.new {
            let ctx_str = std::fs::read_to_string(&last_ctx_path)
                .expect("Unable to read .config/kysy/context.json");
            let parsed: Vec<i64> =
                serde_json::from_str(&ctx_str).expect("Unable to parse context!");
            ctx = Some(parsed);
        }
    } else {
        std::fs::File::create(&last_ctx_path).expect("Unable to create context.json file.");
    }

    let prompt = r#"Only answer to me in JSON. The format you should respond to me in is as follows: {"code": string, "description": string, "programming_language": string, "extension": string}. You can use the description field to give me some information about a command. You will be answering programming questions. Use the programming_language field to respond to what language the "code" output is in. Use the "extension" field to output the file extension for the code without the dot. Remember to handle newlines correctly in the JSON and escape any characters you need to. You should always answer the description in the same user as the language is asking the question in. This is the user's question: "#;
    

    let mut file = String::new();
    if args.file.is_some() {
      file = std::fs::read_to_string(args.file.unwrap()).unwrap();
    }

    let prompt = format!("{prompt}, {} {}", q, file);

    let body: Response = ureq::post("http://localhost:11434/api/generate")
      .send_json(Request {
        model: "llama3".into(),
        prompt: prompt,
        stream: false,
        context: ctx
      })?
      .into_json()?;

    let mut ctx_file = std::fs::OpenOptions::new().write(true).truncate(true).open(last_ctx_path).expect("Unable to open context file for writing.");
    let out_json = serde_json::to_string(&body.context).unwrap();
    write!(ctx_file, "{}", out_json).expect("Unable to write to context file");

    let resp: ModelResponse = serde_json::from_str(&body.response).expect("Model responded with malformed JSON.");
    println!("Question:\n  {}", q);
    println!("Description:\n  {}", resp.description.green());
    if &resp.programming_language != "" {
      println!("Language: {}", resp.programming_language.yellow());
    }
    if &resp.code != "" {
      println!("Code:\n\n{}", resp.code.on_white().black());
    }

    if args.save {
      let ext = if &resp.extension == "" { ".txt" } else { &resp.extension};
      let file_path = format!("./output.{}", ext);
      std::fs::write(&file_path, &resp.code)?;
      println!("Output saved to {file_path}");
    }

    Ok(())
}
