use clap::{Parser, Subcommand};
use colored::Colorize;
use imagegen_kit::auth;
use imagegen_kit::error::{anyhow, Result};
use imagegen_kit::provider::ProviderType;
use imagegen_kit::provider::{create_provider, supported_providers, EditRequest, GenerateRequest};
use imagegen_kit::utils::{default_output_dir, ensure_dir_exists, parse_size, write_json_pretty};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
enum ExitCode {
    Success = 0,
    GeneralFailure = 1,
    UsageError = 2,
    ResourceNotFound = 3,
    PermissionDenied = 4,
    Conflict = 5,
}

#[derive(Serialize)]
struct ErrorJson {
    success: bool,
    error_code: i32,
    error_type: String,
    message: String,
    suggestion: Option<String>,
}

#[derive(Serialize)]
struct DryRunResultJson {
    dry_run: bool,
    command: String,
    provider: String,
    model: Option<String>,
    prompt: String,
    input: Option<String>,
    output_dir: String,
    size: String,
    count: u32,
    quality: Option<String>,
    output_format: Option<String>,
    output_compression: Option<u8>,
    background: Option<String>,
    api_key_source: Option<String>,
    would_create: Vec<String>,
}

#[derive(Serialize)]
struct CommandResultJson {
    success: bool,
    provider: String,
    model: Option<String>,
    artifacts: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "imagegen-kit")]
#[command(about = "Image generation CLI for ZenMux providers")]
#[command(version)]
#[command(after_help = "\
EXAMPLES:
    # Generate through ZenMux OpenAI Images protocol
    imagegen-kit generate \"a clean product photo of a ceramic mug\" --provider zenmux/openai --model gpt-image-2

    # Generate through ZenMux Google Gemini / Vertex AI protocol
    imagegen-kit generate \"a nano banana dish in a fancy restaurant\" --provider zenmux/google --model google/gemini-3-pro-image-preview

    # Preview a request without calling ZenMux
    imagegen-kit generate \"a clean product photo of a ceramic mug\" --dry-run --json

    # Edit an image through ZenMux
    imagegen-kit edit ./input.png \"replace the background with a studio backdrop\" --provider zenmux/openai

    # Store a provider API key securely
    imagegen-kit login --provider zenmux/openai

    # List configured credentials
    imagegen-kit login --list
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn wants_json(&self) -> bool {
        match &self.command {
            Commands::Generate { json, .. }
            | Commands::Edit { json, .. }
            | Commands::Login { json, .. }
            | Commands::Providers { json } => *json,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    Generate {
        #[arg(value_name = "PROMPT")]
        prompt: String,

        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output_dir: Option<PathBuf>,

        #[arg(long, value_name = "NEGATIVE_PROMPT")]
        negative_prompt: Option<String>,

        #[arg(long, default_value = "1024x1024", value_name = "WIDTHxHEIGHT")]
        size: String,

        #[arg(long, value_name = "QUALITY")]
        quality: Option<String>,

        #[arg(long, value_name = "FORMAT")]
        output_format: Option<String>,

        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100), value_name = "0-100")]
        output_compression: Option<u8>,

        #[arg(long, value_name = "BACKGROUND")]
        background: Option<String>,

        #[arg(short = 'n', long, default_value_t = 1, value_name = "COUNT")]
        count: u32,

        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        #[arg(long, value_name = "MODEL")]
        model: Option<String>,

        #[arg(short = 'k', long, value_name = "API_KEY")]
        api_key: Option<String>,

        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        quiet: bool,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        overwrite: bool,
    },

    Edit {
        #[arg(value_name = "INPUT_IMAGE")]
        input: PathBuf,

        #[arg(value_name = "PROMPT")]
        prompt: String,

        #[arg(long, value_name = "MASK_IMAGE")]
        mask: Option<PathBuf>,

        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output_dir: Option<PathBuf>,

        #[arg(long, default_value = "1024x1024", value_name = "WIDTHxHEIGHT")]
        size: String,

        #[arg(long, value_name = "QUALITY")]
        quality: Option<String>,

        #[arg(long, value_name = "FORMAT")]
        output_format: Option<String>,

        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100), value_name = "0-100")]
        output_compression: Option<u8>,

        #[arg(long, value_name = "BACKGROUND")]
        background: Option<String>,

        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        #[arg(long, value_name = "MODEL")]
        model: Option<String>,

        #[arg(short = 'k', long, value_name = "API_KEY")]
        api_key: Option<String>,

        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        quiet: bool,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        overwrite: bool,
    },

    Login {
        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        #[arg(long, value_name = "API_KEY")]
        api_key: Option<String>,

        #[arg(long)]
        list: bool,

        #[arg(long, value_name = "PROVIDER")]
        delete: Option<String>,

        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        quiet: bool,
    },

    Providers {
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json_errors = cli.wants_json();

    let exit_code = match run(cli).await {
        Ok(()) => ExitCode::Success,
        Err(error) => {
            if json_errors {
                let _ = write_json_pretty(&ErrorJson {
                    success: false,
                    error_code: ExitCode::GeneralFailure as i32,
                    error_type: "general_failure".to_string(),
                    message: error.to_string(),
                    suggestion: Some(
                        "Check your provider, model, ZENMUX_API_KEY, and output path.".to_string(),
                    ),
                });
            } else {
                eprintln!("{} {}", "Error:".red().bold(), error);
            }
            ExitCode::GeneralFailure
        }
    };

    std::process::exit(exit_code as i32);
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Generate {
            prompt,
            output_dir,
            negative_prompt,
            size,
            quality,
            output_format,
            output_compression,
            background,
            count,
            provider,
            model,
            api_key,
            json,
            quiet,
            dry_run,
            overwrite,
        } => {
            run_generate(
                prompt,
                output_dir,
                negative_prompt,
                size,
                quality,
                output_format,
                output_compression,
                background,
                count,
                provider,
                model,
                api_key,
                json,
                quiet,
                dry_run,
                overwrite,
            )
            .await
        }
        Commands::Edit {
            input,
            prompt,
            mask,
            output_dir,
            size,
            quality,
            output_format,
            output_compression,
            background,
            provider,
            model,
            api_key,
            json,
            quiet,
            dry_run,
            overwrite,
        } => {
            run_edit(
                input,
                prompt,
                mask,
                output_dir,
                size,
                quality,
                output_format,
                output_compression,
                background,
                provider,
                model,
                api_key,
                json,
                quiet,
                dry_run,
                overwrite,
            )
            .await
        }
        Commands::Login { provider, api_key, list, delete, json, quiet } => {
            run_login(provider, api_key, list, delete, json, quiet)
        }
        Commands::Providers { json } => run_providers(json),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_generate(
    prompt: String,
    output_dir: Option<PathBuf>,
    negative_prompt: Option<String>,
    size: String,
    quality: Option<String>,
    output_format: Option<String>,
    output_compression: Option<u8>,
    background: Option<String>,
    count: u32,
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    json: bool,
    quiet: bool,
    dry_run: bool,
    overwrite: bool,
) -> Result<()> {
    if count == 0 {
        return Err(anyhow!("COUNT must be greater than zero"));
    }
    parse_size(&size)?;

    let output_dir = output_dir.unwrap_or(default_output_dir()?);
    let provider_type = parse_provider(provider.as_deref())?;
    let (api_key, api_key_source) = resolve_api_key(&provider_type, api_key)?;
    let extension = output_extension(output_format.as_deref());

    let would_create = (1..=count)
        .map(|index| {
            output_dir.join(format!("image-{}.{}", index, extension)).display().to_string()
        })
        .collect::<Vec<_>>();

    if dry_run {
        return print_dry_run(
            DryRunResultJson {
                dry_run,
                command: "generate".to_string(),
                provider: provider_type.as_str().to_string(),
                model,
                prompt,
                input: None,
                output_dir: output_dir.display().to_string(),
                size,
                count,
                quality,
                output_format,
                output_compression,
                background,
                api_key_source,
                would_create,
            },
            json,
            quiet,
        );
    }

    ensure_dir_exists(&output_dir)?;
    let progress = progress_bar(quiet || json);
    let provider = create_provider(provider_type.clone(), api_key);
    let request = GenerateRequest {
        prompt,
        negative_prompt,
        model,
        size,
        count,
        quality,
        output_format,
        output_compression,
        background,
        output_dir,
        overwrite,
    };
    let result = provider
        .generate(
            request,
            Box::new(move |update| {
                progress.set_message(update.message);
                progress.inc(1);
            }),
        )
        .await?;

    print_command_result(result.provider, result.model, result.artifacts, json, quiet)
}

#[allow(clippy::too_many_arguments)]
async fn run_edit(
    input: PathBuf,
    prompt: String,
    mask: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    size: String,
    quality: Option<String>,
    output_format: Option<String>,
    output_compression: Option<u8>,
    background: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    json: bool,
    quiet: bool,
    dry_run: bool,
    overwrite: bool,
) -> Result<()> {
    parse_size(&size)?;
    if !input.exists() {
        return Err(anyhow!("Input image not found: {}", input.display()));
    }
    if let Some(mask) = &mask {
        if !mask.exists() {
            return Err(anyhow!("Mask image not found: {}", mask.display()));
        }
    }

    let output_dir = output_dir.unwrap_or(default_output_dir()?);
    let provider_type = parse_provider(provider.as_deref())?;
    let (api_key, api_key_source) = resolve_api_key(&provider_type, api_key)?;
    let extension = output_extension(output_format.as_deref());
    let would_create =
        vec![output_dir.join(format!("edited-image-1.{}", extension)).display().to_string()];

    if dry_run {
        return print_dry_run(
            DryRunResultJson {
                dry_run,
                command: "edit".to_string(),
                provider: provider_type.as_str().to_string(),
                model,
                prompt,
                input: Some(input.display().to_string()),
                output_dir: output_dir.display().to_string(),
                size,
                count: 1,
                quality,
                output_format,
                output_compression,
                background,
                api_key_source,
                would_create,
            },
            json,
            quiet,
        );
    }

    ensure_dir_exists(&output_dir)?;
    let progress = progress_bar(quiet || json);
    let provider = create_provider(provider_type, api_key);
    let request = EditRequest {
        input,
        mask,
        prompt,
        model,
        size,
        quality,
        output_format,
        output_compression,
        background,
        output_dir,
        overwrite,
    };
    let result = provider
        .edit(
            request,
            Box::new(move |update| {
                progress.set_message(update.message);
                progress.inc(1);
            }),
        )
        .await?;

    print_command_result(result.provider, result.model, result.artifacts, json, quiet)
}

fn run_login(
    provider: Option<String>,
    api_key: Option<String>,
    list: bool,
    delete: Option<String>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if list {
        let providers = auth::list_credentials()?;
        if json {
            return write_json_pretty(&json!({ "providers": providers }));
        }
        if providers.is_empty() {
            if !quiet {
                println!("No credentials stored");
            }
        } else {
            for provider in providers {
                println!("{}", provider);
            }
        }
        return Ok(());
    }

    if let Some(provider) = delete {
        let key = parse_provider(Some(&provider))
            .map(|provider_type| auth::provider_key(provider_type.as_str()).to_string())
            .unwrap_or_else(|_| auth::provider_key(&provider).to_string());
        auth::delete_credential(&key)?;
        if json {
            return write_json_pretty(&json!({ "success": true, "deleted": key }));
        }
        if !quiet {
            println!("{} {}", "Deleted credential for".green(), key);
        }
        return Ok(());
    }

    let provider = provider.ok_or_else(|| anyhow!("Specify --provider, --list, or --delete"))?;
    let provider_type = parse_provider(Some(&provider))?;
    let key = auth::provider_key(provider_type.as_str());
    let api_key = match api_key {
        Some(value) => value,
        None => rpassword::prompt_password(format!(
            "Enter API key for {}: ",
            provider_type.display_name()
        ))?,
    };

    auth::set_credential(key, &api_key)?;
    if json {
        write_json_pretty(&json!({ "success": true, "provider": key }))
    } else {
        if !quiet {
            println!("{} {}", "Stored credential for".green(), key);
        }
        Ok(())
    }
}

fn run_providers(json: bool) -> Result<()> {
    let providers = supported_providers()
        .into_iter()
        .map(|provider| {
            json!({
                "id": provider.as_str(),
                "name": provider.display_name(),
                "protocol": provider.protocol(),
                "env_var": provider.env_var(),
                "status": "implemented",
            })
        })
        .collect::<Vec<_>>();

    if json {
        write_json_pretty(&json!({ "providers": providers }))
    } else {
        for provider in providers {
            println!(
                "{}\t{}\t{}\t{}",
                provider["id"].as_str().unwrap_or_default(),
                provider["name"].as_str().unwrap_or_default(),
                provider["protocol"].as_str().unwrap_or_default(),
                provider["status"].as_str().unwrap_or_default()
            );
        }
        Ok(())
    }
}

fn parse_provider(provider: Option<&str>) -> Result<ProviderType> {
    provider
        .map(ProviderType::from_str)
        .transpose()
        .map_err(|_| anyhow!("Unsupported provider. Run 'imagegen-kit providers' to list options"))?
        .or(Some(ProviderType::ZenmuxOpenAi))
        .ok_or_else(|| anyhow!("Failed to resolve provider"))
}

fn resolve_api_key(
    provider_type: &ProviderType,
    explicit_api_key: Option<String>,
) -> Result<(Option<String>, Option<String>)> {
    if let Some(api_key) = explicit_api_key {
        return Ok((Some(api_key), Some("cli".to_string())));
    }

    if let Ok(api_key) = std::env::var(provider_type.env_var()) {
        if !api_key.is_empty() {
            return Ok((Some(api_key), Some(provider_type.env_var().to_string())));
        }
    }

    let key = auth::provider_key(provider_type.as_str());
    if let Some(api_key) = auth::get_credential(key)? {
        return Ok((Some(api_key), Some("stored".to_string())));
    }

    Ok((None, None))
}

fn progress_bar(hidden: bool) -> ProgressBar {
    if hidden {
        return ProgressBar::hidden();
    }

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    progress
}

fn output_extension(output_format: Option<&str>) -> &'static str {
    match output_format.unwrap_or("png").trim().to_lowercase().as_str() {
        "jpg" | "jpeg" | "image/jpg" | "image/jpeg" => "jpg",
        "webp" | "image/webp" => "webp",
        _ => "png",
    }
}

fn print_dry_run(result: DryRunResultJson, json: bool, quiet: bool) -> Result<()> {
    if json {
        return write_json_pretty(&result);
    }

    if quiet {
        println!("{}", result.output_dir);
        return Ok(());
    }

    println!("{}", "Dry run".green().bold());
    println!("{} {}", "Command:".bold(), result.command);
    println!("{} {}", "Provider:".bold(), result.provider);
    if let Some(model) = &result.model {
        println!("{} {}", "Model:".bold(), model);
    }
    println!("{} {}", "Output directory:".bold(), result.output_dir);
    println!("{} {}", "Size:".bold(), result.size);
    println!("{} {}", "Count:".bold(), result.count);
    if let Some(quality) = &result.quality {
        println!("{} {}", "Quality:".bold(), quality);
    }
    if let Some(output_format) = &result.output_format {
        println!("{} {}", "Output format:".bold(), output_format);
    }
    if let Some(output_compression) = result.output_compression {
        println!("{} {}", "Output compression:".bold(), output_compression);
    }
    if let Some(background) = &result.background {
        println!("{} {}", "Background:".bold(), background);
    }
    println!("{} {}", "API key:".bold(), result.api_key_source.as_deref().unwrap_or("none"));
    println!("{}", "Would create:".bold());
    for path in result.would_create {
        println!("  {}", path);
    }
    Ok(())
}

fn print_command_result(
    provider: String,
    model: Option<String>,
    artifacts: Vec<imagegen_kit::ImageArtifact>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let artifact_paths =
        artifacts.iter().map(|artifact| artifact.path.display().to_string()).collect::<Vec<_>>();

    if json {
        return write_json_pretty(&CommandResultJson {
            success: true,
            provider,
            model,
            artifacts: artifact_paths,
        });
    }

    if quiet {
        for path in artifact_paths {
            println!("{}", path);
        }
        return Ok(());
    }

    println!("{}", "Completed".green().bold());
    for artifact in artifacts {
        println!("{}", artifact.path.display());
    }
    Ok(())
}
