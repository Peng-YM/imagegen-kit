use clap::{Parser, Subcommand};
use colored::Colorize;
use imagegen_kit::auth;
use imagegen_kit::error::{anyhow, Result};
use imagegen_kit::models::{self, ModelEntry, ModelOperation};
use imagegen_kit::provider::ProviderType;
use imagegen_kit::provider::{create_provider, supported_providers, EditRequest, GenerateRequest};
use imagegen_kit::utils::{default_output_dir, ensure_dir_exists, parse_size, write_json_pretty};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use serde_json::json;
use std::io::{self, Write};
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

#[derive(Serialize)]
struct ProviderInfoJson {
    id: String,
    name: String,
    protocol: String,
    env_var: String,
    default_generate_model: Option<String>,
    default_edit_model: Option<String>,
    credential_stored: bool,
    models: Vec<ModelInfoJson>,
    status: String,
}

#[derive(Serialize)]
struct ModelInfoJson {
    id: String,
    name: String,
    description: Option<String>,
    api_model: String,
    aliases: Vec<String>,
    protocols: Vec<String>,
    source_protocols: Vec<String>,
    google_method: Option<models::GoogleMethod>,
    supports_generate: bool,
    supports_edit: bool,
    default_generate: bool,
    default_edit: bool,
    note: Option<String>,
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

    # Generate through ZenMux Google Imagen protocol
    imagegen-kit generate \"a clean product render\" --provider zenmux/google --model qwen/qwen-image-2.0

    # Preview a request without calling ZenMux
    imagegen-kit generate \"a clean product photo of a ceramic mug\" --dry-run --json

    # Edit an image through ZenMux
    imagegen-kit edit ./input.png \"replace the background with a studio backdrop\" --provider zenmux/openai

    # List providers and model metadata
    imagegen-kit provider --list

    # Store a provider API key securely
    imagegen-kit provider --login

    # Show logged-in providers and their default models
    imagegen-kit status
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
            | Commands::Status { json, .. }
            | Commands::Provider { json, .. } => *json,
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

    Status {
        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        quiet: bool,
    },

    Provider {
        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        #[arg(long, value_name = "API_KEY")]
        api_key: Option<String>,

        #[arg(long)]
        list: bool,

        #[arg(long)]
        login: bool,

        #[arg(long)]
        logout: bool,

        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        quiet: bool,
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
        Commands::Status { json, quiet } => run_status(json, quiet),
        Commands::Provider { provider, api_key, list, login, logout, json, quiet } => {
            run_provider(provider, api_key, list, login, logout, json, quiet)
        }
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
    let model_entry =
        resolve_provider_model(&provider_type, model.as_deref(), ModelOperation::Generate)?;
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
                model: Some(model_entry.id.clone()),
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
        model: Some(model_entry.api_model),
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
    let model_entry =
        resolve_provider_model(&provider_type, model.as_deref(), ModelOperation::Edit)?;
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
                model: Some(model_entry.id.clone()),
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
        model: Some(model_entry.api_model),
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

fn run_provider(
    provider: Option<String>,
    api_key: Option<String>,
    list: bool,
    login: bool,
    logout: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    if api_key.is_some() && !login {
        return Err(anyhow!("Use --login with --api-key"));
    }

    let action_count = usize::from(list) + usize::from(login) + usize::from(logout);
    if action_count > 1 {
        return Err(anyhow!("Choose only one provider action: --list, --login, or --logout"));
    }

    if logout {
        return logout_provider(provider, json, quiet);
    }

    if login {
        return login_provider(provider, api_key, json, quiet);
    }

    list_provider_catalog(provider.as_deref(), json, quiet)
}

fn run_status(json: bool, quiet: bool) -> Result<()> {
    let providers = supported_providers()
        .into_iter()
        .filter_map(|provider| match provider_logged_in(&provider) {
            Ok(true) => Some(provider_info(&provider)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>>>()?;

    if json {
        return write_json_pretty(&json!({ "providers": providers }));
    }

    if providers.is_empty() {
        if !quiet {
            println!("No providers logged in");
        }
        return Ok(());
    }

    for provider in &providers {
        print_provider_info(provider, quiet);
    }
    Ok(())
}

fn login_provider(
    provider: Option<String>,
    api_key: Option<String>,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let provider_type = match provider {
        Some(provider) => parse_provider(Some(&provider))?,
        None => select_provider_interactively()?,
    };
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

fn logout_provider(provider: Option<String>, json: bool, quiet: bool) -> Result<()> {
    let provider_type = match provider {
        Some(provider) => parse_provider(Some(&provider))?,
        None => select_provider_interactively()?,
    };
    let key = auth::provider_key(provider_type.as_str());
    auth::delete_credential(key)?;
    if json {
        return write_json_pretty(&json!({ "success": true, "logged_out": key }));
    }
    if !quiet {
        println!("{} {}", "Logged out from".green(), key);
    }
    Ok(())
}

fn list_provider_catalog(provider: Option<&str>, json: bool, quiet: bool) -> Result<()> {
    let provider_types = match provider {
        Some(provider) => vec![parse_provider(Some(provider))?],
        None => supported_providers(),
    };

    let providers = provider_types.iter().map(provider_info).collect::<Result<Vec<_>>>()?;

    if json {
        write_json_pretty(&json!({ "providers": providers }))
    } else {
        for provider in &providers {
            print_provider_info(provider, quiet);
        }
        Ok(())
    }
}

fn provider_info(provider: &ProviderType) -> Result<ProviderInfoJson> {
    let default_generate_model =
        models::default_model(provider.as_str(), ModelOperation::Generate)?
            .map(|model| model.id.clone());
    let default_edit_model = models::default_model(provider.as_str(), ModelOperation::Edit)?
        .map(|model| model.id.clone());
    let credential_stored = provider_logged_in(provider)?;
    let models = models::models_for_provider(provider.as_str())
        .into_iter()
        .map(|model| ModelInfoJson {
            id: model.id.clone(),
            name: model.name.clone(),
            description: model.description.clone(),
            api_model: model.api_model.clone(),
            aliases: model.aliases.clone(),
            protocols: model.protocols.clone(),
            source_protocols: model.source_protocols.clone(),
            google_method: model.google_method,
            supports_generate: model.supports_generate,
            supports_edit: model.supports_edit,
            default_generate: default_generate_model.as_deref() == Some(model.id.as_str()),
            default_edit: default_edit_model.as_deref() == Some(model.id.as_str()),
            note: model.note.clone(),
        })
        .collect::<Vec<_>>();

    Ok(ProviderInfoJson {
        id: provider.as_str().to_string(),
        name: provider.display_name().to_string(),
        protocol: provider.protocol().to_string(),
        env_var: provider.env_var().to_string(),
        default_generate_model,
        default_edit_model,
        credential_stored,
        models,
        status: "implemented".to_string(),
    })
}

fn provider_logged_in(provider: &ProviderType) -> Result<bool> {
    auth::get_credential(auth::provider_key(provider.as_str()))
        .map(|credential| credential.is_some())
}

fn print_provider_info(provider: &ProviderInfoJson, quiet: bool) {
    if quiet {
        println!("{}", provider.id);
        for model in &provider.models {
            println!("{}", model.id);
        }
        return;
    }

    println!("{} {}", provider.id.bold(), provider.name);
    println!("{} {}", "Protocol:".bold(), provider.protocol);
    println!(
        "{} {} ({})",
        "Auth:".bold(),
        provider.env_var,
        if provider.credential_stored { "stored" } else { "not stored" }
    );
    println!(
        "{} {}",
        "Default generate:".bold(),
        provider.default_generate_model.as_deref().unwrap_or("none")
    );
    println!(
        "{} {}",
        "Default edit:".bold(),
        provider.default_edit_model.as_deref().unwrap_or("none")
    );
    println!("{}", "Models:".bold());
    for model in &provider.models {
        let default_marker = model_default_marker(model);
        println!("  - {}{}", model.id.bold(), default_marker);
        println!("    Name: {}", model.name);
        if let Some(description) = &model.description {
            println!("    Description: {}", description);
        }
        println!("    Protocols: {}", model.protocols.join(", "));
        println!("    Capabilities: {}", model_capabilities(model).join(", "));
        if let Some(method) = model.google_method {
            println!("    Google method: {}", google_method_label(method));
        }
    }
    println!();
}

fn model_default_marker(model: &ModelInfoJson) -> &'static str {
    match (model.default_generate, model.default_edit) {
        (true, true) => " (default generate/edit)",
        (true, false) => " (default generate)",
        (false, true) => " (default edit)",
        (false, false) => "",
    }
}

fn model_capabilities(model: &ModelInfoJson) -> Vec<&'static str> {
    let mut capabilities = Vec::new();
    if model.supports_generate {
        capabilities.push("generate");
    }
    if model.supports_edit {
        capabilities.push("edit");
    }
    capabilities
}

fn google_method_label(method: models::GoogleMethod) -> &'static str {
    match method {
        models::GoogleMethod::GenerateContent => "generateContent",
        models::GoogleMethod::Predict => "predict",
    }
}

fn select_provider_interactively() -> Result<ProviderType> {
    let providers = supported_providers();
    println!("{}", "Select provider:".bold());
    for (index, provider) in providers.iter().enumerate() {
        println!("  {}) {} ({})", index + 1, provider.display_name(), provider.as_str());
    }
    print!("Provider [1]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        return providers.first().cloned().ok_or_else(|| anyhow!("No providers configured"));
    }

    if let Ok(index) = input.parse::<usize>() {
        return providers
            .get(index.saturating_sub(1))
            .cloned()
            .ok_or_else(|| anyhow!("Provider selection out of range"));
    }

    parse_provider(Some(input))
}

fn parse_provider(provider: Option<&str>) -> Result<ProviderType> {
    provider
        .map(ProviderType::from_str)
        .transpose()
        .map_err(|_| {
            anyhow!("Unsupported provider. Run 'imagegen-kit provider --list' to list options")
        })?
        .or(Some(ProviderType::ZenmuxOpenAi))
        .ok_or_else(|| anyhow!("Failed to resolve provider"))
}

fn resolve_provider_model(
    provider_type: &ProviderType,
    model: Option<&str>,
    operation: ModelOperation,
) -> Result<ModelEntry> {
    models::resolve_model(provider_type.as_str(), model, operation)
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

#[cfg(test)]
mod tests {
    use super::{resolve_provider_model, ModelOperation, ProviderType};

    #[test]
    fn rejects_openai_models_on_google_provider() {
        let result = resolve_provider_model(
            &ProviderType::ZenmuxGoogle,
            Some("openai/gpt-image-2"),
            ModelOperation::Generate,
        );
        assert!(result.is_err());
    }

    #[test]
    fn accepts_google_models_on_google_provider() {
        let result = resolve_provider_model(
            &ProviderType::ZenmuxGoogle,
            Some("google/gemini-3-pro-image-preview"),
            ModelOperation::Generate,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_imagen_models_on_google_provider() {
        let result = resolve_provider_model(
            &ProviderType::ZenmuxGoogle,
            Some("qwen/qwen-image-2.0"),
            ModelOperation::Generate,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_openai_alias_on_openai_provider() {
        let result = resolve_provider_model(
            &ProviderType::ZenmuxOpenAi,
            Some("gpt-image-2"),
            ModelOperation::Generate,
        )
        .unwrap();
        assert_eq!(result.id, "openai/gpt-image-2");
        assert_eq!(result.api_model, "gpt-image-2");
    }

    #[test]
    fn rejects_google_provider_for_edits() {
        let result =
            resolve_provider_model(&ProviderType::ZenmuxGoogle, None, ModelOperation::Edit);
        assert!(result.is_err());
    }
}
