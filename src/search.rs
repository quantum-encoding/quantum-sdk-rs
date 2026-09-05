use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Search Options
// ---------------------------------------------------------------------------

/// Options for configuring web search requests.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchOptions {
    /// Number of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Zero-based result offset for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,

    /// Country code filter (e.g. "US", "GB").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Language code filter (e.g. "en", "fr").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Time range filter (e.g. "24h", "7d", "30d").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,

    /// Adult content filtering ("off", "moderate", "strict").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_search: Option<String>,
}

/// Options for configuring LLM context search requests.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ContextOptions {
    /// Number of context chunks to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Country code filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Language code filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Time range filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,
}

/// A message in a search-answer conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMessage {
    /// Message role ("user" or "assistant").
    pub role: String,

    /// Message text content.
    pub content: String,
}

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

/// LLM-optimised context response from web search.
///
/// Unlike [`SearchContextResponse`], this returns simple string sources
/// and is the type returned by the Go SDK's `SearchContext` method.
#[derive(Debug, Clone, Deserialize)]
pub struct LLMContextResponse {
    /// Original search query.
    pub query: String,

    /// Content chunks suitable for LLM consumption.
    #[serde(default)]
    pub chunks: Vec<ContextChunk>,

    /// Source URLs used.
    #[serde(default)]
    pub sources: Vec<String>,
}

/// A single chunk of context from a web page (simple variant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
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
// Google Grounded Search — Gemini Flash + google_search tool
// ---------------------------------------------------------------------------

/// Request body for Google grounded search via Gemini.
///
/// The premium search backend: Google's index rather than Brave's, billed
/// per executed query at $0.035 each. The model decides how many queries one
/// prompt becomes; `web_search_queries` on the response lists them.
#[derive(Debug, Clone, Serialize, Default)]
pub struct GoogleSearchRequest {
    /// Search query string. Free-form natural language; the model will
    /// translate this into one or more concrete Google searches.
    pub query: String,
}

/// A web source returned by Google grounding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSearchCitation {
    /// Source URL (may be a Google redirect link the user can follow).
    pub url: String,

    /// Source title from the search result.
    #[serde(default)]
    pub title: String,
}

/// Links a span of the answer text to one or more citation indices,
/// enabling inline-citation rendering on the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSearchSupport {
    /// Byte offset where this span starts in the answer text.
    pub start_index: i32,

    /// Byte offset where this span ends (exclusive).
    pub end_index: i32,

    /// The text of the span, so a renderer can match by content when the
    /// answer has been transformed and the byte offsets no longer apply.
    #[serde(default)]
    pub text: String,

    /// Indices into `citations` for the sources backing this span.
    #[serde(default)]
    pub grounding_chunk_indices: Vec<i32>,
}

/// Response from the Google grounded search endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleSearchResponse {
    /// The grounded answer text Gemini produced. May be empty if the
    /// model decided no answer was warranted.
    #[serde(default)]
    pub answer: String,

    /// Web sources Gemini grounded its answer on.
    #[serde(default)]
    pub citations: Vec<GoogleSearchCitation>,

    /// HTML/CSS widget of search-suggestion chips. Google's grounding terms
    /// require it to be rendered, unmodified, alongside any grounded response.
    #[serde(default)]
    pub search_entry_point: String,

    /// The queries Gemini executed against Google Search; each one is a
    /// billing unit.
    #[serde(default)]
    pub web_search_queries: Vec<String>,

    /// Inline-citation spans linking text segments to citations.
    #[serde(default)]
    pub supports: Vec<GoogleSearchSupport>,
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
            .post_json::<SearchContextRequest, SearchContextResponse>("/qai/v1/search/context", req)
            .await?;
        Ok(resp)
    }

    /// Generates an AI-powered answer grounded in live web search results,
    /// with citations.
    pub async fn search_answer(&self, req: &SearchAnswerRequest) -> Result<SearchAnswerResponse> {
        let (resp, _meta) = self
            .post_json::<SearchAnswerRequest, SearchAnswerResponse>("/qai/v1/search/answer", req)
            .await?;
        Ok(resp)
    }

    /// Performs a Google grounded search via Gemini Flash + the
    /// google_search built-in tool. Returns a grounded answer plus
    /// citations, the ToS-required search-entry-point widget, and the
    /// list of queries Gemini actually executed.
    ///
    /// Billed per executed query ($0.035 each). `search_answer` is the
    /// Brave-backed alternative for cheap high-volume search.
    pub async fn google_search(&self, req: &GoogleSearchRequest) -> Result<GoogleSearchResponse> {
        let (resp, _meta) = self
            .post_json::<GoogleSearchRequest, GoogleSearchResponse>("/qai/v1/search/google", req)
            .await?;
        Ok(resp)
    }
}
