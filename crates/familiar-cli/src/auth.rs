use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};
use familiar_core::config::FamiliarConfig;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Subcommand, Debug, Clone)]
pub enum AuthCommand {
    /// Create and persist the server token if it does not already exist.
    Init(AuthArgs),
    /// Print the persisted server token. Treat the output as a secret.
    Show(AuthArgs),
}

#[derive(Args, Debug, Clone)]
pub struct AuthArgs {
    /// Explicit Familiar configuration file.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TokenFileResult {
    pub token: String,
    pub generated: bool,
}

pub fn run(command: &AuthCommand) -> Result<()> {
    let args = match command {
        AuthCommand::Init(args) | AuthCommand::Show(args) => args,
    };
    let config = crate::load_config(args.config.as_deref())?;
    let path = token_path(&config)?;

    match command {
        AuthCommand::Init(_) => {
            if path.exists() {
                return Err(anyhow!(
                    "auth token already exists at {}; use `familiar-cli auth show` to print it",
                    path.display()
                ));
            }
            let token = create_token_file(&path)?;
            println!("Familiar auth token initialized at {}", path.display());
            println!("Store this token securely; it will not be printed by `serve`:");
            println!("{token}");
        }
        AuthCommand::Show(_) => {
            let token = read_token(&path)?;
            println!("{token}");
        }
    }
    Ok(())
}

pub fn resolve_token(config: &FamiliarConfig) -> Result<Option<TokenFileResult>> {
    let Some(path) = config.server.auth.token_file.as_deref() else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if path.exists() {
        return Ok(Some(TokenFileResult {
            token: read_token(&path)?,
            generated: false,
        }));
    }
    if !config.server.auth.auto_generate {
        return Ok(None);
    }
    match create_token_file(&path) {
        Ok(token) => Ok(Some(TokenFileResult {
            token,
            generated: true,
        })),
        Err(_error) if path.exists() => Ok(Some(TokenFileResult {
            token: read_token(&path).with_context(|| {
                format!(
                    "read token file after concurrent initialization: {}",
                    path.display()
                )
            })?,
            generated: false,
        })),
        Err(error) => Err(error),
    }
}

fn token_path(config: &FamiliarConfig) -> Result<PathBuf> {
    config
        .server
        .auth
        .token_file
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow!("server.auth.token_file is required; configure a persistent token file first")
        })
}

fn read_token(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("read auth token metadata: {}", path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!(
            "auth token path is not a regular file: {}",
            path.display()
        ));
    }
    let token = fs::read_to_string(path)
        .with_context(|| format!("read auth token file: {}", path.display()))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(anyhow!("auth token file is empty: {}", path.display()));
    }
    Ok(token)
}

fn create_token_file(path: &Path) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create auth token directory: {}", parent.display()))?;
    }
    let token = generate_token();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file: File = options
        .open(path)
        .with_context(|| format!("create auth token file: {}", path.display()))?;
    file.write_all(token.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .with_context(|| format!("write auth token file: {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("flush auth token file: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file
            .metadata()
            .with_context(|| format!("read auth token permissions: {}", path.display()))?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("restrict auth token permissions: {}", path.display()))?;
    }
    Ok(token)
}

fn generate_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::{create_token_file, generate_token, read_token, resolve_token};
    use familiar_core::config::FamiliarConfig;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn generated_token_is_non_empty_and_random_length() {
        let first = generate_token();
        let second = generate_token();
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    #[test]
    fn token_file_is_created_once_and_reused() {
        let dir = std::env::temp_dir().join(format!(
            "familiar-auth-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("auth").join("token");
        let first = create_token_file(&path).unwrap();
        assert_eq!(read_token(&path).unwrap(), first);
        assert!(create_token_file(&path).is_err());
        #[cfg(unix)]
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let mut config = FamiliarConfig::default();
        config.server.auth.token_file = Some(path.to_string_lossy().into_owned());
        config.server.auth.auto_generate = true;
        let resolved = resolve_token(&config).unwrap().unwrap();
        assert_eq!(resolved.token, first);
        assert!(!resolved.generated);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_token_generates_missing_file_when_enabled() {
        let dir = std::env::temp_dir().join(format!(
            "familiar-auth-generate-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("auth").join("token");
        let mut config = FamiliarConfig::default();
        config.server.auth.token_file = Some(path.to_string_lossy().into_owned());
        config.server.auth.auto_generate = true;

        let resolved = resolve_token(&config).unwrap().unwrap();
        assert!(resolved.generated);
        assert_eq!(read_token(&path).unwrap(), resolved.token);

        std::fs::remove_dir_all(dir).unwrap();
    }
}
