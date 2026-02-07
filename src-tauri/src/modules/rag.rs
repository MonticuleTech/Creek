// RAG Module (Retrieval-Augmented Generation)
//
// Retrieves historical conversation fragments based on current query
// Only triggered when Router 2 determines "context is missing"

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use arrow_schema::{Schema, Field, DataType};
use arrow_array::{
    StringArray, Int64Array, FixedSizeListArray, Float32Array, 
    RecordBatch, RecordBatchIterator, Array as ArrowArray
};
use std::sync::Arc as StdArc;
use futures_util::TryStreamExt;
use sha2::{Sha256, Digest};
use log::{info, warn};

const SIMILARITY_THRESHOLD: f32 = 0.7;

/// Generates a valid LanceDB table name from a recording ID.
/// 
/// Strategy:
/// 1. Replace "-" with "_" (legacy behavior).
/// 2. Check if the resulting ID contains only allowed characters (alphanumeric, _, .).
/// 3. If allowed, use it (preserves backward compatibility).
/// 4. If not allowed (e.g. contains Chinese), use the SHA256 hex hash of the original ID.
fn get_table_name(recording_id: &str, table_type: &str) -> String {
    let legacy_id = recording_id.replace("-", "_");
    
    // LanceDB safe chars: alphanumeric, _, -, .
    // We strictly check for this.
    let is_safe = legacy_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    
    if is_safe {
        format!("{}_{}", table_type, legacy_id)
    } else {
        // Fallback to hash
        let mut hasher = Sha256::new();
        hasher.update(recording_id.as_bytes());
        let result = hasher.finalize();
        let hash_hex = result.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        format!("{}_{}", table_type, hash_hex)
    }
}

/// Search Result - unified result from RAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub content: String,
    pub source: String,       // "conversation" or filename
    pub timestamp: i64,       // Unix timestamp (0 for static docs)
    pub score: f32,           // Similarity score (0.0 to 1.0)
}

/// Conversation Turn - stores ONLY user ASR input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: String,           // UUID
    pub timestamp: i64,       // Unix timestamp in milliseconds
    pub asr_input: String,    // User ASR chunk raw text
}

impl ConversationTurn {
    pub fn new(asr_input: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().timestamp_millis(),
            asr_input,
        }
    }
}

/// Query Agent - generates retrieval queries from user input
pub struct QueryAgent;

impl QueryAgent {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate retrieval query from user input using LLM
    pub async fn generate_query<T: crate::services::llm_client::LLMClient>(
        &self,
        llm: &T,
        user_input: &str,
        current_doc: &str,
    ) -> Result<String> {
        use crate::services::llm_client::ChatMessage;
        use crate::prompts::rag::build_query_generation_prompt;
        
        let prompt = build_query_generation_prompt(user_input, current_doc);
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }
        ];
        
        // Call LLM with timeout
        let query = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.collect_llm_response(llm, messages)
        ).await {
            Ok(Ok(resp)) => {
                let trimmed = resp.trim().to_string();
                if trimmed.is_empty() || trimmed.len() > 100 {
                    user_input.to_string() // Fallback to original input
                } else {
                    trimmed
                }
            }
            _ => user_input.to_string(), // Fallback
        };
        
        Ok(query)
    }
    
    /// Helper to collect full LLM response from stream
    async fn collect_llm_response<T: crate::services::llm_client::LLMClient>(
        &self,
        llm: &T,
        messages: Vec<crate::services::llm_client::ChatMessage>,
    ) -> Result<String> {
        use futures_util::StreamExt;
        
        let mut stream = llm.stream_completion(messages).await
            .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;
        
        let mut response = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => response.push_str(&chunk),
                Err(e) => return Err(anyhow::anyhow!("Stream error: {:?}", e)),
            }
        }
        
        Ok(response)
    }
}

impl Default for QueryAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// RAG Service - manages vector storage and retrieval with fastembed and LanceDB
pub struct RagService {
    model: Arc<Mutex<TextEmbedding>>,
    db: Arc<Connection>,
    embedding_dim: usize,
}

impl RagService {
    /// Create new RAG service with fastembed model and LanceDB
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        info!("[RAG] Initializing fastembed model...");
        
        // Initialize fastembed with multilingual model (supports Chinese)
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallZHV15)
                .with_show_download_progress(true)
        )?;
        
        info!("[RAG] Fastembed model initialized successfully");
        
        // Ensure db path exists
        std::fs::create_dir_all(&db_path)?;
        
        // Initialize LanceDB connection
        info!("[RAG] Connecting to LanceDB...");
        let db_uri = db_path.to_string_lossy().to_string();
        let db = lancedb::connect(&db_uri).execute().await?;
        
        info!("[RAG] LanceDB connected successfully");
        
        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            db: Arc::new(db),
            embedding_dim: 512, // BGE-Small-ZH-v1.5 dimension
        })
    }
    
    /// Store a document (split into chunks) in LanceDB
    pub async fn ingest_document(&self, recording_id: &str, filename: &str, content: &str) -> Result<()> {
        let chunks = self.chunk_text(content, 500); // 500 char chunks
        
        info!("[RAG] Ingesting document '{}' ({} chunks)...", filename, chunks.len());
        
        // Batch embedding generation
        let embeddings = self.model.lock().unwrap().embed(chunks.clone(), None)?;
        
        if embeddings.is_empty() {
            return Ok(());
        }
        
        // Prepare arrays
        let mut ids = Vec::new();
        let mut timestamps = Vec::new();
        let mut sources = Vec::new();
        let mut contents = Vec::new();
        let mut flat_embeddings = Vec::new();
        
        let now = Utc::now().timestamp_millis();
        
        for (i, embedding) in embeddings.iter().enumerate() {
            ids.push(uuid::Uuid::new_v4().to_string());
            timestamps.push(now);
            sources.push(filename.to_string());
            contents.push(chunks[i].clone());
            flat_embeddings.extend_from_slice(embedding);
        }
        
        // Create RecordBatch
        let id_array = StringArray::from(ids);
        let timestamp_array = Int64Array::from(timestamps);
        let source_array = StringArray::from(sources);
        let content_array = StringArray::from(contents);
        
        let values = Float32Array::from(flat_embeddings);
        let field = StdArc::new(Field::new("item", DataType::Float32, true));
        let embedding_array = FixedSizeListArray::try_new(field, self.embedding_dim as i32, StdArc::new(values), None)?;
        
        let schema = self.get_resource_schema();
        let batch = RecordBatch::try_new(
            StdArc::new(schema),
            vec![
                StdArc::new(id_array),
                StdArc::new(timestamp_array),
                StdArc::new(source_array),
                StdArc::new(content_array),
                StdArc::new(embedding_array),
            ],
        )?;
        
        // Store in resources table
        let table_name = get_table_name(recording_id, "resources");
        let db = &self.db;
        
        match db.open_table(&table_name).execute().await {
            Ok(tbl) => {
                let batches = RecordBatchIterator::new(vec![Ok(batch)], StdArc::new(self.get_resource_schema()));
                tbl.add(Box::new(batches)).execute().await?;
            }
            Err(_) => {
                let batches = RecordBatchIterator::new(vec![Ok(batch.clone())], batch.schema());
                db.create_table(&table_name, Box::new(batches)).execute().await?;
            }
        };
        
        info!("[RAG] Document '{}' ingested successfully", filename);
        Ok(())
    }

    /// Retrieve relevant resources (documents)
    pub async fn retrieve_resources(
        &self,
        recording_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_embeddings = self.model.lock().unwrap().embed(vec![query.to_string()], None)?;
        if query_embeddings.is_empty() {
            return Ok(vec![]);
        }
        let query_embedding = &query_embeddings[0];
        
        let table_name = get_table_name(recording_id, "resources");
        let db = &self.db;
        
        let table = match db.open_table(&table_name).execute().await {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        
        let mut stream = table
            .query()
            .nearest_to(query_embedding.clone())?
            .limit(top_k)
            .execute()
            .await?;
            
        let mut results = Vec::new();
        
        while let Some(batch) = stream.try_next().await? {
            let source_col = batch.column(2).as_any().downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow::anyhow!("Invalid source column"))?;
            let content_col = batch.column(3).as_any().downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow::anyhow!("Invalid content column"))?;
            
            // Try to find _distance column
            let distance_col = batch.column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
            
            for i in 0..batch.num_rows() {
                let distance = if let Some(col) = distance_col {
                    col.value(i)
                } else {
                    0.0
                };
                
                // Convert L2 distance to cosine similarity
                let similarity = 1.0 - (distance * distance) / 2.0;
                
                if similarity >= SIMILARITY_THRESHOLD {
                    results.push(SearchResult {
                        content: content_col.value(i).to_string(),
                        source: source_col.value(i).to_string(),
                        timestamp: 0, 
                        score: similarity,
                    });
                }
            }
        }
        
        // Sort results by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        Ok(results)
    }

    /// Store a conversation turn with embedding in LanceDB
    pub async fn store_turn(&self, recording_id: &str, turn: &ConversationTurn) -> Result<()> {
        // Generate embedding
        let embeddings = self.model.lock().unwrap().embed(vec![turn.asr_input.clone()], None)?;
        
        if embeddings.is_empty() {
            return Err(anyhow::anyhow!("Failed to generate embedding"));
        }
        
        let embedding = &embeddings[0];
        
        // Prepare data for LanceDB
        let id_array = StringArray::from(vec![turn.id.clone()]);
        let timestamp_array = Int64Array::from(vec![turn.timestamp]);
        let asr_input_array = StringArray::from(vec![turn.asr_input.clone()]);
        
        // Convert embedding Vec<f32> to FixedSizeListArray
        let values = Float32Array::from(embedding.clone());
        let field = StdArc::new(Field::new("item", DataType::Float32, true));
        let embedding_array = FixedSizeListArray::try_new(field, self.embedding_dim as i32, StdArc::new(values), None)?;
        
        // Create RecordBatch
        let schema = self.get_schema();
        let batch = RecordBatch::try_new(
            StdArc::new(schema),
            vec![
                StdArc::new(id_array),
                StdArc::new(timestamp_array),
                StdArc::new(asr_input_array),
                StdArc::new(embedding_array),
            ],
        )?;
        
        // Get or create table
        let table_name = get_table_name(recording_id, "recording");
        let db = &self.db;
        
        // Check if table exists, if not create it
        match db.open_table(&table_name).execute().await {
            Ok(tbl) => {
                // Table exists, append data
                let batches = RecordBatchIterator::new(vec![Ok(batch)], StdArc::new(self.get_schema()));
                tbl.add(Box::new(batches)).execute().await?;
            }
            Err(_) => {
                // Table doesn't exist, create it with first batch
                let batches = RecordBatchIterator::new(vec![Ok(batch.clone())], batch.schema());
                db.create_table(&table_name, Box::new(batches)).execute().await?;
            }
        };
        
        // Safe string truncation for display
        let display_text = turn.asr_input.chars().take(20).collect::<String>();
        let suffix = if turn.asr_input.chars().count() > 20 { "..." } else { "" };
        info!("[RAG] Stored turn in LanceDB {} (input: \"{}{}\")", turn.id, display_text, suffix);
        Ok(())
    }
    
    /// Get Arrow schema for conversation turns
    fn get_schema(&self) -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("timestamp", DataType::Int64, false),
            Field::new("asr_input", DataType::Utf8, false),
            Field::new("embedding", DataType::FixedSizeList(
                StdArc::new(Field::new("item", DataType::Float32, true)),
                self.embedding_dim as i32,
            ), false),
        ])
    }
    
    /// Get Arrow schema for resources
    fn get_resource_schema(&self) -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("timestamp", DataType::Int64, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("embedding", DataType::FixedSizeList(
                StdArc::new(Field::new("item", DataType::Float32, true)),
                self.embedding_dim as i32,
            ), false),
        ])
    }

    /// Simple text chunking
    fn chunk_text(&self, text: &str, chunk_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let end = std::cmp::min(i + chunk_size, chars.len());
            chunks.push(chars[i..end].iter().collect());
            i += chunk_size - 50; // 50 chars overlap
        }
        chunks
    }

    /// Retrieve relevant conversation turns using LanceDB vector search
    pub async fn retrieve(
        &self,
        recording_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<(ConversationTurn, f32)>> {
        // Generate query embedding
        let query_embeddings = self.model.lock().unwrap().embed(vec![query.to_string()], None)?;
        
        if query_embeddings.is_empty() {
            return Err(anyhow::anyhow!("Failed to generate query embedding"));
        }
        
        let query_embedding = &query_embeddings[0];
        
        // Open table for this recording
        let table_name = get_table_name(recording_id, "recording");
        let db = &self.db;
        
        let table = match db.open_table(&table_name).execute().await {
            Ok(t) => t,
            Err(_) => {
                // Table doesn't exist yet
                return Ok(vec![]);
            }
        };
        
        // Vector search using LanceDB
        let mut stream = table
            .query()
            .nearest_to(query_embedding.clone())?
            .limit(top_k * 2) // Fetch more candidates to filter by score
            .execute()
            .await?;
        
        // Convert results to ConversationTurn
        let mut turns = Vec::new();
        
        while let Some(batch) = stream.try_next().await? {
            let id_col = batch.column(0).as_any().downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow::anyhow!("Invalid id column"))?;
            let timestamp_col = batch.column(1).as_any().downcast_ref::<Int64Array>()
                .ok_or_else(|| anyhow::anyhow!("Invalid timestamp column"))?;
            let asr_input_col = batch.column(2).as_any().downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow::anyhow!("Invalid asr_input column"))?;
            
            // Try to find _distance column
            let distance_col = batch.column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
            
            for i in 0..batch.num_rows() {
                let distance = if let Some(col) = distance_col {
                    col.value(i)
                } else {
                    0.0
                };
                
                // Convert L2 distance to cosine similarity
                let similarity = 1.0 - (distance * distance) / 2.0;
                
                if similarity >= SIMILARITY_THRESHOLD {
                    let turn = ConversationTurn {
                        id: id_col.value(i).to_string(),
                        timestamp: timestamp_col.value(i),
                        asr_input: asr_input_col.value(i).to_string(),
                    };
                    
                    // Safe string truncation for display
                    let display_text = turn.asr_input.chars().take(20).collect::<String>();
                    let suffix = if turn.asr_input.chars().count() > 20 { "..." } else { "" };
                    info!("[RAG] Retrieved: \"{}{}\" from LanceDB (score: {:.4})", display_text, suffix, similarity);
                    
                    turns.push((turn, similarity));
                }
            }
        }
        
        turns.truncate(top_k);
        Ok(turns)
    }
    
    /// Retrieve relevant context (turns + documents)
    pub async fn retrieve_unified(
        &self,
        recording_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        // 1. Get turns
        match self.retrieve(recording_id, query, top_k).await {
            Ok(turns) => {
                for (turn, score) in turns {
                    results.push(SearchResult {
                        content: turn.asr_input,
                        source: "conversation".to_string(),
                        timestamp: turn.timestamp,
                        score,
                    });
                }
            }
            Err(_) => {} // Ignore errors (e.g. table not found)
        }
        
        // 2. Get resources
        match self.retrieve_resources(recording_id, query, top_k).await {
            Ok(docs) => {
                results.extend(docs);
            }
            Err(_) => {}
        }
        
        // Sort combined results by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        
        Ok(results)
    }

    /// Initialize LanceDB table for a recording
    pub async fn init_recording(&self, recording_id: &str) -> Result<()> {
        let table_name = get_table_name(recording_id, "recording");
        let db = &self.db;
        
        // Check if table already exists
        match db.open_table(&table_name).execute().await {
            Ok(_) => {
                info!("[RAG] Recording table already exists: {}", recording_id);
                Ok(())
            }
            Err(_) => {
                // Table will be created on first insert
                info!("[RAG] Recording initialized (table will be created on first insert): {}", recording_id);
                Ok(())
            }
        }
    }

    /// Delete all RAG data for a recording
    pub async fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let recording_table = get_table_name(recording_id, "recording");
        let resources_table = get_table_name(recording_id, "resources");
        let db = &self.db;

        // Try to drop recording table
        if let Err(e) = db.drop_table(&recording_table, &[] as &[String]).await {
            // Log but don't fail, as table might not exist
            warn!("[RAG] Note: returning error when dropping table {} (might not exist): {:?}", recording_table, e);
        } else {
            info!("[RAG] Dropped table: {}", recording_table);
        }

        // Try to drop resources table
        if let Err(e) = db.drop_table(&resources_table, &[] as &[String]).await {
             warn!("[RAG] Note: returning error when dropping table {} (might not exist): {:?}", resources_table, e);
        } else {
            info!("[RAG] Dropped table: {}", resources_table);
        }

        Ok(())
    }
}
