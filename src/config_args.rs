//! CLI syntax only; configuration operations live in the library.
use clap::{Args, Subcommand};
use oyzu::config::operations as config;
#[derive(Subcommand)]
pub enum Command {
    Show,
    Get {
        key: String,
    },
    Explain {
        key: Option<String>,
    },
    Profiles,
    Validate {
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        all_profiles: bool,
    },
    Status,
    Refresh,
    Set {
        key: String,
        value: String,
        #[arg(long)]
        json_value: bool,
        #[command(flatten)]
        scope: WriteScope,
    },
    Unset {
        key: String,
        #[command(flatten)]
        scope: WriteScope,
    },
}
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct WriteScope {
    #[arg(long)]
    user: bool,
    #[arg(long)]
    project: bool,
    #[arg(long)]
    local: bool,
}
impl WriteScope {
    fn operation(&self) -> config::WriteScope {
        if self.user {
            config::WriteScope::User
        } else if self.project {
            config::WriteScope::Project
        } else {
            config::WriteScope::Local
        }
    }
}
impl Command {
    pub fn operation(&self) -> config::Command {
        match self {
            Self::Show => config::Command::Show,
            Self::Get { key } => config::Command::Get { key: key.clone() },
            Self::Explain { key } => config::Command::Explain { key: key.clone() },
            Self::Profiles => config::Command::Profiles,
            Self::Validate {
                strict,
                all_profiles,
            } => config::Command::Validate {
                strict: *strict,
                all_profiles: *all_profiles,
            },
            Self::Status => config::Command::Status,
            Self::Refresh => config::Command::Refresh,
            Self::Set {
                key,
                value,
                json_value,
                scope,
            } => config::Command::Set {
                key: key.clone(),
                value: value.clone(),
                json_value: *json_value,
                scope: scope.operation(),
            },
            Self::Unset { key, scope } => config::Command::Unset {
                key: key.clone(),
                scope: scope.operation(),
            },
        }
    }
}
