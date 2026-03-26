use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Web Search
// ---------------------------------------------------------------------------

/// Request body for Brave web search.
#[derive(Debug, Clone, Serialize, Default)]
pub struct WebSearchRequest {
    /// Search query string.
    pub query: String,

    /// Number of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Pagination offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,

    /// Country code filter (e.g. "US", "GB").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Language code filter (e.g. "en", "fr").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Freshness filter (e.g. "pd" for past day, "pw" for past week).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,

    /// Safe search level (e.g. "off", "moderate", "strict").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<String>,
}

/// A single web search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResult {
    /// Page title.
    pub title: String,

    /// Page URL.
    pub url: String,

    /// Result description / snippet.
    #[serde(default)]
    pub description: String,

    /// Age of the result (e.g. "2 hours ago").
    #[serde(default)]
    pub age: Option<String>,

    /// Favicon URL.
    #[serde(default)]
    pub favicon: Option<String>,
}

/// A news search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResult {
    /// Article title.
    pub title: String,

    /// Article URL.
    pub url: String,

    /// Short description.
    #[serde(default)]
    pub description: String,

    /// Age of the article.
    #[serde(default)]
    pub age: Option<String>,

    /// Publisher name.
    #[serde(default)]
    pub source: Option<String>,
}

/// A video search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResult {
    /// Video title.
    pub title: String,

    /// Video page URL.
    pub url: String,

    /// Short description.
    #[serde(default)]
    pub description: String,

    /// Thumbnail URL.
    #[serde(default)]
    pub thumbnail: Option<String>,

    /// Age of the video.
    #[serde(default)]
    pub age: Option<String>,
}

/// An infobox (knowledge panel) result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Infobox {
    /// Infobox title.
    pub title: String,

    /// Long description.
    #[serde(default)]
    pub description: String,

    /// Source URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// Backwards-compatible alias.
pub type InfoboxResult = Infobox;

/// A discussion / forum result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    /// Discussion title.
    pub title: String,

    /// Discussion URL.
    pub url: String,

    /// Short description.
    #[serde(default)]
    pub description: String,

    /// Age of the discussion.
    #[serde(default)]
    pub age: Option<String>,

    /// Forum name.
    #[serde(default)]
    pub forum: Option<String>,
}

/// Backwards-compatible alias.
pub type DiscussionResult = Discussion;

/// Response from the web search endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct WebSearchResponse {
    /// Original query.
    pub query: String,

    /// Web search results.
    #[serde(default)]
    pub web: Vec<WebResult>,

    /// News results.
    #[serde(default)]
    pub news: Vec<NewsResult>,

    /// Video results.
    #[serde(default)]
    pub videos: Vec<VideoResult>,

    /// Infobox / knowledge panel entries.
    #[serde(default)]
    pub infobox: Vec<Infobox>,

    /// Discussion / forum results.
    #[serde(default)]
    pub discussions: Vec<Discussion>,
}

// ---------------------------------------------------------------------------
// Search Context
// ---------------------------------------------------------------------------

/// Request body for search context (returns chunked page content).
#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchContextRequest {
    /// Search query string.
    pub query: String,

    /// Number of results to fetch context from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Country code filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Language code filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Freshness filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,
}

/// A content chunk from search context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextChunk {
    /// Extracted page content.
    pub content: String,

    /// Source URL.
    pub url: String,

    /// Page title.
    #[serde(default)]
    pub title: String,

    /// Relevance score.
    #[serde(default)]
    pub score: f64,

    /// Content type (e.g. "text/html").
    #[serde(default)]
    pub content_type: Option<String>,
}

/// A source reference from search context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextSource {
    /// Source URL.
    pub url: String,

    /// Source title.
    #[serde(default)]
    pub title: String,
}

/// Response from the search context endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchContextResponse {
    /// Content chunks extracted from search results.
    pub chunks: Vec<SearchContextChunk>,

    /// Source references.
    #[serde(default)]
    pub sources: Vec<SearchContextSource>,

    /// Original query.
    pub query: String,
}

// ---------------------------------------------------------------------------
// Search Answer (AI-generated answer with citations)
// ---------------------------------------------------------------------------

/// A chat message for the search answer endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnswerMessage {
    /// Message role ("user", "assistant", "system").
    pub role: String,

    /// Message text content.
    pub content: String,
}

/// Request body for search answer (AI-generated answer grounded in search).
#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchAnswerRequest {
    /// Conversation messages.
    pub messages: Vec<SearchAnswerMessage>,

    /// Model to use for answer generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// A citation reference in a search answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnswerCitation {
    /// Source URL.
    pub url: String,

    /// Source title.
    #[serde(default)]
    pub title: String,

    /// Snippet from the source.
    #[serde(default)]
    pub snippet: Option<String>,
}

/// A choice in the search answer response.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchAnswerChoice {
    /// Choice index.
    pub index: i32,

    /// The generated message.
    pub message: SearchAnswerMessage,

    /// Finish reason (e.g. "stop").
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Response from the search answer endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchAnswerResponse {
    /// Generated answer choices.
    pub choices: Vec<SearchAnswerChoice>,

    /// Model that produced the answer.
    #[serde(default)]
    pub model: String,

    /// Unique response identifier.
    #[serde(default)]
    pub id: String,

    /// Citations used in the answer.
    #[serde(default)]
    pub citations: Vec<SearchAnswerCitation>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Performs a Brave web search, returning structured results across web, news,
    /// videos, discussions, and infoboxes.
    pub async fn web_search(&self, req: &WebSearchRequest) -> Result<WebSearchResponse> {
        let (resp, _meta) = self
            .post_json::<WebSearchRequest, WebSearchResponse>("/qai/v1/search/web", req)
            .await?;
        Ok(resp)
    }

    /// Searches the web and returns chunked page content suitable for RAG or
    /// context injection into LLM prompts.
    pub async fn search_context(
        &self,
        req: &SearchContextRequest,
    ) -> Result<SearchContextResponse> {
        let (resp, _meta) = self
            .post_json::<SearchContextRequest, SearchContextResponse>(
                "/qai/v1/search/context",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Generates an AI-powered answer grounded in live web search results,
    /// with citations.
    pub async fn search_answer(&self, req: &SearchAnswerRequest) -> Result<SearchAnswerResponse> {
        let (resp, _meta) = self
            .post_json::<SearchAnswerRequest, SearchAnswerResponse>(
                "/qai/v1/search/answer",
                req,
            )
            .await?;
        Ok(resp)
    }
}
