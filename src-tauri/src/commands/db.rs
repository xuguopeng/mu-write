use crate::state::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

pub async fn dispatch_db(
    state: Arc<AppState>,
    channel: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    let db_path = current_db_path(state).await;
    with_db(db_path, |conn| {
        match channel {
        "db:init" => Ok(json!(init_database())),
        "db:close" => Ok(json!({ "success": true })),
        "db:project-core-get" => project_core_get(conn),
        "db:project-core-update" => project_core_update(conn, arg_value(&args, 0)),
        "db:blueprint-get-all" => blueprint_all(conn),
        "db:blueprint-get" => blueprint_get(conn, arg_i64(&args, 0)?),
        "db:blueprint-upsert" => blueprint_upsert(conn, arg_value(&args, 0)),
        "db:blueprint-upsert-many" => {
            for item in arg_value(&args, 0).as_array().cloned().unwrap_or_default() { blueprint_upsert(conn, item)?; }
            Ok(json!({ "success": true }))
        }
        "db:blueprint-update-notes" => exec_ok(conn, "UPDATE blueprints SET notes=?, notes_updated_at=datetime('now'), updated_at=datetime('now') WHERE chapter_number=?", params![arg_string(&args, 1)?, arg_i64(&args, 0)?]),
        "db:character-get-all" => character_all(conn),
        "db:character-upsert" => character_upsert(conn, arg_value(&args, 0)),
        "db:character-save-all" => {
            for item in arg_value(&args, 0).as_array().cloned().unwrap_or_default() { character_upsert(conn, item)?; }
            Ok(json!({ "success": true }))
        }
        "db:character-delete" => exec_ok(conn, "DELETE FROM characters WHERE name=?", params![arg_string(&args, 0)?]),
        "db:character-update-state" => character_update_state(conn, arg_string(&args, 0)?, arg_value(&args, 1)),
        "db:draft-create" => draft_create(conn, arg_value(&args, 0)),
        "db:draft-list" => draft_list(conn, arg_i64(&args, 0)?),
        "db:draft-get-meta" => draft_meta(conn, arg_i64(&args, 0)?),
        "db:draft-get-full" => draft_full(conn, arg_i64(&args, 0)?),
        "db:draft-get-latest" => draft_one(conn, "SELECT * FROM drafts WHERE chapter_number=? ORDER BY version DESC LIMIT 1", arg_i64(&args, 0)?),
        "db:draft-get-finalized" => draft_one(conn, "SELECT * FROM drafts WHERE chapter_number=? AND status='finalized' ORDER BY version DESC LIMIT 1", arg_i64(&args, 0)?),
        "db:draft-get-max-finalized-chapter" => scalar_i64(conn, "SELECT COALESCE(MAX(chapter_number),0) FROM drafts WHERE status='finalized'", []),
        "db:draft-next-version" => scalar_i64_plus_one(conn, "SELECT COALESCE(MAX(version),0) FROM drafts WHERE chapter_number=?", [arg_i64(&args, 0)?]),
        "db:draft-update-status" => update_draft_status(conn, arg_i64(&args, 0)?, arg_string(&args, 1)?, args.get(2).and_then(Value::as_i64)),
        "db:draft-update-content" => update_draft_content(conn, arg_i64(&args, 0)?, arg_string(&args, 1)?, arg_i64(&args, 2)?),
        "db:revision-create" => branch_create(conn, "revisions", arg_value(&args, 0)),
        "db:revision-list" => branch_list(conn, "revisions", "base_draft_id", arg_i64(&args, 0)?, false),
        "db:revision-get-pending" => branch_list(conn, "revisions", "base_draft_id", arg_i64(&args, 0)?, true),
        "db:revision-get-full" => branch_full(conn, "revisions", arg_i64(&args, 0)?),
        "db:revision-next-index" => scalar_i64_plus_one(conn, "SELECT COALESCE(MAX(revision_index),0) FROM revisions WHERE base_draft_id=?", [arg_i64(&args, 0)?]),
        "db:revision-mark-merged" => exec_ok(conn, "UPDATE revisions SET status='merged', merged_to_draft_id=?, updated_at=datetime('now') WHERE id=?", params![arg_i64(&args, 1)?, arg_i64(&args, 0)?]),
        "db:revision-mark-discarded" => exec_ok(conn, "UPDATE revisions SET status='discarded', updated_at=datetime('now') WHERE id=?", params![arg_i64(&args, 0)?]),
        "db:review-create" => review_create(conn, arg_value(&args, 0)),
        "db:review-list" => review_list(conn, arg_i64(&args, 0)?),
        "db:review-get-latest" => review_latest(conn, arg_i64(&args, 0)?),
        "db:review-get-full" => review_full(conn, arg_i64(&args, 0)?),
        "db:review-next-index" => scalar_i64_plus_one(conn, "SELECT COALESCE(MAX(review_index),0) FROM reviews WHERE base_draft_id=?", [arg_i64(&args, 0)?]),
        "db:post-process-create-run" => post_run_create(conn, arg_value(&args, 0)),
        "db:post-process-get-latest-run" => post_run_latest(conn, arg_string(&args, 0)?, arg_string(&args, 1)?),
        "db:post-process-get-steps" => post_steps(conn, arg_string(&args, 0)?),
        "db:post-process-mark-step-ok" => post_step_mark(conn, arg_string(&args, 0)?, arg_string(&args, 1)?, None),
        "db:post-process-mark-step-failed" => post_step_mark(conn, arg_string(&args, 0)?, arg_string(&args, 1)?, Some(arg_string(&args, 2)?)),
        "db:post-process-is-all-passed" => post_all_passed(conn, arg_string(&args, 0)?, arg_string(&args, 1)?),
        "db:log-llm-call" => log_llm(conn, arg_value(&args, 0)),
        "db:get-llm-stats" => llm_stats(conn),
        "db:get-llm-history" => llm_history(conn, args.first().and_then(Value::as_i64).unwrap_or(30)),
        "db:save-summary-snapshot" => exec_ok(conn, "INSERT INTO summary_snapshots (chapter_number, character_states) VALUES (?,?)", params![arg_i64(&args, 0)?, arg_string(&args, 1)?]),
        "db:get-latest-summary" => latest_summary(conn),
        _ => Err(format!("Unsupported IPC channel: {channel}")),
    }
    })
}

async fn current_db_path(state: Arc<AppState>) -> PathBuf {
    if let Some(path) = state.current_db_path.read().await.clone() {
        return path;
    }
    if let Some(path) = state.current_project_path.read().await.clone() {
        return path.join(".vela").join("vela.db");
    }
    state.vela_home.join("vela.db")
}

fn with_db<F>(path: PathBuf, f: F) -> Result<Value, String>
where
    F: FnOnce(&Connection) -> Result<Value, String>,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn =
        Connection::open(&path).map_err(|e| format!("Failed to open '{}': {e}", path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    for sql in init_database() {
        conn.execute_batch(&sql).map_err(|e| e.to_string())?;
    }
    f(&conn)
}

pub fn init_database() -> Vec<String> {
    vec![
    "CREATE TABLE IF NOT EXISTS project_core (id TEXT PRIMARY KEY DEFAULT 'main', project_name TEXT NOT NULL DEFAULT '', genre TEXT DEFAULT '', sub_genre TEXT DEFAULT '', target_audience TEXT DEFAULT '', total_chapters INTEGER DEFAULT 100, words_per_chapter INTEGER DEFAULT 3000, plot_structure TEXT DEFAULT 'three_act', narrative_pov TEXT DEFAULT 'third_limited', writing_style TEXT DEFAULT '', reference_works TEXT DEFAULT '', global_guidance TEXT DEFAULT '', golden_finger TEXT DEFAULT '', premise TEXT DEFAULT '', worldbuilding TEXT DEFAULT '', characters_arch TEXT DEFAULT '', synopsis TEXT DEFAULT '', character_states TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS blueprints (chapter_number INTEGER PRIMARY KEY, title TEXT NOT NULL DEFAULT '', role TEXT DEFAULT '', purpose TEXT DEFAULT '', key_events TEXT DEFAULT '', characters TEXT DEFAULT '[]', suspense_hook TEXT DEFAULT '', user_guidance TEXT DEFAULT '', notes TEXT DEFAULT '', notes_updated_at TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS characters (name TEXT PRIMARY KEY, role TEXT DEFAULT 'supporting', gender TEXT DEFAULT '', age TEXT DEFAULT '', appearance TEXT DEFAULT '', personality TEXT DEFAULT '', background TEXT DEFAULT '', abilities TEXT DEFAULT '', motivation TEXT DEFAULT '', relationships TEXT DEFAULT '', arc TEXT DEFAULT '', notes TEXT DEFAULT '', cs_location TEXT DEFAULT '', cs_power_level TEXT DEFAULT '', cs_physical_state TEXT DEFAULT '', cs_mental_state TEXT DEFAULT '', cs_key_items TEXT DEFAULT '', cs_recent_events TEXT DEFAULT '', cs_updated_at_chapter INTEGER DEFAULT 0, created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS contents (id INTEGER PRIMARY KEY AUTOINCREMENT, body TEXT NOT NULL DEFAULT '', created_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS drafts (id INTEGER PRIMARY KEY AUTOINCREMENT, chapter_number INTEGER NOT NULL, version INTEGER NOT NULL DEFAULT 1, status TEXT DEFAULT 'draft', source TEXT DEFAULT 'write', content_id INTEGER NOT NULL REFERENCES contents(id) ON DELETE RESTRICT, word_count INTEGER DEFAULT 0, created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now'))); CREATE INDEX IF NOT EXISTS idx_drafts_chapter ON drafts(chapter_number)".into(),
    "CREATE TABLE IF NOT EXISTS revisions (id INTEGER PRIMARY KEY AUTOINCREMENT, base_draft_id INTEGER NOT NULL REFERENCES drafts(id) ON DELETE CASCADE, revision_index INTEGER NOT NULL, revision_type TEXT NOT NULL, status TEXT DEFAULT 'pending', merged_to_draft_id INTEGER, user_prompt TEXT DEFAULT '', review_source_id INTEGER, content_id INTEGER NOT NULL REFERENCES contents(id) ON DELETE RESTRICT, word_count INTEGER DEFAULT 0, created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS reviews (id INTEGER PRIMARY KEY AUTOINCREMENT, base_draft_id INTEGER NOT NULL REFERENCES drafts(id) ON DELETE CASCADE, review_index INTEGER NOT NULL, content_id INTEGER NOT NULL REFERENCES contents(id) ON DELETE RESTRICT, created_at TEXT DEFAULT (datetime('now')))".into(),
    "CREATE TABLE IF NOT EXISTS post_process_runs (id TEXT PRIMARY KEY, trigger_source_type TEXT NOT NULL, trigger_source_id TEXT NOT NULL, source_label TEXT DEFAULT '', all_critical_passed INTEGER DEFAULT 0, created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now'))); CREATE INDEX IF NOT EXISTS idx_post_runs_source ON post_process_runs(trigger_source_type, trigger_source_id)".into(),
    "CREATE TABLE IF NOT EXISTS post_process_steps (id INTEGER PRIMARY KEY AUTOINCREMENT, run_id TEXT NOT NULL REFERENCES post_process_runs(id) ON DELETE CASCADE, step_key TEXT NOT NULL, label TEXT DEFAULT '', critical INTEGER DEFAULT 0, ok INTEGER DEFAULT 0, error_msg TEXT DEFAULT '', attempt_count INTEGER DEFAULT 0, completed_at TEXT DEFAULT '', last_attempt_at TEXT DEFAULT '')".into(),
    "CREATE TABLE IF NOT EXISTS llm_calls (id INTEGER PRIMARY KEY AUTOINCREMENT, model_id TEXT NOT NULL, model_name TEXT DEFAULT '', purpose TEXT DEFAULT '', prompt_tokens INTEGER DEFAULT 0, completion_tokens INTEGER DEFAULT 0, total_tokens INTEGER DEFAULT 0, duration_ms INTEGER DEFAULT 0, success INTEGER DEFAULT 1, error_message TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now'))); CREATE INDEX IF NOT EXISTS idx_llm_calls_time ON llm_calls(created_at)".into(),
    "CREATE TABLE IF NOT EXISTS summary_snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, chapter_number INTEGER NOT NULL, character_states TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now')))".into(),
]
}

fn project_core_get(c: &Connection) -> Result<Value, String> {
    c.execute(
        "INSERT OR IGNORE INTO project_core (id) VALUES ('main')",
        [],
    )
    .map_err(|e| e.to_string())?;
    c.query_row("SELECT * FROM project_core WHERE id='main'",[],|r|Ok(json!({"projectName":r.get::<_,String>("project_name")?,"genre":r.get::<_,String>("genre")?,"subGenre":r.get::<_,String>("sub_genre")?,"targetAudience":r.get::<_,String>("target_audience")?,"totalChapters":r.get::<_,i64>("total_chapters")?,"wordsPerChapter":r.get::<_,i64>("words_per_chapter")?,"plotStructure":r.get::<_,String>("plot_structure")?,"narrativePov":r.get::<_,String>("narrative_pov")?,"writingStyle":r.get::<_,String>("writing_style")?,"referenceWorks":r.get::<_,String>("reference_works")?,"globalGuidance":r.get::<_,String>("global_guidance")?,"goldenFinger":r.get::<_,String>("golden_finger")?,"premise":r.get::<_,String>("premise")?,"worldbuilding":r.get::<_,String>("worldbuilding")?,"charactersArch":r.get::<_,String>("characters_arch")?,"synopsis":r.get::<_,String>("synopsis")?,"characterStates":r.get::<_,String>("character_states")?}))).map_err(|e|e.to_string())
}
fn project_core_update(c: &Connection, v: Value) -> Result<Value, String> {
    c.execute(
        "INSERT OR IGNORE INTO project_core (id) VALUES ('main')",
        [],
    )
    .map_err(|e| e.to_string())?;
    for (k, col) in [
        ("projectName", "project_name"),
        ("genre", "genre"),
        ("subGenre", "sub_genre"),
        ("targetAudience", "target_audience"),
        ("totalChapters", "total_chapters"),
        ("wordsPerChapter", "words_per_chapter"),
        ("plotStructure", "plot_structure"),
        ("narrativePov", "narrative_pov"),
        ("writingStyle", "writing_style"),
        ("referenceWorks", "reference_works"),
        ("globalGuidance", "global_guidance"),
        ("goldenFinger", "golden_finger"),
        ("premise", "premise"),
        ("worldbuilding", "worldbuilding"),
        ("charactersArch", "characters_arch"),
        ("synopsis", "synopsis"),
        ("characterStates", "character_states"),
    ] {
        if let Some(x) = v.get(k) {
            c.execute(
                &format!(
                    "UPDATE project_core SET {col}=?, updated_at=datetime('now') WHERE id='main'"
                ),
                [to_text(x)],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(json!({"success":true}))
}

fn blueprint_all(c: &Connection) -> Result<Value, String> {
    collect(
        c,
        "SELECT * FROM blueprints ORDER BY chapter_number ASC",
        [],
        blueprint_row,
    )
}
fn blueprint_get(c: &Connection, n: i64) -> Result<Value, String> {
    c.query_row(
        "SELECT * FROM blueprints WHERE chapter_number=?",
        [n],
        blueprint_row,
    )
    .optional()
    .map(|x| x.unwrap_or(Value::Null))
    .map_err(|e| e.to_string())
}
fn blueprint_upsert(c: &Connection, v: Value) -> Result<Value, String> {
    c.execute("INSERT INTO blueprints (chapter_number,title,role,purpose,key_events,characters,suspense_hook,user_guidance,notes,notes_updated_at) VALUES (?,?,?,?,?,?,?,?,?,?) ON CONFLICT(chapter_number) DO UPDATE SET title=excluded.title,role=excluded.role,purpose=excluded.purpose,key_events=excluded.key_events,characters=excluded.characters,suspense_hook=excluded.suspense_hook,user_guidance=excluded.user_guidance,notes=excluded.notes,notes_updated_at=excluded.notes_updated_at,updated_at=datetime('now')",params![fi(&v,"chapterNumber"),fs(&v,"title"),fs(&v,"role"),fs(&v,"purpose"),fs(&v,"keyEvents"),v.get("characters").cloned().unwrap_or(json!([])).to_string(),fs(&v,"suspenseHook"),fs(&v,"userGuidance"),fs(&v,"notes"),fs(&v,"notesUpdatedAt")]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true}))
}
fn blueprint_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let chars: String = r.get("characters")?;
    Ok(
        json!({"chapterNumber":r.get::<_,i64>("chapter_number")?,"title":r.get::<_,String>("title")?,"role":r.get::<_,String>("role")?,"purpose":r.get::<_,String>("purpose")?,"keyEvents":r.get::<_,String>("key_events")?,"characters":serde_json::from_str::<Value>(&chars).unwrap_or(json!([])),"suspenseHook":r.get::<_,String>("suspense_hook")?,"userGuidance":r.get::<_,String>("user_guidance")?,"notes":r.get::<_,String>("notes")?,"notesUpdatedAt":r.get::<_,String>("notes_updated_at")?}),
    )
}

fn character_all(c: &Connection) -> Result<Value, String> {
    collect(c,"SELECT * FROM characters ORDER BY CASE role WHEN 'protagonist' THEN 0 WHEN 'supporting' THEN 1 WHEN 'antagonist' THEN 2 WHEN 'minor' THEN 3 ELSE 9 END ASC",[],character_row)
}
fn character_upsert(c: &Connection, v: Value) -> Result<Value, String> {
    let s = v.get("currentState").cloned().unwrap_or_default();
    c.execute("INSERT INTO characters (name,role,gender,age,appearance,personality,background,abilities,motivation,relationships,arc,notes,cs_location,cs_power_level,cs_physical_state,cs_mental_state,cs_key_items,cs_recent_events,cs_updated_at_chapter) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(name) DO UPDATE SET role=excluded.role,gender=excluded.gender,age=excluded.age,appearance=excluded.appearance,personality=excluded.personality,background=excluded.background,abilities=excluded.abilities,motivation=excluded.motivation,relationships=excluded.relationships,arc=excluded.arc,notes=excluded.notes,cs_location=excluded.cs_location,cs_power_level=excluded.cs_power_level,cs_physical_state=excluded.cs_physical_state,cs_mental_state=excluded.cs_mental_state,cs_key_items=excluded.cs_key_items,cs_recent_events=excluded.cs_recent_events,cs_updated_at_chapter=excluded.cs_updated_at_chapter,updated_at=datetime('now')",params![fs(&v,"name"),fss(&v,"role","supporting"),fs(&v,"gender"),fs(&v,"age"),fs(&v,"appearance"),fs(&v,"personality"),fs(&v,"background"),fs(&v,"abilities"),fs(&v,"motivation"),fs(&v,"relationships"),fs(&v,"arc"),fs(&v,"notes"),fs(&s,"location"),fs(&s,"powerLevel"),fs(&s,"physicalState"),fs(&s,"mentalState"),fs(&s,"keyItems"),fs(&s,"recentEvents"),fi(&s,"updatedAtChapter")]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true}))
}
fn character_update_state(c: &Connection, name: String, s: Value) -> Result<Value, String> {
    c.execute("UPDATE characters SET cs_location=?,cs_power_level=?,cs_physical_state=?,cs_mental_state=?,cs_key_items=?,cs_recent_events=?,cs_updated_at_chapter=?,updated_at=datetime('now') WHERE name=?",params![fs(&s,"location"),fs(&s,"powerLevel"),fs(&s,"physicalState"),fs(&s,"mentalState"),fs(&s,"keyItems"),fs(&s,"recentEvents"),fi(&s,"updatedAtChapter"),name]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true}))
}
fn character_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let n: i64 = r.get("cs_updated_at_chapter")?;
    let mut v = json!({"name":r.get::<_,String>("name")?,"role":r.get::<_,String>("role")?,"gender":r.get::<_,String>("gender")?,"age":r.get::<_,String>("age")?,"appearance":r.get::<_,String>("appearance")?,"personality":r.get::<_,String>("personality")?,"background":r.get::<_,String>("background")?,"abilities":r.get::<_,String>("abilities")?,"motivation":r.get::<_,String>("motivation")?,"relationships":r.get::<_,String>("relationships")?,"arc":r.get::<_,String>("arc")?,"notes":r.get::<_,String>("notes")?});
    if n > 0 {
        v["currentState"] = json!({"location":r.get::<_,String>("cs_location")?,"powerLevel":r.get::<_,String>("cs_power_level")?,"physicalState":r.get::<_,String>("cs_physical_state")?,"mentalState":r.get::<_,String>("cs_mental_state")?,"keyItems":r.get::<_,String>("cs_key_items")?,"recentEvents":r.get::<_,String>("cs_recent_events")?,"updatedAtChapter":n});
    }
    Ok(v)
}

fn draft_create(c: &Connection, v: Value) -> Result<Value, String> {
    c.execute(
        "INSERT INTO contents (body) VALUES (?)",
        [fs(&v, "content")],
    )
    .map_err(|e| e.to_string())?;
    let cid = c.last_insert_rowid();
    c.execute("INSERT INTO drafts (chapter_number,version,source,content_id,word_count) VALUES (?,?,?,?,?)",params![fi(&v,"chapterNumber"),fi(&v,"version"),fss(&v,"source","write"),cid,fi(&v,"wordCount")]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true,"id":c.last_insert_rowid()}))
}
fn draft_list(c: &Connection, n: i64) -> Result<Value, String> {
    collect(
        c,
        "SELECT * FROM drafts WHERE chapter_number=? ORDER BY version ASC",
        [n],
        draft_row,
    )
}
fn draft_meta(c: &Connection, id: i64) -> Result<Value, String> {
    c.query_row("SELECT * FROM drafts WHERE id=?", [id], draft_row)
        .optional()
        .map(|x| x.unwrap_or(Value::Null))
        .map_err(|e| e.to_string())
}
fn draft_full(c: &Connection, id: i64) -> Result<Value, String> {
    c.query_row(
        "SELECT d.*,c.body FROM drafts d LEFT JOIN contents c ON c.id=d.content_id WHERE d.id=?",
        [id],
        draft_full_row,
    )
    .optional()
    .map(|x| x.unwrap_or(Value::Null))
    .map_err(|e| e.to_string())
}
fn draft_one(c: &Connection, sql: &str, n: i64) -> Result<Value, String> {
    c.query_row(sql, [n], draft_row)
        .optional()
        .map(|x| x.unwrap_or(Value::Null))
        .map_err(|e| e.to_string())
}
fn update_draft_status(
    c: &Connection,
    id: i64,
    status: String,
    wc: Option<i64>,
) -> Result<Value, String> {
    if let Some(w) = wc {
        c.execute(
            "UPDATE drafts SET status=?,word_count=?,updated_at=datetime('now') WHERE id=?",
            params![status, w, id],
        )
    } else {
        c.execute(
            "UPDATE drafts SET status=?,updated_at=datetime('now') WHERE id=?",
            params![status, id],
        )
    }
    .map_err(|e| e.to_string())?;
    Ok(json!({"success":true}))
}
fn update_draft_content(c: &Connection, id: i64, body: String, wc: i64) -> Result<Value, String> {
    let cid: Option<i64> = c
        .query_row("SELECT content_id FROM drafts WHERE id=?", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(cid) = cid {
        c.execute("UPDATE contents SET body=? WHERE id=?", params![body, cid])
            .map_err(|e| e.to_string())?;
        c.execute(
            "UPDATE drafts SET word_count=?,updated_at=datetime('now') WHERE id=?",
            params![wc, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(json!({"success":true}))
}
fn draft_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,i64>("id")?,"chapterNumber":r.get::<_,i64>("chapter_number")?,"version":r.get::<_,i64>("version")?,"status":r.get::<_,String>("status")?,"source":r.get::<_,String>("source")?,"contentId":r.get::<_,i64>("content_id")?,"wordCount":r.get::<_,i64>("word_count")?,"createdAt":r.get::<_,String>("created_at")?,"updatedAt":r.get::<_,String>("updated_at")?}),
    )
}
fn draft_full_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let mut v = draft_row(r)?;
    v["content"] = json!(r.get::<_, Option<String>>("body")?.unwrap_or_default());
    Ok(v)
}

fn branch_create(c: &Connection, _table: &str, v: Value) -> Result<Value, String> {
    c.execute(
        "INSERT INTO contents (body) VALUES (?)",
        [fs(&v, "content")],
    )
    .map_err(|e| e.to_string())?;
    let cid = c.last_insert_rowid();
    c.execute("INSERT INTO revisions (base_draft_id,revision_index,revision_type,user_prompt,review_source_id,content_id,word_count) VALUES (?,?,?,?,?,?,?)",params![fi(&v,"baseDraftId"),fi(&v,"revisionIndex"),fss(&v,"revisionType","refine"),fs(&v,"userPrompt"),v.get("reviewSourceId").and_then(Value::as_i64),cid,fi(&v,"wordCount")]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true,"id":c.last_insert_rowid()}))
}
fn branch_list(
    c: &Connection,
    _t: &str,
    _k: &str,
    id: i64,
    pending: bool,
) -> Result<Value, String> {
    let sql = if pending {
        "SELECT * FROM revisions WHERE base_draft_id=? AND status='pending' ORDER BY revision_index ASC"
    } else {
        "SELECT * FROM revisions WHERE base_draft_id=? ORDER BY revision_index ASC"
    };
    collect(c, sql, [id], revision_row)
}
fn branch_full(c: &Connection, _t: &str, id: i64) -> Result<Value, String> {
    c.query_row(
        "SELECT r.*,c.body FROM revisions r LEFT JOIN contents c ON c.id=r.content_id WHERE r.id=?",
        [id],
        revision_full_row,
    )
    .optional()
    .map(|x| x.unwrap_or(Value::Null))
    .map_err(|e| e.to_string())
}
fn revision_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,i64>("id")?,"baseDraftId":r.get::<_,i64>("base_draft_id")?,"revisionIndex":r.get::<_,i64>("revision_index")?,"revisionType":r.get::<_,String>("revision_type")?,"status":r.get::<_,String>("status")?,"mergedToDraftId":r.get::<_,Option<i64>>("merged_to_draft_id")?,"userPrompt":r.get::<_,String>("user_prompt")?,"reviewSourceId":r.get::<_,Option<i64>>("review_source_id")?,"contentId":r.get::<_,i64>("content_id")?,"wordCount":r.get::<_,i64>("word_count")?,"createdAt":r.get::<_,String>("created_at")?,"updatedAt":r.get::<_,String>("updated_at")?}),
    )
}
fn revision_full_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let mut v = revision_row(r)?;
    v["content"] = json!(r.get::<_, Option<String>>("body")?.unwrap_or_default());
    Ok(v)
}

fn review_create(c: &Connection, v: Value) -> Result<Value, String> {
    c.execute(
        "INSERT INTO contents (body) VALUES (?)",
        [fs(&v, "content")],
    )
    .map_err(|e| e.to_string())?;
    let cid = c.last_insert_rowid();
    c.execute(
        "INSERT INTO reviews (base_draft_id,review_index,content_id) VALUES (?,?,?)",
        params![fi(&v, "baseDraftId"), fi(&v, "reviewIndex"), cid],
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({"success":true,"id":c.last_insert_rowid()}))
}
fn review_list(c: &Connection, id: i64) -> Result<Value, String> {
    collect(
        c,
        "SELECT * FROM reviews WHERE base_draft_id=? ORDER BY review_index ASC",
        [id],
        review_row,
    )
}
fn review_latest(c: &Connection, id: i64) -> Result<Value, String> {
    c.query_row("SELECT r.*,c.body FROM reviews r LEFT JOIN contents c ON c.id=r.content_id WHERE r.base_draft_id=? ORDER BY r.review_index DESC LIMIT 1",[id],review_full_row).optional().map(|x|x.unwrap_or(Value::Null)).map_err(|e|e.to_string())
}
fn review_full(c: &Connection, id: i64) -> Result<Value, String> {
    c.query_row(
        "SELECT r.*,c.body FROM reviews r LEFT JOIN contents c ON c.id=r.content_id WHERE r.id=?",
        [id],
        review_full_row,
    )
    .optional()
    .map(|x| x.unwrap_or(Value::Null))
    .map_err(|e| e.to_string())
}
fn review_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,i64>("id")?,"baseDraftId":r.get::<_,i64>("base_draft_id")?,"reviewIndex":r.get::<_,i64>("review_index")?,"contentId":r.get::<_,i64>("content_id")?,"createdAt":r.get::<_,String>("created_at")?}),
    )
}
fn review_full_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let mut v = review_row(r)?;
    v["content"] = json!(r.get::<_, Option<String>>("body")?.unwrap_or_default());
    Ok(v)
}

fn post_run_create(c: &Connection, v: Value) -> Result<Value, String> {
    let id = uuid::Uuid::new_v4().to_string();
    c.execute("INSERT INTO post_process_runs (id,trigger_source_type,trigger_source_id,source_label) VALUES (?,?,?,?)",params![id,fs(&v,"triggerSourceType"),fs(&v,"triggerSourceId"),fs(&v,"sourceLabel")]).map_err(|e|e.to_string())?;
    for s in v
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        c.execute(
            "INSERT INTO post_process_steps (run_id,step_key,label,critical) VALUES (?,?,?,?)",
            params![
                id,
                fs(&s, "key"),
                fs(&s, "label"),
                if s.get("critical").and_then(Value::as_bool).unwrap_or(false) {
                    1
                } else {
                    0
                }
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(json!({"success":true,"id":id}))
}
fn post_run_latest(c: &Connection, t: String, id: String) -> Result<Value, String> {
    c.query_row("SELECT * FROM post_process_runs WHERE trigger_source_type=? AND trigger_source_id=? ORDER BY created_at DESC LIMIT 1",params![t,id],post_run_row).optional().map(|x|x.unwrap_or(Value::Null)).map_err(|e|e.to_string())
}
fn post_steps(c: &Connection, id: String) -> Result<Value, String> {
    collect(
        c,
        "SELECT * FROM post_process_steps WHERE run_id=? ORDER BY id ASC",
        [id],
        post_step_row,
    )
}
fn post_step_mark(
    c: &Connection,
    run: String,
    key: String,
    err: Option<String>,
) -> Result<Value, String> {
    if let Some(e)=err{c.execute("UPDATE post_process_steps SET ok=0,error_msg=?,last_attempt_at=datetime('now'),attempt_count=attempt_count+1 WHERE run_id=? AND step_key=?",params![e,run,key])}else{c.execute("UPDATE post_process_steps SET ok=1,completed_at=datetime('now'),last_attempt_at=datetime('now'),attempt_count=attempt_count+1 WHERE run_id=? AND step_key=?",params![run,key])}.map_err(|e|e.to_string())?;
    let left: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM post_process_steps WHERE run_id=? AND critical=1 AND ok=0",
            [run.clone()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE post_process_runs SET all_critical_passed=?,updated_at=datetime('now') WHERE id=?",
        params![if left == 0 { 1 } else { 0 }, run],
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({"success":true}))
}
fn post_all_passed(c: &Connection, t: String, id: String) -> Result<Value, String> {
    Ok(json!(post_run_latest(c, t, id)?
        .get("allCriticalPassed")
        .and_then(Value::as_bool)
        .unwrap_or(false)))
}
fn post_run_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,String>("id")?,"triggerSourceType":r.get::<_,String>("trigger_source_type")?,"triggerSourceId":r.get::<_,String>("trigger_source_id")?,"sourceLabel":r.get::<_,String>("source_label")?,"allCriticalPassed":r.get::<_,i64>("all_critical_passed")?==1,"createdAt":r.get::<_,String>("created_at")?,"updatedAt":r.get::<_,String>("updated_at")?}),
    )
}
fn post_step_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,i64>("id")?,"runId":r.get::<_,String>("run_id")?,"stepKey":r.get::<_,String>("step_key")?,"label":r.get::<_,String>("label")?,"critical":r.get::<_,i64>("critical")?==1,"ok":r.get::<_,i64>("ok")?==1,"errorMsg":r.get::<_,String>("error_msg")?,"attemptCount":r.get::<_,i64>("attempt_count")?,"completedAt":r.get::<_,String>("completed_at")?,"lastAttemptAt":r.get::<_,String>("last_attempt_at")?}),
    )
}

fn log_llm(c: &Connection, v: Value) -> Result<Value, String> {
    c.execute("INSERT INTO llm_calls (model_id,model_name,purpose,prompt_tokens,completion_tokens,total_tokens,duration_ms,success,error_message) VALUES (?,?,?,?,?,?,?,?,?)",params![fs(&v,"modelId"),fs(&v,"modelName"),fs(&v,"purpose"),fi(&v,"promptTokens"),fi(&v,"completionTokens"),fi(&v,"totalTokens"),fi(&v,"durationMs"),if v.get("success").and_then(Value::as_bool).unwrap_or(true){1}else{0},fs(&v,"errorMessage")]).map_err(|e|e.to_string())?;
    Ok(json!({"success":true}))
}
fn llm_stats(c: &Connection) -> Result<Value, String> {
    c.query_row("SELECT COUNT(*) a,COALESCE(SUM(total_tokens),0) b,COALESCE(SUM(prompt_tokens),0) c,COALESCE(SUM(completion_tokens),0) d FROM llm_calls",[],|r|Ok(json!({"totalCalls":r.get::<_,i64>(0)?,"totalTokens":r.get::<_,i64>(1)?,"totalPromptTokens":r.get::<_,i64>(2)?,"totalCompletionTokens":r.get::<_,i64>(3)?}))).map_err(|e|e.to_string())
}
fn llm_history(c: &Connection, limit: i64) -> Result<Value, String> {
    collect(
        c,
        "SELECT * FROM llm_calls ORDER BY id DESC LIMIT ?",
        [limit],
        |r| {
            Ok(
                json!({"id":r.get::<_,i64>("id")?,"modelId":r.get::<_,String>("model_id")?,"modelName":r.get::<_,String>("model_name")?,"purpose":r.get::<_,String>("purpose")?,"promptTokens":r.get::<_,i64>("prompt_tokens")?,"completionTokens":r.get::<_,i64>("completion_tokens")?,"totalTokens":r.get::<_,i64>("total_tokens")?,"durationMs":r.get::<_,i64>("duration_ms")?,"success":r.get::<_,i64>("success")?==1,"errorMessage":r.get::<_,String>("error_message")?,"createdAt":r.get::<_,String>("created_at")?}),
            )
        },
    )
}
fn latest_summary(c: &Connection) -> Result<Value, String> {
    c.query_row(
        "SELECT character_states,chapter_number FROM summary_snapshots ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok(json!({"characterStates":r.get::<_,String>(0)?,"chapterNumber":r.get::<_,i64>(1)?})),
    )
    .optional()
    .map(|x| x.unwrap_or(Value::Null))
    .map_err(|e| e.to_string())
}

fn exec_ok<P: rusqlite::Params>(c: &Connection, sql: &str, p: P) -> Result<Value, String> {
    c.execute(sql, p).map_err(|e| e.to_string())?;
    Ok(json!({"success":true}))
}
fn scalar_i64<P: rusqlite::Params>(c: &Connection, sql: &str, p: P) -> Result<Value, String> {
    let n: i64 = c
        .query_row(sql, p, |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(json!(n))
}
fn scalar_i64_plus_one<P: rusqlite::Params>(
    c: &Connection,
    sql: &str,
    p: P,
) -> Result<Value, String> {
    let n: i64 = c
        .query_row(sql, p, |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(json!(n + 1))
}
fn collect<P, F>(c: &Connection, sql: &str, p: P, f: F) -> Result<Value, String>
where
    P: rusqlite::Params,
    F: FnMut(&rusqlite::Row) -> rusqlite::Result<Value>,
{
    let mut s = c.prepare(sql).map_err(|e| e.to_string())?;
    let rows = s.query_map(p, f).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?)
    }
    Ok(Value::Array(out))
}
fn arg_value(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}
fn arg_i64(args: &[Value], i: usize) -> Result<i64, String> {
    args.get(i)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("Invalid numeric argument at {i}"))
}
fn arg_string(args: &[Value], i: usize) -> Result<String, String> {
    args.get(i)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Invalid string argument at {i}"))
}
fn fi(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
fn fs(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
fn fss(v: &Value, k: &str, d: &str) -> String {
    let s = fs(v, k);
    if s.is_empty() {
        d.to_string()
    } else {
        s
    }
}
fn to_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "1" } else { "0" }.into(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}
