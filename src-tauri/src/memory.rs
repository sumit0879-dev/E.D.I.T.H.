use futures::StreamExt;
use tauri::Manager;
use arrow_array::{
    FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator,
    StringArray, Array,
};
use arrow_schema::{DataType, Field};
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryChunk {
    pub id: String,
    pub text: String,
    pub source: String,
    pub score: Option<f32>,
}

async fn get_db(app: &tauri::AppHandle) -> Result<lancedb::connection::Connection, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let lance_path = base.join("lancedb");
    std::fs::create_dir_all(&lance_path).map_err(|e| e.to_string())?;
    let path_str = lance_path
        .to_str()
        .ok_or_else(|| "Invalid LanceDB path".to_string())?;
    connect(path_str).execute().await.map_err(|e| e.to_string())
}

fn get_schema() -> Arc<arrow_schema::Schema> {
    Arc::new(arrow_schema::Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384),
            false,
        ),
    ]))
}


fn make_fixed_size_list(
    item_field: Arc<Field>,
    size: i32,
    values: Arc<dyn Array>,
) -> Result<FixedSizeListArray, String> {
    FixedSizeListArray::try_new(item_field, size, values, None)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn save_to_memory_cmd(
    app: tauri::AppHandle,
    text: String,
    source: String,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    let db = get_db(&app).await?;
    let schema = get_schema();
    let table_name = "memory_chunks";

    let raw_chunks: Vec<String> = if text.len() < 500 {
        vec![text.trim().to_string()]
    } else {
        text.split("\n\n")
            .filter(|s| !s.trim().is_empty())
            .flat_map(|para| {
                let p = para.trim();
                if p.len() <= 500 {
                    vec![p.to_string()]
                } else {
                    p.chars()
                        .collect::<Vec<char>>()
                        .chunks(500)
                        .map(|c| c.iter().collect::<String>())
                        .collect()
                }
            })
            .collect()
    };

    if raw_chunks.is_empty() {
        return Ok(());
    }

    let ts = chrono::Utc::now().timestamp();
    let ids: Vec<String> = raw_chunks
        .iter()
        .enumerate()
        .map(|(i, _)| format!("{}-{}-{}", source, i, ts))
        .collect();
    let texts: Vec<String> = raw_chunks.clone();
    let sources: Vec<String> = raw_chunks.iter().map(|_| source.clone()).collect();
    let mut flat_vectors: Vec<f32> = Vec::new();
    for chunk in &raw_chunks {
        flat_vectors.extend(crate::embedding::embed_text_hash(chunk));
    }

    let id_arr = Arc::new(StringArray::from(ids)) as Arc<dyn Array>;
    let text_arr = Arc::new(StringArray::from(texts)) as Arc<dyn Array>;
    let src_arr = Arc::new(StringArray::from(sources)) as Arc<dyn Array>;

    let float_vals = Arc::new(Float32Array::from(flat_vectors)) as Arc<dyn Array>;
    let item_field = Arc::new(Field::new("item", DataType::Float32, true));
    let vec_arr = Arc::new(make_fixed_size_list(item_field, 384, float_vals)?) as Arc<dyn Array>;

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![id_arr, text_arr, src_arr, vec_arr],
    )
    .map_err(|e| e.to_string())?;

    let reader = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());
    let table_names = db.table_names().execute().await.map_err(|e| e.to_string())?;

    if table_names.contains(&table_name.to_string()) {
        let table = db
            .open_table(table_name)
            .execute()
            .await
            .map_err(|e| e.to_string())?;
        table
            .add(Box::new(reader))
            .execute()
            .await
            .map_err(|e| e.to_string())?;
    } else {
        db.create_table(table_name, Box::new(reader))
            .execute()
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
pub async fn search_memory_cmd(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<MemoryChunk>, String> {
    let db = get_db(&app).await?;
    let table_name = "memory_chunks";

    let table_names = db.table_names().execute().await.map_err(|e| e.to_string())?;
    if !table_names.contains(&table_name.to_string()) {
        return Ok(vec![]);
    }

    let table = db
        .open_table(table_name)
        .execute()
        .await
        .map_err(|e| e.to_string())?;
    let qvec = crate::embedding::embed_text_hash(&query);

    let mut stream = table
        .vector_search(qvec)
        .map_err(|e| e.to_string())?
        .limit(5)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let mut chunks = Vec::new();
    while let Some(batch_res) = stream.next().await {
        let batch = batch_res.map_err(|e| e.to_string())?;
        let id_col = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let txt_col = batch
            .column_by_name("text")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let src_col = batch
            .column_by_name("source")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let dist_col = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        for i in 0..batch.num_rows() {
            chunks.push(MemoryChunk {
                id: id_col.value(i).to_string(),
                text: txt_col.value(i).to_string(),
                source: src_col.value(i).to_string(),
                score: dist_col.map(|d| d.value(i)),
            });
        }
    }

    Ok(chunks)
}

#[command]
pub async fn get_memories_cmd(app: tauri::AppHandle) -> Result<Vec<MemoryChunk>, String> {
    let db = get_db(&app).await?;
    let table_name = "memory_chunks";

    let table_names = db.table_names().execute().await.map_err(|e| e.to_string())?;
    if !table_names.contains(&table_name.to_string()) {
        return Ok(vec![]);
    }

    let table = db
        .open_table(table_name)
        .execute()
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = table
        .query()
        .select(lancedb::query::Select::Columns(vec![
            "id".to_string(),
            "text".to_string(),
            "source".to_string(),
        ]))
        .limit(200)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let mut seen = std::collections::HashSet::new();
    let mut chunks = Vec::new();

    while let Some(batch_res) = stream.next().await {
        let batch = batch_res.map_err(|e| e.to_string())?;
        let id_col = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let txt_col = batch
            .column_by_name("text")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let src_col = batch
            .column_by_name("source")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        for i in 0..batch.num_rows() {
            let src = src_col.value(i).to_string();
            if !seen.contains(&src) {
                seen.insert(src.clone());
                let preview: String = txt_col.value(i).chars().take(100).collect();
                chunks.push(MemoryChunk {
                    id: id_col.value(i).to_string(),
                    text: preview + "...",
                    source: src,
                    score: None,
                });
            }
        }
    }

    Ok(chunks)
}

#[tauri::command]
pub async fn delete_memory_cmd(app: tauri::AppHandle, source: String) -> Result<(), String> {
    let db = get_db(&app).await?;
    let table = match db.open_table("memory_chunks").execute().await {
        Ok(t) => t,
        Err(_) => return Ok(()), // table doesn't exist
    };
    let predicate = crate::security::LanceDbSanitizer::sanitize_source_predicate(&source)?;
    table.delete(predicate.as_str()).await.map_err(|e| e.to_string())?;
    Ok(())
}
