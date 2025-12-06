use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use reading_assistant_core::ports::{PortError, PortResult, TitleGenerationService};

pub struct OpenAiTitleAdapter {
    client: Client<OpenAIConfig>,
}

impl OpenAiTitleAdapter {
    pub fn new(client: Client<OpenAIConfig>) -> Self {  // ✅ Accept client instead of API key
        Self { client }
    }
}

#[async_trait]
impl TitleGenerationService for OpenAiTitleAdapter {
    async fn generate_title_from_text(&self, text: &str) -> PortResult<String> {
        let preview = text.chars().take(1000).collect::<String>();

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a title generation assistant. Generate a short, descriptive title (maximum 6 words) for the given document. The title should capture the main topic or subject. Respond with ONLY the title, no quotes, no explanation.")
                    .build()
                    .map_err(|e| PortError::Unexpected(e.to_string()))?
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(format!("Generate a title for this document:\n\n{}", preview))
                    .build()
                    .map_err(|e| PortError::Unexpected(e.to_string()))?
            ),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(messages)
            .max_tokens(20u32)
            .temperature(0.7)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let title = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| PortError::Unexpected("No title generated".to_string()))?;

        Ok(title.trim().to_string())
    }
}