use serde::{Deserialize, Serialize};
use surrealdb::engine::local::File;
use surrealdb::Surreal;

use std::io::Write;

use anyhow::Context;
use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};

#[derive(Debug, Serialize, Deserialize)]
struct Chat {
    messages: Vec<ChatCompletionRequestMessage>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = if let Some(config) = config::get_config().await {
        std::env::set_var("OPENAI_API_KEY", config.openai_api_key.as_ref());
        config.surreal_db_path
    } else {
        if std::env::var("OPENAI_API_KEY").is_err() {
            dotenv::dotenv()
                .with_context(|| "Set OPENAI_API_KEY environment variable or add in .env file")?;
        }
        config::save_config(config::Config::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            "/tmp/cgpt.cli.rocks.db",
        ))
        .await?;
        "/tmp/cgpt.cli.rocks.db".to_string().into()
    };
    let db_path = db_path.as_ref();

    let openai_client = Client::new();
    let db = Surreal::new::<File>(db_path).await?;

    db.use_ns("cgpt").use_db("default").await?;

    let chat: Option<Chat> = db.select(("chat", "default_chat")).await?;

    let mut messages = if let Some(chat) = chat {
        chat.messages
    } else {
        vec![ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(
                "You are a helpful software engineer expert in Rust language and AWS Cloud Platform.",
            )
            .build()?]
    };

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
                    .unwrap_or("No content in response!")
            );
        } else {
            println!("No response!")
        }
    }

    Ok(())
}

mod config {
    use std::borrow::Cow;

    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tokio::fs::{self, File};
    use tokio::io::AsyncWriteExt;
    use tokio::task;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Config {
        pub(crate) openai_api_key: Cow<'static, str>,
        pub surreal_db_path: Cow<'static, str>,
    }

    impl Config {
        pub fn new<T1, T2>(openai_api_key: T1, surreal_db_path: T2) -> Self
        where
            T1: Into<Cow<'static, str>>,
            T2: Into<Cow<'static, str>>,
        {
            Self {
                openai_api_key: openai_api_key.into(),
                surreal_db_path: surreal_db_path.into(),
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|p| p.join(".config/cgpt/config.toml"))
    }

    pub async fn get_config() -> Option<Config> {
        let path = get_config_path()?;
        if fs::metadata(&path).await.is_ok() {
            let content = fs::read(path).await.ok()?;
            let content = String::from_utf8(content).ok()?;
            task::spawn_blocking(move || toml::from_str::<Config>(&content).ok())
                .await
                .ok()
                .flatten()
        } else {
            None
        }
    }

    pub async fn save_config(config: Config) -> Result<PathBuf> {
        let err_fn = || anyhow::anyhow!("Couldn't establish a config path!");
        let path = get_config_path().ok_or_else(err_fn)?;
        if fs::metadata(&path).await.is_err() {
            let parent = path.parent().ok_or_else(err_fn)?;
            fs::create_dir_all(parent).await?;
        }
        let toml = task::spawn_blocking(move || toml::to_string(&config)).await??;
        let mut file = File::create(&path).await?;
        file.write_all(toml.as_bytes()).await?;
        Ok(path)
    }
}
