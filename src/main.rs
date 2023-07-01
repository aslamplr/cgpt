// use serde::{Deserialize, Serialize};
// use surrealdb::engine::local::File;
// use surrealdb::Surreal;

use std::io::Write;

use anyhow::Context;
use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !std::env::var("OPENAI_API_KEY").is_ok() {
        dotenv::dotenv()
            .with_context(|| "Set OPENAI_API_KEY environment variable or add in .env file")?;
    }

    let openai_client = Client::new();
    // let db = Surreal::new::<File>("/tmp/cgpt.rocks.db").await?;

    let mut messages = vec![ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content(
            "You are a helpful software engineer expert in Rust language and AWS Cloud Platform.",
        )
        .build()?];

    println!("This is a chat gpt CLI; type `exit` or Control-C to exit the promt! Requires internet conntectivity!");

    loop {
        // take input from user
        let mut input = String::new();
        print!(" ⌨ : ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input).unwrap();

        if input.trim() == "exit" {
            break;
        }

        let new_msg = ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(input.trim())
            .build()?;

        messages.push(new_msg);

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo")
            .messages(messages.clone())
            .build()?;

        let response = openai_client.chat().create(request).await?;

        print!("🤖 : ");
        std::io::stdout().flush()?;
        let response = response.choices.first();
        if let Some(response) = response {
            println!(
                "{}",
                response
                    .message
                    .content
                    .as_deref()
                    .unwrap_or_else(|| "No content in response!")
            );
        } else {
            println!("No response!")
        }
    }

    Ok(())
}
