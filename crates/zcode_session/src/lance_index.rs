//! LanceDB-backed vector retrieval for session turns.
//!
//! The database is a rebuildable local index derived from the JSONL session log.
//! JSONL remains the source of truth; LanceDB is used for vector candidate search.

use crate::intent::{
    HashedIntentVectorizer, IntentMatch, IntentProfile, IntentVector, VECTOR_DIMS,
};
use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Int32Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::database::CreateTableMode;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, DistanceType};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zcode_core::{Result, ZcodeError};

const TABLE_NAME: &str = "turns";
const ROW_ID_COLUMN: &str = "row_id";
const VECTOR_COLUMN: &str = "vector";

#[derive(Debug, Clone)]
pub struct LanceIntentDocument {
    pub item: usize,
    pub vector: IntentVector,
    pub profile: IntentProfile,
}

impl LanceIntentDocument {
    pub fn from_text(item: usize, text: &str) -> Self {
        let vectorizer = HashedIntentVectorizer;
        Self {
            item,
            vector: vectorizer.embed(text),
            profile: IntentProfile::analyze(text),
        }
    }
}

pub struct LanceIntentIndex {
    index_dir: PathBuf,
    documents: Vec<LanceIntentDocument>,
}

impl LanceIntentIndex {
    pub fn new(index_dir: impl AsRef<Path>, documents: Vec<LanceIntentDocument>) -> Self {
        Self {
            index_dir: index_dir.as_ref().to_path_buf(),
            documents,
        }
    }

    pub fn search(
        &self,
        prompt: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<IntentMatch<usize>>> {
        if prompt.trim().is_empty() || limit == 0 || self.documents.is_empty() {
            return Ok(Vec::new());
        }

        block_on_lancedb(search_async(
            self.index_dir.clone(),
            self.documents.clone(),
            prompt.to_string(),
            threshold,
            limit,
        ))
    }
}

async fn search_async(
    index_dir: PathBuf,
    documents: Vec<LanceIntentDocument>,
    prompt: String,
    threshold: f32,
    limit: usize,
) -> Result<Vec<IntentMatch<usize>>> {
    std::fs::create_dir_all(&index_dir)?;
    let db = connect(index_dir.to_string_lossy().as_ref())
        .execute()
        .await
        .map_err(lancedb_error)?;
    let table = db
        .create_table(TABLE_NAME, record_batch_for_documents(&documents)?)
        .mode(CreateTableMode::Overwrite)
        .execute()
        .await
        .map_err(lancedb_error)?;

    let vectorizer = HashedIntentVectorizer;
    let query_vector = vectorizer.embed(&prompt);
    let query_profile = IntentProfile::analyze(&prompt);
    let candidate_limit = documents.len().min(limit.saturating_mul(8).max(limit));
    let batches = table
        .query()
        .limit(candidate_limit)
        .nearest_to(query_vector.values().to_vec())
        .map_err(lancedb_error)?
        .distance_type(DistanceType::Cosine)
        .select(lancedb::query::Select::columns(&[ROW_ID_COLUMN]))
        .execute()
        .await
        .map_err(lancedb_error)?
        .try_collect::<Vec<_>>()
        .await
        .map_err(lancedb_error)?;

    let mut matches = Vec::new();
    for row_id in row_ids_from_batches(&batches)? {
        let Some(document) = documents.get(row_id) else {
            continue;
        };
        let vector_score = query_vector.cosine_similarity(&document.vector);
        let relation = query_profile.relation_to(&document.profile);
        if !relation.is_related(vector_score, threshold) {
            continue;
        }
        let profile_score = relation.score;
        let score = (vector_score * 0.45) + (profile_score * 0.55);
        matches.push(IntentMatch {
            item: document.item,
            score,
            vector_score,
            profile_score,
        });
    }

    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    matches.truncate(limit);
    Ok(matches)
}

fn record_batch_for_documents(documents: &[LanceIntentDocument]) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new(ROW_ID_COLUMN, DataType::Int32, false),
        Field::new(
            VECTOR_COLUMN,
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                VECTOR_DIMS as i32,
            ),
            true,
        ),
    ]));

    let row_ids = Int32Array::from_iter_values((0..documents.len()).map(|index| index as i32));
    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        documents.iter().map(|document| {
            Some(
                document
                    .vector
                    .values()
                    .iter()
                    .copied()
                    .map(Some)
                    .collect::<Vec<_>>(),
            )
        }),
        VECTOR_DIMS as i32,
    );

    RecordBatch::try_new(schema, vec![Arc::new(row_ids), Arc::new(vectors)])
        .map_err(|error| ZcodeError::IndexError(format!("LanceDB Arrow batch error: {}", error)))
}

fn row_ids_from_batches(batches: &[RecordBatch]) -> Result<Vec<usize>> {
    let mut row_ids = Vec::new();
    for batch in batches {
        let column = batch.column_by_name(ROW_ID_COLUMN).ok_or_else(|| {
            ZcodeError::IndexError(format!("LanceDB result missing `{}` column", ROW_ID_COLUMN))
        })?;
        let ids = column
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or_else(|| {
                ZcodeError::IndexError(format!(
                    "LanceDB `{}` column had unexpected type",
                    ROW_ID_COLUMN
                ))
            })?;
        row_ids.extend(ids.iter().flatten().map(|id| id as usize));
    }
    Ok(row_ids)
}

fn block_on_lancedb<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .map_err(ZcodeError::IoError)?
            .block_on(future)
    })
    .join()
    .map_err(|_| ZcodeError::IndexError("LanceDB worker thread panicked".to_string()))?
}

fn lancedb_error(error: lancedb::Error) -> ZcodeError {
    ZcodeError::IndexError(format!("LanceDB error: {}", error))
}
