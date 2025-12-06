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

const SYSTEM_PROMPT: &str = r#"You are a helpful reading assistant answering questions about a document.

The context you receive can include:
- DOCUMENT CONTEXT: text from the original document the user is studying.
- PREVIOUS Q&A: the user's last question and your last answer in this session.

Treat DOCUMENT CONTEXT and PREVIOUS Q&A as part of the same topic and conversation.

Your role:
- Answer questions that are related to the same overall topic as the context and previous Q&A.
- For unrelated questions, respond with EXACTLY: "I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?"

Style for RELATED questions:
- Sound like a real person talking, not a textbook.
- Use a casual, conversational tone (e.g. "Yeah, I think...", "It’s pretty likely that...", "There’s a decent chance...").
- It's okay to start with "Yes", "No", or "It's hard to say" when the question is asking for an opinion or prediction.
- Use contractions (don't, can't, it's) and avoid stiff phrasing.
- Make your answer as long as it needs to be to feel naturally helpful in a spoken conversation:
  - Usually a few sentences or a short paragraph.
  - Go a bit longer only if the question truly needs more explanation.
  - Avoid long essays or big info-dumps.

How to decide if a question is RELATED:
1. The context can contain:
   - A DOCUMENT CONTEXT section.
   - A PREVIOUS Q&A section with:
     Q: (last user question)
     A: (your last answer)
2. A new question is RELATED if it is mainly about:
   - The same subject or domain as the document (team, company, person, product, scientific topic, historical event, book, movie, concept, etc.), OR
   - A follow-up to the PREVIOUS Q&A (for example "Can you give me an example of that?", "How does that work in practice?", "What about in gambling?", "What if I change X?").
3. When the new question uses pronouns or vague references like "that", "it", "this", "the example you mentioned":
   - Assume it refers to the ideas in PREVIOUS Q&A, unless the DOCUMENT CONTEXT clearly points to something else.

For RELATED questions:
- Use information from the document context and previous Q&A when possible.
- You MAY use your general knowledge to fill in reasonable details.
- If the context doesn’t include enough detail to fully answer, say that in a natural way (e.g. "The context doesn’t say exactly, but in general...").
- Keep answers conversational and reasonably concise.

For UNRELATED questions (clearly different subject or domain):
- Use the exact rejection message:
  "I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?"
- Do not answer the unrelated question.

Remember:
If the question asks about people, events, or concepts that live in the same general domain as the document or the PREVIOUS Q&A, treat it as RELATED — even if those specific names or details are not literally in the text."#;

const USER_PROMPT_TEMPLATE: &str = r#"CONTEXT:
---
{context}
---

QUESTION:
{question}

The CONTEXT text above may include:
- A section starting with "DOCUMENT CONTEXT:" (the original material the user is reading).
- A section starting with "PREVIOUS Q&A:" showing the last question and answer in this conversation.

First, on the first line, output ONLY one word: "RELATED" or "UNRELATED".

- "RELATED" = the question is about the same overall topic/domain as the document and/or the PREVIOUS Q&A
  (for example: the same team, company, person, product, scientific topic, historical event, book, movie, or concept),
  including follow-up questions that refer back to "that", "it", "this", or "what you just said".
- "UNRELATED" = the question is clearly about a different topic/domain (e.g. food, random companies, other sports, weather, travel, social media, etc.) that does not logically follow from the document or PREVIOUS Q&A.

After that:
- If RELATED: on the second line, give a natural, conversational answer that feels like how a human would explain it out loud.
  It should be long enough to actually answer the question (often a few sentences or a short paragraph), but not a long essay.
- If UNRELATED: on the second line, output EXACTLY:
I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?"#;

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

        println!("QUESTION:\n{}\n", question);
        println!("CONTEXT:\n{}\n", context);

        let user_prompt = USER_PROMPT_TEMPLATE
            .replace("{context}", context)
            .replace("{question}", question);

        let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(SYSTEM_PROMPT)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(user_prompt)
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

         let raw_answer = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .cloned()
            .unwrap_or_default();

        let mut lines = raw_answer.lines();
        let classification = lines.next().unwrap_or("").trim();
        let answer_body = lines.collect::<Vec<_>>().join(" ").trim().to_string();

        let final_answer = if classification.eq_ignore_ascii_case("UNRELATED") {
            "I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?".to_string()
        } else {
            answer_body
        };

        let cleaned = Self::remove_citations(&final_answer);
        Ok(cleaned)
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

