use std::{collections::HashMap, env, error::Error};

use clap::Parser;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

const WRITE_SUPPORTED_LANGUAGES: &[&str] = &[
    "DE", "EN", "EN-GB", "EN-US", "ES", "FR", "IT", "JA", "KO", "PT", "PT-BR", "PT-PT", "ZH",
    "ZH-HANS",
];

const COMMIT_MSG_HOOK_TEMPLATE: &str = include_str!("../templates/commit-msg.sh");

fn languages() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("ACE", "Acehnese"),
        ("AF", "Afrikaans"),
        ("AN", "Aragonese"),
        ("AR", "Arabic"),
        ("AS", "Assamese"),
        ("AY", "Aymara"),
        ("AZ", "Azerbaijani"),
        ("BA", "Bashkir"),
        ("BE", "Belarusian"),
        ("BG", "Bulgarian"),
        ("BHO", "Bhojpuri"),
        ("BN", "Bengali"),
        ("BR", "Breton"),
        ("BS", "Bosnian"),
        ("CA", "Catalan"),
        ("CEB", "Cebuano"),
        ("CKB", "Kurdish (Sorani)"),
        ("CS", "Czech"),
        ("CY", "Welsh"),
        ("DA", "Danish"),
        ("DE", "German"),
        ("DE-CH", "German (Swiss)"),
        ("DE-DE", "German (Germany)"),
        ("EL", "Greek"),
        ("EN", "English (Unspecified variant)"),
        ("EN-GB", "English (British)"),
        ("EN-US", "English (American)"),
        ("EO", "Esperanto"),
        ("ES", "Spanish"),
        ("ES-419", "Spanish (Latin America)"),
        ("ET", "Estonian"),
        ("EU", "Basque"),
        ("FA", "Persian"),
        ("FI", "Finnish"),
        ("FR", "French"),
        ("FR-CA", "French (Canadian)"),
        ("FR-FR", "French (France)"),
        ("GA", "Irish"),
        ("GL", "Galician"),
        ("GN", "Guarani"),
        ("GOM", "Konkani"),
        ("GU", "Gujarati"),
        ("HA", "Hausa"),
        ("HE", "Hebrew"),
        ("HI", "Hindi"),
        ("HR", "Croatian"),
        ("HT", "Haitian Creole"),
        ("HU", "Hungarian"),
        ("HY", "Armenian"),
        ("ID", "Indonesian"),
        ("IG", "Igbo"),
        ("IS", "Icelandic"),
        ("IT", "Italian"),
        ("JA", "Japanese"),
        ("JV", "Javanese"),
        ("KA", "Georgian"),
        ("KK", "Kazakh"),
        ("KMR", "Kurdish (Kurmanji)"),
        ("KO", "Korean"),
        ("KY", "Kyrgyz"),
        ("LA", "Latin"),
        ("LB", "Luxembourgish"),
        ("LMO", "Lombard"),
        ("LN", "Lingala"),
        ("LT", "Lithuanian"),
        ("LV", "Latvian"),
        ("MAI", "Maithili"),
        ("MG", "Malagasy"),
        ("MI", "Maori"),
        ("MK", "Macedonian"),
        ("ML", "Malayalam"),
        ("MN", "Mongolian"),
        ("MR", "Marathi"),
        ("MS", "Malay"),
        ("MT", "Maltese"),
        ("MY", "Burmese"),
        ("NB", "Norwegian (bokmål)"),
        ("NE", "Nepali"),
        ("NL", "Dutch"),
        ("OC", "Occitan"),
        ("OM", "Oromo"),
        ("PA", "Punjabi"),
        ("PAG", "Pangasinan"),
        ("PAM", "Kapampangan"),
        ("PL", "Polish"),
        ("PRS", "Dari"),
        ("PS", "Pashto"),
        ("PT", "Portuguese (all Portuguese varieties mixed)"),
        ("PT-BR", "Portuguese (Brazilian)"),
        ("PT-PT", "Portuguese (European)"),
        ("QU", "Quechua"),
        ("RO", "Romanian"),
        ("RU", "Russian"),
        ("SA", "Sanskrit"),
        ("SCN", "Sicilian"),
        ("SK", "Slovak"),
        ("SL", "Slovenian"),
        ("SQ", "Albanian"),
        ("SR", "Serbian"),
        ("ST", "Sesotho"),
        ("SU", "Sundanese"),
        ("SV", "Swedish"),
        ("SW", "Swahili"),
        ("TA", "Tamil"),
        ("TE", "Telugu"),
        ("TG", "Tajik"),
        ("TH", "Thai"),
        ("TK", "Turkmen"),
        ("TL", "Tagalog"),
        ("TN", "Tswana"),
        ("TR", "Turkish"),
        ("TS", "Tsonga"),
        ("TT", "Tatar"),
        ("UK", "Ukrainian"),
        ("UR", "Urdu"),
        ("UZ", "Uzbek"),
        ("VI", "Vietnamese"),
        ("WO", "Wolof"),
        ("XH", "Xhosa"),
        ("YI", "Yiddish"),
        ("YUE", "Cantonese"),
        ("ZH", "Chinese"),
        ("ZH-HANS", "Chinese (simplified)"),
        ("ZH-HANT", "Chinese (traditional)"),
        ("ZU", "Zulu"),
    ])
}

#[derive(Parser)]
#[command(
    name = "babello",
    version,
    about = "A 'bello' CLI translator via DeepL"
)]
struct Cli {
    /// The text to translate
    #[arg(
        required_unless_present = "usage",
        required_unless_present = "languages",
        required_unless_present = "generate_hook_warn",
        required_unless_present = "generate_hook_block"
    )]
    text: Option<Vec<String>>,

    /// The language to translate from
    #[arg(long)]
    from: Option<String>,

    /// The language to translate to
    #[arg(
        long,
        required_unless_present = "usage",
        required_unless_present = "languages",
        required_unless_present = "improve",
        required_unless_present = "generate_hook_warn",
        required_unless_present = "generate_hook_block"
    )]
    to: Option<String>,

    /// Get the API usage
    #[arg(long)]
    usage: bool,

    /// Get the supported languages list
    #[arg(long)]
    languages: bool,

    /// Improve text by correcting spelling and grammar errors
    #[arg(long)]
    improve: bool,

    /// Print a git commit-msg hook that warns about spelling/grammar issues
    #[arg(long, conflicts_with = "generate_hook_block")]
    generate_hook_warn: bool,

    /// Print a git commit-msg hook that blocks the commit on spelling/grammar issues
    #[arg(long, conflicts_with = "generate_hook_warn")]
    generate_hook_block: bool,

    /// Print the translation/improvement as JSON instead of human-readable text
    #[arg(long)]
    json: bool,
}

/// Response body of the DeepL `/v2/translate` endpoint
#[derive(Deserialize)]
struct DeepLTranslateTextRespone {
    /// One translation per input text, in the same order as the request
    translations: Vec<DeepLTranslation>,
}

/// A single translated text
#[derive(Deserialize)]
struct DeepLTranslation {
    /// The language detected for the source text, when `source_lang` was not specified
    detected_source_language: String,
    /// The translated text
    text: String,
    // /// The number of characters billed for this translation
    // billed_characters: Option<u64>,
    // /// The translation engine used to produce this translation
    // model_type_used: Option<String>,
    // /// The version of tag handling used for this translation
    // tag_handling_version: Option<String>,
}

/// Response body of the DeepL `/v2/write/correct` endpoint
#[derive(Deserialize)]
struct DeepLWriteRephraseRespone {
    /// One improvement per input text, in the same order as the request
    improvements: Vec<DeepLWriteImprovement>,
}

/// A single spelling/grammar-corrected text
#[derive(Deserialize)]
struct DeepLWriteImprovement {
    /// The language detected for the source text
    detected_source_language: String,
    // /// The language the text was corrected/improved in
    // target_language: String,
    /// The corrected text
    text: String,
}

/// Response body of the DeepL `/v2/usage` endpoint
#[derive(Deserialize)]
struct DeepLUsageAndLimits {
    /// The number of characters used in the current billing period
    character_count: usize,
    /// The maximum number of characters available in the current billing period
    character_limit: usize,
}

/// A DeepL API client for a single translation/correction request
struct Babello<'a> {
    /// HTTP client
    client: &'a Client,
    /// DeepL API Key
    api_key: &'a str,
    /// The text/s to translate
    text: Vec<&'a str>,
    /// The language to translate from
    source_lang: Option<&'a str>,
    /// The language to translate to
    target_lang: &'a str,
}

impl Babello<'_> {
    /// Translate text
    fn translate(&self) -> Result<Vec<DeepLTranslation>, Box<dyn Error>> {
        let response = self
            .client
            .post("https://api.deepl.com/v2/translate")
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .header("User-Agent", concat!("babello/", env!("CARGO_PKG_VERSION")))
            .json(&json!({
                "text": self.text,
                "source_lang": format!("{}", self.source_lang.unwrap_or_default()),
                "target_lang": format!("{}", self.target_lang)
            }))
            .send()?;

        let translate_response: DeepLTranslateTextRespone = response.json()?;

        Ok(translate_response.translations)
    }

    /// Improve text by correcting spelling and grammar errors
    fn improve(&self) -> Result<Vec<DeepLWriteImprovement>, Box<dyn Error>> {
        let response = self
            .client
            .post("https://api.deepl.com/v2/write/rephrase")
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .header("User-Agent", concat!("babello/", env!("CARGO_PKG_VERSION")))
            .json(&json!({
                "text": self.text,
                "target_lang": format!("{}", self.target_lang)
            }))
            .send()?;

        let corrections: DeepLWriteRephraseRespone = response.json()?;

        Ok(corrections.improvements)
    }

    /// Retrieve near-real-time character usage and account limits for the current billing period
    fn get_usage_and_limit(&self) -> Result<DeepLUsageAndLimits, Box<dyn Error>> {
        let response = self
            .client
            .get("https://api.deepl.com/v2/usage")
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .send()?;

        let usage_limit: DeepLUsageAndLimits = response.json()?;

        Ok(usage_limit)
    }
}

/// Render the git commit-msg hook template for the given source/target languages
fn generate_commit_msg_hook(to: Option<&str>, block: bool) -> Result<String, Box<dyn Error>> {
    let to = to.unwrap_or("EN-US");

    if !WRITE_SUPPORTED_LANGUAGES.contains(&to) {
        return Err(format!("'{to}' unsupported language for commit-msg correction").into());
    }

    let exit_on_issue = if block { "1" } else { "0" };

    Ok(COMMIT_MSG_HOOK_TEMPLATE
        .replace("__BABELLO_TO__", to)
        .replace("__BABELLO_EXIT_ON_ISSUE__", exit_on_issue))
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let languages = languages();
    let language_from = cli.from.as_deref();
    let language_to = cli.to.as_deref().unwrap_or_default();

    // List the supported languages by DeepL
    if cli.languages {
        let mut sorted_languages: Vec<_> = languages.iter().collect();
        sorted_languages.sort_by_key(|(lang_code, _)| *lang_code);

        for (lang_code, lang) in sorted_languages {
            println!("{lang_code} : {lang}");
        }
        return Ok(());
    }

    // Print a git commit-msg hook based on the user's preferences
    if cli.generate_hook_warn || cli.generate_hook_block {
        let hook = generate_commit_msg_hook(cli.to.as_deref(), cli.generate_hook_block)?;
        print!("{hook}");
        return Ok(());
    }

    // Check if the 'from' language is supported by DeepL
    if !language_from.is_none() && !languages.contains_key(&language_from.unwrap_or_default()) {
        return Err(format!(
            "'{}' unsupported language to translate from",
            language_from.unwrap_or_default()
        )
        .into());
    }

    if cli.improve && !WRITE_SUPPORTED_LANGUAGES.contains(&language_to) {
        return Err(format!("'{language_to}' unsupported language for text improvement").into());
    } else if !cli.usage && !languages.contains_key(language_to) {
        // Check if the 'to' language is supported by DeepL
        return Err(format!("'{language_to}' unsupported language to translate to").into());
    }

    let client = Client::new();

    let api_key = env::var("DEEPL_API_KEY").map_err(|_| "DEEPL_API_KEY is not set".to_string())?;

    let babello = Babello {
        client: &client,
        api_key: &api_key,
        text: cli.text.iter().flatten().map(String::as_str).collect(),
        source_lang: language_from,
        target_lang: language_to,
    };

    if cli.improve {
        let improvements = babello.improve()?;

        if cli.json {
            let results: Vec<_> = babello
                .text
                .iter()
                .zip(improvements.iter())
                .map(|(txt, improvement)| {
                    json!({
                        "source": txt,
                        "text": improvement.text,
                        "detected_source_language": improvement.detected_source_language,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string(&results)?);
            return Ok(());
        }

        for (txt, improvement) in babello.text.iter().zip(improvements.iter()) {
            if babello.target_lang.is_empty() {
                println!(
                    "Detected source language: {}",
                    improvement.detected_source_language
                )
            }
            println!("{} -> {}", txt, improvement.text);
        }

        return Ok(());
    }

    if cli.usage {
        let usage_limit = babello.get_usage_and_limit()?;
        println!(
            "character_count: {} - character_limit: {}",
            usage_limit.character_count, usage_limit.character_limit
        );
        return Ok(());
    }

    let translations = babello.translate()?;

    if cli.json {
        let results: Vec<_> = babello
            .text
            .iter()
            .zip(translations.iter())
            .map(|(txt, translation)| {
                json!({
                    "source": txt,
                    "text": translation.text,
                    "detected_source_language": translation.detected_source_language,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&results)?);
        return Ok(());
    }

    for (txt, translation) in babello.text.iter().zip(translations.iter()) {
        if babello.source_lang.is_none() {
            println!(
                "Detected source language: {}",
                translation.detected_source_language
            )
        }
        println!("{} -> {}", txt, translation.text);
    }

    Ok(())
}

fn main() -> std::process::ExitCode {
    if let Err(e) = run() {
        eprintln!("babello: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
