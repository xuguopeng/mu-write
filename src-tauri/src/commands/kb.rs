use crate::commands::arg;
use crate::state::AppState;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const CHUNK_SIZE: usize = 1200;
const SUPPORTED_EXTENSIONS: &[&str] = &["txt", "md", "text"];

pub async fn dispatch_kb(
    state: Arc<AppState>,
    channel: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    match channel {
        "kb:import-document" => import_document(state, args).await,
        "kb:import-folder" => import_folder(state, args).await,
        "kb:import-text" => import_text(state, args).await,
        "kb:search" => search(state, args, false).await,
        "kb:search-with-scope" => search(state, args, true).await,
        "kb:list-documents" => list_documents(state).await,
        "kb:remove-document" => remove_document(state, args).await,
        "kb:stats" => stats(state).await,
        "kb:get-vectorless-count" => vectorless_count(state).await,
        "kb:backfill-vectors" => backfill_vectors(state).await,
        _ => Err(format!("Unsupported IPC channel: {channel}")),
    }
}

async fn import_document(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let file_path: String = arg(&args, 0, "filePath")?;
    let path = PathBuf::from(&file_path);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "导入文档.txt".to_string());
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("读取文档失败 '{}': {error}", path.display()))?;
    import_text_inner(state, text, file_name, None, Some(file_path)).await
}

async fn import_folder(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let folder_path: String = arg(&args, 0, "folderPath")?;
    let files = collect_supported_files(Path::new(&folder_path))?;
    let mut imported_count = 0usize;
    let mut failed_files = Vec::new();

    for path in files {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "导入文档.txt".to_string());
        match fs::read_to_string(&path) {
            Ok(text) => match import_text_inner(
                state.clone(),
                text,
                file_name,
                None,
                Some(path.to_string_lossy().to_string()),
            )
            .await
            {
                Ok(result)
                    if result
                        .get("success")
                        .and_then(Value::as_bool)
                        .unwrap_or(false) =>
                {
                    imported_count += 1;
                }
                _ => failed_files.push(path.to_string_lossy().to_string()),
            },
            Err(_) => failed_files.push(path.to_string_lossy().to_string()),
        }
    }

    Ok(json!({
        "success": true,
        "importedCount": imported_count,
        "failedFiles": failed_files,
    }))
}

async fn import_text(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let text: String = arg(&args, 0, "text")?;
    let file_name: String = arg(&args, 1, "fileName")?;
    let project_path: String = arg(&args, 2, "projectPath")?;
    import_text_inner(state, text, file_name, Some(project_path), None).await
}

async fn import_text_inner(
    state: Arc<AppState>,
    text: String,
    file_name: String,
    project_path: Option<String>,
    source_path: Option<String>,
) -> Result<Value, String> {
    let kb_dir = kb_dir_for_project(&state, project_path).await;
    let texts_dir = kb_dir.join("texts");
    fs::create_dir_all(&texts_dir).map_err(|error| error.to_string())?;

    let doc_id = Uuid::new_v4().to_string();
    let stored_path = texts_dir.join(format!("{doc_id}.txt"));
    fs::write(&stored_path, &text).map_err(|error| error.to_string())?;

    let chunks = chunk_text(&text);
    let imported_at = Utc::now().to_rfc3339();
    let chapter_number = infer_chapter_number(&file_name).unwrap_or_default();
    let conn = open_kb(&kb_dir)?;

    conn.execute(
        "INSERT INTO kb_documents (id,file_name,file_path,source_path,imported_at,chunk_count,chapter_number) VALUES (?,?,?,?,?,?,?)",
        params![doc_id, file_name, stored_path.to_string_lossy().to_string(), source_path.unwrap_or_default(), imported_at, chunks.len() as i64, chapter_number],
    )
    .map_err(|error| error.to_string())?;

    for (index, chunk) in chunks.iter().enumerate() {
        conn.execute(
            "INSERT INTO kb_chunks (doc_id,chunk_index,text,chapter_number,has_vector) VALUES (?,?,?,?,0)",
            params![doc_id, index as i64, chunk, chapter_number],
        )
        .map_err(|error| error.to_string())?;
        let chunk_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO kb_chunks_fts (rowid,text,file_name) VALUES (?,?,?)",
            params![chunk_id, chunk, file_name],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(json!({ "success": true, "docId": doc_id, "chunkCount": chunks.len() }))
}

async fn search(state: Arc<AppState>, args: Vec<Value>, scoped: bool) -> Result<Value, String> {
    let query: String = arg(&args, 0, "query")?;
    let top_k_index = if scoped { 3 } else { 1 };
    let top_k = args.get(top_k_index).and_then(Value::as_u64).unwrap_or(5) as usize;
    if query.trim().is_empty() || top_k == 0 {
        return Ok(json!([]));
    }

    let scope = if scoped {
        let from_chapter = args.get(1).and_then(Value::as_i64).unwrap_or(0);
        let to_chapter = args.get(2).and_then(Value::as_i64).unwrap_or(i64::MAX);
        Some((from_chapter, to_chapter))
    } else {
        None
    };

    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let mut hits = search_fts(&conn, &query, top_k, scope).unwrap_or_default();
    if hits.is_empty() {
        hits = search_like(&conn, &query, top_k, scope)?;
    }
    Ok(Value::Array(hits))
}

async fn list_documents(state: Arc<AppState>) -> Result<Value, String> {
    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let mut stmt = conn
        .prepare("SELECT id,file_name,file_path,imported_at,chunk_count FROM kb_documents ORDER BY imported_at DESC")
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "fileName": row.get::<_, String>(1)?,
                "filePath": row.get::<_, String>(2)?,
                "importedAt": row.get::<_, String>(3)?,
                "chunkCount": row.get::<_, i64>(4)?,
            }))
        })
        .map_err(|error| error.to_string())?;

    let mut docs = Vec::new();
    for row in rows {
        docs.push(row.map_err(|error| error.to_string())?);
    }
    Ok(Value::Array(docs))
}

async fn remove_document(state: Arc<AppState>, args: Vec<Value>) -> Result<Value, String> {
    let doc_id: String = arg(&args, 0, "docId")?;
    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let file_path = conn
        .query_row(
            "SELECT file_path FROM kb_documents WHERE id=?",
            params![doc_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id FROM kb_chunks WHERE doc_id=?")
        .map_err(|error| error.to_string())?;
    let chunk_ids = stmt
        .query_map(params![doc_id], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    for chunk_id in chunk_ids {
        conn.execute("DELETE FROM kb_chunks_fts WHERE rowid=?", params![chunk_id])
            .map_err(|error| error.to_string())?;
    }
    conn.execute("DELETE FROM kb_chunks WHERE doc_id=?", params![doc_id])
        .map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM kb_documents WHERE id=?", params![doc_id])
        .map_err(|error| error.to_string())?;

    if let Some(path) = file_path {
        let _ = fs::remove_file(path);
    }
    Ok(json!({ "success": true }))
}

async fn stats(state: Arc<AppState>) -> Result<Value, String> {
    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let document_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM kb_documents", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let total_chunks: i64 = conn
        .query_row("SELECT COUNT(*) FROM kb_chunks", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;

    Ok(json!({
        "documentCount": document_count,
        "totalChunks": total_chunks,
        "vectorDimension": 0,
    }))
}

async fn vectorless_count(state: Arc<AppState>) -> Result<Value, String> {
    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM kb_chunks WHERE has_vector=0",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({ "count": count }))
}

async fn backfill_vectors(state: Arc<AppState>) -> Result<Value, String> {
    let kb_dir = kb_dir_for_project(&state, None).await;
    let conn = open_kb(&kb_dir)?;
    let processed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM kb_chunks WHERE has_vector=0",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    conn.execute("UPDATE kb_chunks SET has_vector=1 WHERE has_vector=0", [])
        .map_err(|error| error.to_string())?;
    Ok(json!({ "success": true, "processed": processed, "failed": 0 }))
}

async fn kb_dir_for_project(state: &Arc<AppState>, project_path: Option<String>) -> PathBuf {
    if let Some(path) = project_path.filter(|path| !path.trim().is_empty()) {
        return PathBuf::from(path).join(".vela").join("kb");
    }

    if let Some(path) = state.current_project_path.read().await.clone() {
        return path.join(".vela").join("kb");
    }

    state.vela_home.join("kb")
}

fn open_kb(kb_dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(kb_dir).map_err(|error| error.to_string())?;
    let conn = Connection::open(kb_dir.join("kb.sqlite")).map_err(|error| error.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kb_documents (
            id TEXT PRIMARY KEY,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            source_path TEXT DEFAULT '',
            imported_at TEXT NOT NULL,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            chapter_number INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS kb_chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            doc_id TEXT NOT NULL REFERENCES kb_documents(id) ON DELETE CASCADE,
            chunk_index INTEGER NOT NULL,
            text TEXT NOT NULL,
            chapter_number INTEGER NOT NULL DEFAULT 0,
            has_vector INTEGER NOT NULL DEFAULT 0
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS kb_chunks_fts USING fts5(text, file_name);
        CREATE INDEX IF NOT EXISTS idx_kb_chunks_doc ON kb_chunks(doc_id);
        CREATE INDEX IF NOT EXISTS idx_kb_chunks_chapter ON kb_chunks(chapter_number);",
    )
    .map_err(|error| error.to_string())?;
    Ok(conn)
}

fn search_fts(
    conn: &Connection,
    query: &str,
    top_k: usize,
    scope: Option<(i64, i64)>,
) -> Result<Vec<Value>, String> {
    let fts_query = build_fts_query(query);
    if fts_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut sql = String::from(
        "SELECT c.text,d.file_name,bm25(kb_chunks_fts) AS rank
         FROM kb_chunks_fts
         JOIN kb_chunks c ON c.id=kb_chunks_fts.rowid
         JOIN kb_documents d ON d.id=c.doc_id
         WHERE kb_chunks_fts MATCH ?",
    );
    if scope.is_some() {
        sql.push_str(" AND c.chapter_number>=? AND c.chapter_number<=?");
    }
    sql.push_str(" ORDER BY rank ASC LIMIT ?");

    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let mut rows = if let Some((from_chapter, to_chapter)) = scope {
        stmt.query(params![fts_query, from_chapter, to_chapter, top_k as i64])
    } else {
        stmt.query(params![fts_query, top_k as i64])
    }
    .map_err(|error| error.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let rank: f64 = row.get(2).unwrap_or(0.0);
        out.push(json!({
            "text": row.get::<_, String>(0).map_err(|error| error.to_string())?,
            "fileName": row.get::<_, String>(1).map_err(|error| error.to_string())?,
            "score": rank_to_score(rank),
        }));
    }
    Ok(out)
}

fn search_like(
    conn: &Connection,
    query: &str,
    top_k: usize,
    scope: Option<(i64, i64)>,
) -> Result<Vec<Value>, String> {
    let mut sql = String::from(
        "SELECT c.text,d.file_name,c.chapter_number
         FROM kb_chunks c
         JOIN kb_documents d ON d.id=c.doc_id",
    );
    if scope.is_some() {
        sql.push_str(" WHERE c.chapter_number>=? AND c.chapter_number<=?");
    }

    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let mut rows = if let Some((from_chapter, to_chapter)) = scope {
        stmt.query(params![from_chapter, to_chapter])
    } else {
        stmt.query([])
    }
    .map_err(|error| error.to_string())?;

    let query_lc = query.to_lowercase();
    let terms = query_lc
        .split_whitespace()
        .filter(|term| !term.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut hits = Vec::new();

    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let text: String = row.get(0).map_err(|error| error.to_string())?;
        let text_lc = text.to_lowercase();
        let term_hits = terms
            .iter()
            .filter(|term| text_lc.contains(term.as_str()))
            .count();
        let direct_hit = if text_lc.contains(&query_lc) { 2 } else { 0 };
        let score_raw = term_hits + direct_hit;
        if score_raw == 0 {
            continue;
        }
        let score = (score_raw as f64 / (terms.len().max(1) as f64 + 2.0)).min(1.0);
        hits.push(json!({
            "text": text,
            "fileName": row.get::<_, String>(1).map_err(|error| error.to_string())?,
            "score": score,
        }));
    }

    hits.sort_by(|a, b| {
        let ascore = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let bscore = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        bscore
            .partial_cmp(&ascore)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(top_k);
    Ok(hits)
}

fn collect_supported_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Err(format!("目录不存在: {}", root.display()));
    }
    let mut files = Vec::new();
    collect_supported_files_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_supported_files_inner(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if is_supported_file(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let child = entry.path();
        if child
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }
        collect_supported_files_inner(&child, out)?;
    }
    Ok(())
}

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| SUPPORTED_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn chunk_text(text: &str) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    chars
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect::<String>().trim().to_string())
        .filter(|chunk| !chunk.is_empty())
        .collect()
}

fn infer_chapter_number(file_name: &str) -> Option<i64> {
    let lower = file_name.to_lowercase();
    if let Some(rest) = lower.strip_prefix("chapter_") {
        let digits = rest
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        return digits.parse::<i64>().ok();
    }

    let start = file_name.find('第')? + '第'.len_utf8();
    let rest = &file_name[start..];
    let end = rest.find('章')?;
    rest[..end].trim().parse::<i64>().ok()
}

fn build_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|term| term.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && !is_cjk(ch)))
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn rank_to_score(rank: f64) -> f64 {
    (1.0 / (1.0 + rank.abs())).clamp(0.0, 1.0)
}
