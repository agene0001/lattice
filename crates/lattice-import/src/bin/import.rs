//! `lattice-import` CLI — run the import pipeline against a dataset and emit
//! `problems.json` (spec §10).
//!
//! Example:
//! ```text
//! ANTHROPIC_API_KEY=sk-... \
//!   cargo run -p lattice-import -- \
//!     --dataset /path/to/MATH/train \
//!     --subject subjects/math \
//!     --provider anthropic \
//!     --limit 25
//! ```
//! Tagged + re-solve-verified problems are merged into `<subject>/problems.json`;
//! unverified ones land in `<subject>/problems.review.json` for manual checking.
//! Re-runs de-duplicate by id, so it's safe to resume.

use std::path::PathBuf;
use std::process::ExitCode;

use lattice_content::load_subject;
use lattice_import::{merge_into_problems_json, run_import, ConceptVocab, MathDatasetSource};
use lattice_llm::{Provider, ProviderConfig};

struct Args {
    dataset: PathBuf,
    subject: PathBuf,
    provider: Provider,
    model: Option<String>,
    limit: Option<usize>,
    id_prefix: String,
}

fn parse_args() -> Result<Args, String> {
    let mut dataset = None;
    let mut subject = PathBuf::from("subjects/math");
    let mut provider = Provider::Anthropic;
    let mut model = None;
    let mut limit = None;
    let mut id_prefix = "math".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("missing value for {flag}"));
        match flag.as_str() {
            "--dataset" => dataset = Some(PathBuf::from(value()?)),
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
            "--limit" => {
                limit = Some(value()?.parse().map_err(|_| "--limit expects a number".to_string())?)
            }
            "--id-prefix" => id_prefix = value()?,
            "-h" | "--help" => return Err("help".to_string()),
            other => return Err(format!("unknown flag `{other}`")),
        }
    }

    Ok(Args {
        dataset: dataset.ok_or("--dataset <dir> is required")?,
        subject,
        provider,
        model,
        limit,
        id_prefix,
    })
}

fn api_key_env(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "ANTHROPIC_API_KEY",
        Provider::OpenAi => "OPENAI_API_KEY",
        Provider::Gemini => "GEMINI_API_KEY",
    }
}

const USAGE: &str = "usage: lattice-import --dataset <dir> [--subject <dir>] \
[--provider anthropic|openai|gemini] [--model <id>] [--limit <n>] [--id-prefix <s>]\n\
the API key is read from ANTHROPIC_API_KEY / OPENAI_API_KEY / GEMINI_API_KEY.";

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
    let vocab = ConceptVocab::new(
        subject
            .concepts
            .iter()
            .map(|c| (c.id.clone(), c.label.clone())),
    );

    let source = MathDatasetSource::new(&args.dataset);
    println!(
        "Importing from {} into subject `{}` (limit {})…",
        args.dataset.display(),
        subject.id,
        args.limit.map(|n| n.to_string()).unwrap_or_else(|| "none".into())
    );

    let report = match run_import(&config, &source, &vocab, &args.id_prefix, args.limit).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out = args.subject.join("problems.json");
    let review = args.subject.join("problems.review.json");
    let added = match merge_into_problems_json(&out, &report.imported) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error writing {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
    };
    let parked = if report.needs_review.is_empty() {
        0
    } else {
        match merge_into_problems_json(&review, &report.needs_review) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("error writing {}: {e}", review.display());
                return ExitCode::FAILURE;
            }
        }
    };

    println!(
        "\nDone. processed {} · verified+added {} → {} · needs review {} → {} · \
         skipped {} (unmapped) + {} (no solution)",
        report.processed,
        added,
        out.display(),
        parked,
        review.display(),
        report.skipped_unmapped,
        report.skipped_no_solution,
    );
    ExitCode::SUCCESS
}
