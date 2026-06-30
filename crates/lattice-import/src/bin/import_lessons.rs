//! `lattice-import-lessons` CLI — Lane-A grounded lesson generation (spec §10.5).
//!
//! Drop converted openly-licensed section text into a grounding directory, one
//! file per concept (`<concept_id>.md` / `.txt`), then:
//! ```text
//! ANTHROPIC_API_KEY=sk-... \
//!   cargo run -p lattice-import --bin lattice-import-lessons -- \
//!     --grounding /path/to/openstax-sections \
//!     --subject subjects/math \
//!     --source-label "OpenStax Calculus Vol 1" \
//!     --license cc-by
//! ```
//! Writes `subjects/<id>/notes/<concept>.md` with attribution frontmatter, only
//! for concepts that don't already have a lesson (use `--force` to overwrite).

use std::path::PathBuf;
use std::process::ExitCode;

use lattice_content::load_subject;
use lattice_import::{parse_license, run_lesson_import, LocalGroundingSource};
use lattice_llm::{Provider, ProviderConfig};

struct Args {
    grounding: PathBuf,
    subject: PathBuf,
    provider: Provider,
    model: Option<String>,
    source_label: String,
    license: String,
    limit: Option<usize>,
    force: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut grounding = None;
    let mut subject = PathBuf::from("subjects/math");
    let mut provider = Provider::Anthropic;
    let mut model = None;
    let mut source_label = None;
    let mut license = "cc-by".to_string();
    let mut limit = None;
    let mut force = false;

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("missing value for {flag}"));
        match flag.as_str() {
            "--grounding" => grounding = Some(PathBuf::from(value()?)),
            "--subject" => subject = PathBuf::from(value()?),
            "--provider" => {
                provider = match value()?.to_ascii_lowercase().as_str() {
                    "anthropic" => Provider::Anthropic,
                    "openai" => Provider::OpenAi,
                    "gemini" => Provider::Gemini,
                    other => return Err(format!("unknown provider `{other}`")),
                }
            }
            "--model" => model = Some(value()?),
            "--source-label" => source_label = Some(value()?),
            "--license" => license = value()?,
            "--limit" => {
                limit = Some(value()?.parse().map_err(|_| "--limit expects a number".to_string())?)
            }
            "--force" => force = true,
            "-h" | "--help" => return Err("help".to_string()),
            other => return Err(format!("unknown flag `{other}`")),
        }
    }

    Ok(Args {
        grounding: grounding.ok_or("--grounding <dir> is required")?,
        subject,
        provider,
        source_label: source_label.ok_or("--source-label <text> is required (for attribution)")?,
        license,
        model,
        limit,
        force,
    })
}

fn api_key_env(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "ANTHROPIC_API_KEY",
        Provider::OpenAi => "OPENAI_API_KEY",
        Provider::Gemini => "GEMINI_API_KEY",
    }
}

const USAGE: &str = "usage: lattice-import-lessons --grounding <dir> --source-label <text> \
[--subject <dir>] [--license cc-by|cc-by-nc-sa|mit|personal] \
[--provider anthropic|openai|gemini] [--model <id>] [--limit <n>] [--force]\n\
grounding files are named <concept_id>.md/.txt; the API key comes from \
ANTHROPIC_API_KEY / OPENAI_API_KEY / GEMINI_API_KEY.";

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            if msg == "help" {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            eprintln!("error: {msg}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let key = match std::env::var(api_key_env(args.provider)) {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            eprintln!("error: set {} to your API key", api_key_env(args.provider));
            return ExitCode::FAILURE;
        }
    };
    let config = ProviderConfig {
        provider: args.provider,
        api_key: key,
        model: args
            .model
            .unwrap_or_else(|| args.provider.default_model().to_string()),
    };

    let subject = match load_subject(&args.subject) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: loading subject at {}: {e}", args.subject.display());
            return ExitCode::FAILURE;
        }
    };

    let source = LocalGroundingSource::new(
        &args.grounding,
        &args.source_label,
        parse_license(&args.license),
    );
    let notes_dir = args.subject.join("notes");

    println!(
        "Generating grounded lessons from {} into {} …",
        args.grounding.display(),
        notes_dir.display()
    );
    let report = match run_lesson_import(
        &config,
        &source,
        &subject,
        &notes_dir,
        args.limit,
        args.force,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "\nDone. wrote {} lesson(s){} · skipped {} (already had a lesson) + {} (no matching concept)",
        report.written.len(),
        if report.written.is_empty() {
            String::new()
        } else {
            format!(
                " [{}]",
                report.written.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ")
            )
        },
        report.skipped_existing,
        report.skipped_unmapped,
    );
    ExitCode::SUCCESS
}
