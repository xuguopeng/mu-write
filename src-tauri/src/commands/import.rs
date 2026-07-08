use crate::commands::arg;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedChapter {
    number: usize,
    title: String,
    content: String,
    word_count: usize,
}

pub async fn dispatch_import(
    app: AppHandle,
    channel: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    match channel {
        "dialog:select-novel-files" => select_novel_files(app).await,
        "dialog:select-files" => select_novel_files(app).await,
        "dialog:select-import-folder" => select_import_folder(app).await,
        "import:split-chapters" => split_chapters(args).await,
        _ => Err(format!("Unsupported IPC channel: {channel}")),
    }
}

async fn select_novel_files(app: AppHandle) -> Result<Value, String> {
    let files = app
        .dialog()
        .file()
        .add_filter("Novel Text", &["txt", "md", "text"])
        .blocking_pick_files();

    let paths = files.map(|files| {
        files
            .into_iter()
            .filter_map(|path| path.as_path().map(|p| p.to_string_lossy().to_string()))
            .collect::<Vec<_>>()
    });

    Ok(json!(paths))
}

async fn select_import_folder(app: AppHandle) -> Result<Value, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    let path = folder.and_then(|path| path.as_path().map(|p| p.to_string_lossy().to_string()));
    Ok(json!(path))
}

async fn split_chapters(args: Vec<Value>) -> Result<Value, String> {
    let file_paths: Vec<String> = arg(&args, 0, "filePaths")?;
    if file_paths.is_empty() {
        return Ok(
            json!({ "success": false, "chapters": [], "totalWords": 0, "error": "未选择文件" }),
        );
    }

    let mut chapters = Vec::new();
    for path in file_paths {
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("读取导入文件失败 '{}': {error}", path))?;
        let mut parsed = split_content_into_chapters(&content, &path);
        chapters.append(&mut parsed);
    }

    chapters.sort_by_key(|chapter| chapter.number);
    renumber_duplicate_or_empty(&mut chapters);

    let total_words = chapters
        .iter()
        .map(|chapter| chapter.word_count)
        .sum::<usize>();
    Ok(json!({
        "success": true,
        "chapters": chapters,
        "totalWords": total_words,
    }))
}

fn split_content_into_chapters(content: &str, path: &str) -> Vec<ImportedChapter> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut markers = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if let Some((number, title)) = parse_chapter_heading(line) {
            markers.push((index, number, title));
        }
    }

    if markers.is_empty() {
        let title = PathBuf::from(path)
            .file_stem()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "导入正文".to_string());
        return vec![build_chapter(1, title, normalized.trim().to_string())];
    }

    let mut chapters = Vec::new();
    for (idx, (start, number, title)) in markers.iter().enumerate() {
        let end = markers
            .get(idx + 1)
            .map(|next| next.0)
            .unwrap_or(lines.len());
        let body = lines[*start..end].join("\n").trim().to_string();
        if !body.is_empty() {
            chapters.push(build_chapter(*number, title.clone(), body));
        }
    }

    chapters
}

fn parse_chapter_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return None;
    }

    let candidates = ["第", "正文 第", "### 第", "## 第", "# 第"];
    if !candidates.iter().any(|prefix| trimmed.starts_with(prefix)) {
        return None;
    }

    let start = trimmed.find('第')? + '第'.len_utf8();
    let rest = &trimmed[start..];
    let chapter_pos = rest
        .find('章')
        .or_else(|| rest.find('回'))
        .or_else(|| rest.find('节'))?;
    let number_part = rest[..chapter_pos].trim();
    let number = parse_chapter_number(number_part)?;
    let title = rest[chapter_pos + '章'.len_utf8()..]
        .trim_matches(|c: char| c.is_whitespace() || c == ':' || c == '：' || c == '-' || c == '—')
        .trim()
        .to_string();

    Some((number, title))
}

fn parse_chapter_number(raw: &str) -> Option<usize> {
    if let Ok(n) = raw.parse::<usize>() {
        return Some(n);
    }

    chinese_number(raw)
}

fn chinese_number(raw: &str) -> Option<usize> {
    let mut total = 0usize;
    let mut section = 0usize;
    let mut number = 0usize;
    let mut seen = false;

    for ch in raw.chars() {
        let digit = match ch {
            '零' | '〇' => Some(0),
            '一' => Some(1),
            '二' | '两' => Some(2),
            '三' => Some(3),
            '四' => Some(4),
            '五' => Some(5),
            '六' => Some(6),
            '七' => Some(7),
            '八' => Some(8),
            '九' => Some(9),
            _ => None,
        };

        if let Some(value) = digit {
            number = value;
            seen = true;
            continue;
        }

        match ch {
            '十' => {
                section += if number == 0 { 10 } else { number * 10 };
                number = 0;
                seen = true;
            }
            '百' => {
                section += if number == 0 { 100 } else { number * 100 };
                number = 0;
                seen = true;
            }
            '千' => {
                section += if number == 0 { 1000 } else { number * 1000 };
                number = 0;
                seen = true;
            }
            '万' => {
                total += (section + number).max(1) * 10_000;
                section = 0;
                number = 0;
                seen = true;
            }
            _ => return None,
        }
    }

    if !seen {
        return None;
    }
    Some(total + section + number)
}

fn build_chapter(number: usize, title: String, content: String) -> ImportedChapter {
    ImportedChapter {
        number,
        title: if title.is_empty() {
            format!("第{number}章")
        } else {
            title
        },
        word_count: count_words(&content),
        content,
    }
}

fn count_words(content: &str) -> usize {
    content.chars().filter(|ch| !ch.is_whitespace()).count()
}

fn renumber_duplicate_or_empty(chapters: &mut [ImportedChapter]) {
    let mut seen = std::collections::HashSet::new();
    let has_duplicate = chapters.iter().any(|chapter| !seen.insert(chapter.number));
    if has_duplicate || chapters.iter().any(|chapter| chapter.number == 0) {
        for (index, chapter) in chapters.iter_mut().enumerate() {
            chapter.number = index + 1;
        }
    }
}
