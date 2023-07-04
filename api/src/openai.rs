use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    },
    Client as OpenAiClient,
};

pub struct Client {
    inner_client: OpenAiClient<OpenAIConfig>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner_client: OpenAiClient::new(),
        }
    }

    pub async fn chat(
        &self,
        messages: &[ChatCompletionRequestMessage],
    ) -> anyhow::Result<CreateChatCompletionResponse> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo")
            .messages(messages)
            .build()?;

        Ok(self.inner_client.chat().create(request).await?)
    }
}
