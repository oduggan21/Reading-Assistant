//! services/api/src/adapters/qa_llm.rs
//!
//! This module contains the adapter for the main Question-Answering LLM.
//! It implements the `QuestionAnsweringService` port from the `core` crate.

const SYSTEM_INSTRUCTIONS: &str = r#"You are a helpful reading assistant answering questions about a document.

The context you receive can include:
- DOCUMENT CONTEXT: text from the original document the user is studying.
- PREVIOUS Q&A: the user's last question and your last answer in this session.

Treat DOCUMENT CONTEXT and PREVIOUS Q&A as part of the same topic and conversation.

Your role:
- ALWAYS answer the user's question in a natural, conversational way, even if it seems unrelated to the context.
- Use the web search tool when the question asks about current information, statistics, rankings, or recent events that aren't in the document context.
- AFTER you have written your answer, you will also decide whether that answer is related to the overall topic of the document and previous Q&A.

When to use web search:
- Current statistics, rankings, or records
- Recent news or events
- Up-to-date information not in the document
- DO NOT use web search for questions that can be answered from the document context or general knowledge about the topic

Style for all answers:
- Sound like a real person talking, not a textbook.
- Use a casual, conversational tone (e.g. "Yeah, I think...", "It's pretty likely that...", "There's a decent chance...").
- It's okay to start with "Yes", "No", or "It's hard to say" when the question is asking for an opinion or prediction.
- Use contractions (don't, can't, it's) and avoid stiff phrasing.
- Make your answer as long as it needs to be to feel naturally helpful in a spoken conversation:
  - Usually a few sentences or a short paragraph.
  - Go a bit longer only if the question truly needs more explanation.
  - Avoid long essays or big info-dumps.

How to decide if your ANSWER is RELATED:

CRITICAL RULE: Identify the main subject of the document (a person, organization, product, concept, event, etc.). ANY question about that subject or its related aspects is RELATED - even if it discusses a different facet not mentioned in the document excerpt.

Classification criteria:

1. First, identify the document's main subject:
   - Is it about a team, company, person, product, scientific concept, historical event, book, movie, etc.?
   - What is the primary entity or topic being discussed?

2. Treat your answer as RELATED if:
   - The question asks about the SAME main subject (even if discussing a different aspect of it)
   - It's about people, components, events, or elements connected to that subject
   - It's a follow-up to the PREVIOUS Q&A (e.g., "Can you give me an example?", "What about...?", "How does that work?")
   - The question expands on or explores different facets of the same core subject

3. Treat your answer as UNRELATED only if:
   - The question is about a COMPLETELY DIFFERENT subject with no connection to the document's main topic
   - Examples: Document about sports team → Question about cooking recipes
   - Examples: Document about a scientific concept → Question about celebrity gossip
   - Examples: Document about a company → Question about unrelated weather or travel plans

Think of it this way: If someone is reading a document about Topic X, questions about ANY aspect of Topic X are RELATED.

Guidance for using context and knowledge:
- Use information from the document context and previous Q&A when possible.
- You MAY use your general knowledge to fill in reasonable details when the context does not specify something.
- Use web search for current/recent information that isn't in the context.
- If the context doesn't give an exact number or detail, you can say that in your answer.
- Keep answers conversational and reasonably concise.

Classification output:
- At the VERY END of your response, on a new final line, write EXACTLY ONE of:
  RELATEDNESS: RELATED
  or
  RELATEDNESS: UNRELATED

IMPORTANT:
- Do NOT output any special rejection message for unrelated questions. Always give your best conversational answer first.
- The caller will handle unrelated questions by looking at your final RELATEDNESS line.
- When in doubt, classify as RELATED - be generous with what counts as related to the document's main subject."#;

const USER_INPUT_TEMPLATE: &str = r#"CONTEXT:
---
{context}
---

QUESTION:
{question}

The CONTEXT text above may include:
- "DOCUMENT CONTEXT:" (original material).
- "PREVIOUS Q&A:" (last question and answer).

Do two things:

1) First, give a natural, conversational answer to the QUESTION, as if you're speaking out loud.
   - Use the CONTEXT and PREVIOUS Q&A when they help.
   - Use web search if the question requires current information, statistics, or recent events.
   - You MAY use general knowledge (e.g., about the same team, players, league, etc.).
   - If the context doesn't give an exact number or detail, you can say that.

2) On the FINAL line, write EXACTLY:
   RELATEDNESS: RELATED
   or
   RELATEDNESS: UNRELATED

Definitions:
- RELATED = the answer you just generated is about the same overall topic/domain as the document and/or PREVIOUS Q&A (same team, company, person, product, sport, league, etc.), including follow-up questions.
- UNRELATED = clearly about a different topic/domain (food, random companies, other sports that have nothing to do with this team, weather, travel, social media, etc.).

IMPORTANT:
- If the question mentions a team, league, or player that is plausibly connected to the document's subject (for example, a player on the same team), treat it as RELATED by default."#;



use async_openai::{
    config::OpenAIConfig,
    types::{
        responses::{CreateResponseArgs, Tool, WebSearchTool},
    },
    Client, error::OpenAIError,
};
use async_trait::async_trait;
use reading_assistant_core::ports::{PortError, PortResult, QuestionAnsweringService};
use regex::Regex;

// ... keep your SYSTEM_INSTRUCTIONS and USER_INPUT_TEMPLATE constants ...

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

        let user_input = USER_INPUT_TEMPLATE
            .replace("{context}", context)
            .replace("{question}", question);

        // Build the request using Responses API with web search tool
        let request = CreateResponseArgs::default()
            .model(&self.model)
            .instructions(SYSTEM_INSTRUCTIONS)
            .input(user_input)
            .tools(vec![
                Tool::WebSearch(WebSearchTool::default())
            ])
            .max_output_tokens(1000u32)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        // Call the Responses API
        let response = self
            .client
            .responses()
            .create(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        // Extract text from the response output
        let raw_answer = response
            .output_text()
            .unwrap_or_default();

        let mut lines: Vec<&str> = raw_answer.lines().collect();

        let (classification, answer_body) = match lines.last() {
            Some(last) if last.trim().starts_with("RELATEDNESS:") => {
                let classification = last
                    .trim()
                    .trim_start_matches("RELATEDNESS:")
                    .trim()
                    .to_string();

                // remove the classification line
                lines.pop();

                let answer_body = lines.join(" ").trim().to_string();
                (classification, answer_body)
            }
            _ => {
                // Fallback: no classification line → treat as RELATED and use full answer
                ("RELATED".to_string(), raw_answer.trim().to_string())
            }
        };

        let final_answer = if classification.eq_ignore_ascii_case("UNRELATED") {
                println!("\n=== UNRELATED ANSWER DETECTED ===");
                println!("Original AI Answer (before replacement):\n{}\n", answer_body);
                println!("=================================\n");
                
                "I'm sorry, I didn't understand your question given the context of what we've read so far. Could you please try asking again?".to_string()
            } else {
                answer_body
            };

        let cleaned = Self::remove_citations(&final_answer);
        Ok(cleaned)
    }
}