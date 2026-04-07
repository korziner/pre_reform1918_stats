use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use serde::Deserialize;
use serde_json::{Map, Value, Number};
use regex::Regex;
use clap::Parser;
use rayon::prelude::*;

fn deserialize_date<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(Error::custom("Ожидалась строка или число для поля date")),
    }
}

#[derive(Parser)]
#[command(
    author = "Романъ Тельповъ",
    version = "0.7.1",
    about = long_help(),
    long_about = None,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help = "Показать справку")]
    _help: (),

    #[arg(short = 'r', long = "rules-file", help = "Путь къ JSON-файлу съ правилами")]
    rules_file: Option<PathBuf>,

    #[arg(short = 'v', long = "verbose", help = "Подробная статистика", action = clap::ArgAction::SetTrue)]
    verbose: bool,

    #[arg(short = 't', long = "threads", help = "Число потоковъ", value_name = "N")]
    threads: Option<usize>,

    #[arg(short = 'b', long = "batch-size", help = "Размѣръ пакета", value_name = "N", default_value = "1000")]
    batch_size: usize,

    #[arg(long = "no-parallel", help = "Отключить параллельность", action = clap::ArgAction::SetTrue)]
    no_parallel: bool,

    #[arg(short = 'l', long = "title-len", help = "Длина title", value_name = "N", default_value = "50")]
    title_len: usize,
}

fn long_help() -> &'static str {
    r#"
ПРЕ-РЕФОРМЕННЫЙ ОРѲОГРАФИЧЕСКІЙ АНАЛИЗАТОРЪ v0.7.1
===================================================

ПАРАЛЛЕЛЬНАЯ ОБРАБОТКА: rayon на всѣ ядра CPU.

ФОРМИРОВАНИЕ title:
  • JSONL: изъ поля "title"
  • Текстъ: первые N + "..." + послѣдніе N символовъ (-l N)

ОСОБЕННОСТИ:
  • Буквы: і, ѳ, ѵ, ѣ
  • Окончанія: -ыя, -аго, -ыи, -ія
  • Твёрдый знакъ: ъ\b
  • Двойныя согласныя, приставки раз-/без-/воз-
  • Точки аббревіатуръ: \b[А-ЯЁ]\. (только одиночныя заглавныя)

ПРИМѢРЫ:
  $ cat text.txt | ./pre_reform_stats
  $ ./pre_reform_stats -t 8 -l 100 < input.txt
  $ ./pre_reform_stats -v < data.txt

© 1918–2026. Всѣ права сохранены.
"#
}

#[derive(Deserialize, Clone)]
struct InputRecord {
    #[serde(default)] text: String,
    #[serde(default)] title: String,
    #[serde(default)] collection: String,
    #[serde(default, deserialize_with = "deserialize_date")] date: String,
}

#[derive(Deserialize)]
struct CustomRule { name: String, regex: String }

#[derive(Clone)]
struct InputLine {
    index: usize,
    text: String,
    title: String,
    collection: String,
    date: String,
}

struct OutputLine { index: usize, json: String }

fn build_hardcoded_rules() -> Vec<(String, Regex)> {
    vec![
        (String::from("і_десятеричное"), Regex::new(r"і[аеёиоуыэюяй]").unwrap()),
        (String::from("ѳ_фита"), Regex::new(r"ѳ").unwrap()),
        (String::from("ѵ_ижица"), Regex::new(r"ѵ").unwrap()),
        (String::from("ѣ_ять"), Regex::new(r"ѣ").unwrap()),
        (String::from("оконч_ыя"), Regex::new(r"ыя\b").unwrap()),
        (String::from("оконч_аго"), Regex::new(r"аго\b").unwrap()),
        (String::from("оконч_ыи"), Regex::new(r"ыи\b").unwrap()),
        (String::from("оконч_ія"), Regex::new(r"ія\b").unwrap()),
        (String::from("еръ_конечный"), Regex::new(r"ъ\b").unwrap()),
//         (String::from("двойн_согласн"), Regex::new(r"(?i)([бвгджзклмнпрстфхцчшщж])\1").unwrap()),
        (String::from("приставка_раз_с"), Regex::new(r"\bраз[сз]").unwrap()),
        (String::from("приставка_без_с"), Regex::new(r"\bбез[сз]").unwrap()),
        (String::from("приставка_воз_с"), Regex::new(r"\bвоз[сз]").unwrap()),
        (String::from("приставка_из_с"), Regex::new(r"\bиз[сз]").unwrap()),
        // ИСПРАВЛЕНО: \b передъ [А-ЯЁ] исключаетъ концы длинныхъ словъ
        (String::from("точки_аббревіатуръ"), Regex::new(r"\b[А-ЯЁ]\.").unwrap()),
    ]
}

fn make_title_from_text(text: &str, len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if total <= len * 2 {
        text.to_string()
    } else {
        let start: String = chars[..len].iter().collect();
        let end: String = chars[(total - len)..].iter().collect();
        format!("{}...{}", start, end)
    }
}

fn parse_line(line: &str, index: usize, title_len: usize) -> InputLine {
    if line.trim().starts_with('{') {
        if let Ok(record) = serde_json::from_str::<InputRecord>(line) {
            return InputLine {
                index,
                text: record.text,
                title: record.title,
                collection: record.collection,
                date: record.date,
            };
        }
    }
    InputLine {
        index,
        text: line.to_string(),
        title: make_title_from_text(line, title_len),
        collection: String::new(),
        date: String::new(),
    }
}

fn process_input(input: &InputLine, patterns: &[(String, Regex)], verbose: bool) -> OutputLine {
    let text = &input.text;
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    let total = chars as f64;

    let mut features = Map::new();
    let mut matches = Map::new();

    for (name, re) in patterns {
        let count = re.find_iter(text).count();
        let pct = if total > 0.0 { (count as f64 / total) * 100.0 } else { 0.0 };
        let rounded = (pct * 10_000.0).round() / 10_000.0;
        let num = Number::from_f64(rounded).unwrap_or_else(|| Number::from(0i64));
        features.insert(name.clone(), Value::Number(num));
        if verbose {
            matches.insert(name.clone(), Value::Number((count as i64).into()));
        }
    }

    let mut out = Map::new();
    out.insert("title".into(), Value::String(input.title.clone()));
    out.insert("collection".into(), Value::String(input.collection.clone()));
    out.insert("date".into(), Value::String(input.date.clone()));
    out.insert("words".into(), Value::Number((words as i64).into()));
    out.insert("chars".into(), Value::Number((chars as i64).into()));
    out.insert("features_percent".into(), Value::Object(features));
    if verbose {
        out.insert("matches".into(), Value::Object(matches));
    }

    OutputLine {
        index: input.index,
        json: serde_json::to_string(&Value::Object(out)).unwrap_or_default(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }

    let mut patterns = build_hardcoded_rules();

    if let Some(path) = &cli.rules_file {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Невозможно прочесть файлъ правилъ {}: {}", path.display(), e))?;
        let custom: Vec<CustomRule> = serde_json::from_str(&content)
            .map_err(|e| format!("Ошибка парсинга правилъ: {}", e))?;
        for rule in custom {
            let re = Regex::new(&rule.regex)
                .map_err(|e| format!("Некорректное правило '{}': {}", rule.name, e))?;
            patterns.push((rule.name, re));
        }
    }

    let patterns = Arc::new(patterns);
    let verbose = cli.verbose;
    let batch_size = cli.batch_size;
    let no_parallel = cli.no_parallel;
    let title_len = cli.title_len;

    let stdin = io::stdin();
    let mut lines: Vec<InputLine> = Vec::new();
    let mut index: usize = 0;

    for line_result in stdin.lock().lines() {
        let line = line_result?;
        if line.trim().is_empty() { continue; }
        lines.push(parse_line(&line, index, title_len));
        index += 1;
    }

    let mut results: Vec<OutputLine> = if no_parallel {
        lines.into_iter().map(|input| process_input(&input, &patterns, verbose)).collect()
    } else {
        lines.par_iter().with_min_len(batch_size).map(|input| process_input(input, &patterns, verbose)).collect()
    };

    results.par_sort_by_key(|r| r.index);

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for result in results {
        writeln!(handle, "{}", result.json)?;
    }

    Ok(())
}
