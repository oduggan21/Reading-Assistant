//! services/api/src/adapters/qa_llm.rs
//!
//! This module contains the adapter for the main Question-Answering LLM.
//! It implements the `QuestionAnsweringService` port from the `core` crate.

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client, error::OpenAIError,
};
use async_trait::async_trait;
use reading_assistant_core::ports::{PortError, PortResult, QuestionAnsweringService};
use regex::Regex;
use futures::{Stream, StreamExt};
use std::pin::Pin;
//=========================================================================================
// The Main Adapter Struct
//=========================================================================================

/// An adapter that implements `QuestionAnsweringService` using an OpenAI-compatible LLM.
#[derive(Clone)]
pub struct OpenAiQaAdapter {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAiQaAdapter {
    /// Creates a new `OpenAiQaAdapter`.
    pub fn new(client: Client<OpenAIConfig>, model: String) -> Self {
        Self { client, model }
    }
    fn remove_citations(text: &str) -> String {
        // Remove markdown citations like ([url.com](link))
        let citation_regex = Regex::new(r"\(\[.*?\]\(.*?\)\)").unwrap();
        let without_citations = citation_regex.replace_all(text, "");
        
        // Remove any lines that start with ## or - (sections/bullet points)
        let lines: Vec<&str> = without_citations
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("##") && 
                !trimmed.starts_with("- [") &&
                !trimmed.is_empty()
            })
            .collect();
        
        // Take only the first 1-2 sentences (before the citations section)
        let result = lines.join(" ").trim().to_string();
        
        // Find the first occurrence of multiple sentences and cut off
        if let Some(pos) = result.find(". ") {
            // Look for second sentence
            if let Some(second_pos) = result[pos+2..].find(". ") {
                return result[..pos + second_pos + 3].to_string();
            }
        }
        
        result
    }

}

//=========================================================================================
// `QuestionAnsweringService` Trait Implementation
//=========================================================================================

#[async_trait]
impl QuestionAnsweringService for OpenAiQaAdapter {
    /// Answers a user's question based on a provided snippet of text (context).
    async fn answer_question(&self, question: &str, context: &str) -> PortResult<String> {

        let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content("You are a strict validation assistant. Your ONLY job is to check if the question relates to the provided context. The context is about a specific topic. If the question asks about ANYTHING not mentioned in the context, you MUST respond with EXACTLY: 'I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?' Do NOT answer unrelated questions. Do NOT use your general knowledge. ONLY answer if the question is directly about something in the context.")
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "CONTEXT:\n---\n{}\n---\n\nQUESTION: {}\n\nIs this question about something in the context? If NO, respond with the exact rejection message. If YES, answer briefly in 1-2 sentences using ONLY information from the context.",
                context, question
            ))
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?
            .into(),
    ];

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        if let Some(choice) = response.choices.into_iter().next() {
            if let Some(content) = choice.message.content {
                // ✅ Clean up the response by removing citations and extra content
                let cleaned = Self::remove_citations(&content);
                Ok(cleaned)
            } else {
                Err(PortError::Unexpected(
                    "LLM response contained no text content.".to_string(),
                ))
            }
        } else {
            Err(PortError::Unexpected(
                "LLM returned no choices in its response.".to_string(),
            ))
        }
    }

     async fn answer_question_streaming(
        &self,
        question: &str,
        context: &str,
    ) -> PortResult<Pin<Box<dyn Stream<Item = Result<String, PortError>> + Send>>> {
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are an expert tutor. Answer the user's question based on the provided context and any recent information. Be concise and clear. Keep your response limited to 1-2 sentences. Do NOT include any URLs, citations, or references in your answer - only provide the information in natural conversational language.")
                .build()
                .map_err(|e| PortError::Unexpected(e.to_string()))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!(
                    "CONTEXT:\n---\n{}\n---\n\nQUESTION: {}",
                    context, question
                ))
                .build()
                .map_err(|e| PortError::Unexpected(e.to_string()))?
                .into(),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .stream(true)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let stream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        // Convert the stream to our result type
        let mapped_stream = stream.map(|result| {
            result
                .map_err(|e| PortError::Unexpected(e.to_string()))
                .and_then(|response| {
                    let content: String = response
                        .choices
                        .into_iter()
                        .filter_map(|choice| choice.delta.content)
                        .collect();
                    Ok(content)
                })
                .and_then(|content| {
                    if content.is_empty() {
                        Ok(String::new())
                    } else {
                        Ok(content)
                    }
                })
        });

        Ok(Box::pin(mapped_stream))
    }
}

