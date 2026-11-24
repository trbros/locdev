//! Deploy platforms through their public APIs (`locdev deploy connect,
//! link, status, run, cancel, rollback, logs`; the site's Deploy tab). The
//! token is checked against the platform before it's kept, in the system
//! keyring (`deploy:<platform>`); only the account's name goes to a file.
//! A deploy builds the commit the local repository has pushed: unpushed
//! commits, uncommitted changes or another repository block it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use locdev_core::protocol::{Request, Response};
use locdev_core::remote::{
    DeployGitCheck, DeployLink, RemoteAccount, RemotePlatform, RemoteStatus,
};
use locdev_core::{AppError, ErrorCode};
use locdev_deploy::remote::{self, Client, GitRef};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::Daemon;
use crate::config::KeyStore;
use crate::fsutil::write_atomic;

const KEYRING_SERVICE: &str = "dev.locdev.locdev";

/// What's kept about a connection (no secret here).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Accounts {
    #[serde(default)]
    accounts: BTreeMap<String, RemoteAccount>,
}

impl Daemon {
    fn accounts_file(&self) -> PathBuf {
        self.paths.config_dir.join("deploy-accounts.toml")
    }

    fn tokens_file(&self) -> PathBuf {
        self.paths.config_dir.join("deploy-tokens.toml")
    }

    fn accounts(&self) -> Accounts {
        std::fs::read_to_string(self.accounts_file())
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save_accounts(&self, accounts: &Accounts) -> Result<(), AppError> {
        let raw =
            toml::to_string_pretty(accounts).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.accounts_file(), raw.as_bytes())
    }

    fn token(&self, platform: &str) -> Result<Option<String>, AppError> {
        match self.config.deploy.token_store {
            KeyStore::Keyring => {
                let entry = keyring::Entry::new(KEYRING_SERVICE, &format!("deploy:{platform}"))
                    .map_err(|e| AppError::internal(format!("system keyring: {e}")))?;
                match entry.get_password() {
                    Ok(token) => Ok(Some(token)),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(AppError::internal(format!("system keyring: {e}"))),
                }
            }
            KeyStore::File => Ok(std::fs::read_to_string(self.tokens_file())
                .ok()
                .and_then(|t| toml::from_str::<BTreeMap<String, String>>(&t).ok())
                .and_then(|mut m| m.remove(platform))),
        }
    }

    fn set_token(&self, platform: &str, token: Option<&str>) -> Result<(), AppError> {
        match self.config.deploy.token_store {
            KeyStore::Keyring => {
                let entry = keyring::Entry::new(KEYRING_SERVICE, &format!("deploy:{platform}"))
                    .map_err(|e| AppError::internal(format!("system keyring: {e}")))?;
                let result = match token {
                    Some(token) => entry.set_password(token),
                    None => match entry.delete_credential() {
                        Err(keyring::Error::NoEntry) => Ok(()),
                        other => other,
                    },
                };
                result.map_err(|e| AppError::internal(format!("system keyring: {e}")))
            }
            KeyStore::File => {
                let path = self.tokens_file();
                let mut tokens: BTreeMap<String, String> = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| toml::from_str(&t).ok())
                    .unwrap_or_default();
                match token {
                    Some(token) => tokens.insert(platform.to_string(), token.to_string()),
                    None => tokens.remove(platform),
                };
                let raw =
                    toml::to_string(&tokens).map_err(|e| AppError::internal(e.to_string()))?;
                write_atomic(&path, raw.as_bytes())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
                }
                Ok(())
            }
        }
    }

    /// Another API address from config.toml: https, or http on loopback.
    fn api_base(&self, platform: &str) -> Result<Option<String>, AppError> {
        let base = match platform {
            remote::vercel::ID => self.config.deploy.vercel_api.clone(),
            remote::netlify::ID => self.config.deploy.netlify_api.clone(),
            _ => None,
        };
        if let Some(base) = &base
            && !(base.starts_with("https://")
                || base.starts_with("http://127.0.0.1:")
                || base.starts_with("http://localhost:"))
        {
            return Err(AppError::new(
                ErrorCode::Config,
                format!("[deploy] {platform}_api must be https (or http on loopback): {base}"),
            ));
        }
        Ok(base)
    }

    fn client(&self, platform: &str) -> Result<(Client, String), AppError> {
        let token = self.token(platform)?.ok_or_else(|| {
            AppError::new(
                ErrorCode::Unauthorized,
                format!("{platform} isn't connected; `locdev deploy connect {platform}`"),
            )
        })?;
        let account = self.accounts().accounts.get(platform).cloned();
        let team = account.as_ref().and_then(|a| a.team.clone());
        // Vercel dashboard addresses go under the team, else the user.
        let scope = team
            .clone()
            .or_else(|| account.map(|a| a.user))
            .unwrap_or_default();
        let client = Client::new(platform, token, team, self.api_base(platform)?)
            .map_err(AppError::invalid)?;
        Ok((client, scope))
    }

    /// Runs a blocking API call off the runtime.
    async fn remote_call<T: Send + 'static>(
        &self,
        platform: &str,
        call: impl FnOnce(&Client, &str) -> Result<T, String> + Send + 'static,
    ) -> Result<T, AppError> {
        if self.settings.get().network.offline {
            return Err(AppError::new(ErrorCode::Remote, "offline mode is on"));
        }
        let (client, scope) = self.client(platform)?;
        tokio::task::spawn_blocking(move || call(&client, &scope))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map_err(|e| AppError::new(ErrorCode::Remote, e))
    }

    fn remote_platforms(&self) -> Vec<RemotePlatform> {
        let accounts = self.accounts().accounts;
        remote::platforms()
            .into_iter()
            .map(|mut p| {
                p.account = accounts.get(&p.id).cloned();
                p
            })
            .collect()
    }

    fn remote_platform(id: &str) -> Result<RemotePlatform, AppError> {
        remote::platforms()
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "LocDev doesn't connect to '{id}' yet (available: vercel, netlify)"
                ))
            })
    }

    /// The commit a deploy would build, and what doesn't match the local code.
    async fn deploy_git(&self, site: &str, repo: Option<&str>) -> (DeployGitCheck, Option<GitRef>) {
        let mut check = DeployGitCheck::default();
        let Ok(project) = self.project(site).await else {
            return (check, None);
        };
        let status = match self.with_repo(&project, |r| r.status()).await {
            Ok(status) => status,
            Err(error) => {
                check.problems.push(format!(
                    "not a Git repository LocDev can read: {}",
                    error.message
                ));
                return (check, None);
            }
        };
        check.branch = status.branch.clone();
        check.commit = status.head.clone();
        if status.branch.is_none() {
            check
                .problems
                .push("HEAD is detached; switch to a branch".into());
        }
        if !status.changes.is_empty() {
            check.problems.push(format!(
                "{} uncommitted change(s) wouldn't be deployed; commit and push them first",
                status.changes.len()
            ));
        }
        if status.upstream.is_none() {
            check
                .problems
                .push("the branch isn't on the remote yet; push it first".into());
        } else if status.ahead > 0 {
            check.problems.push(format!(
                "{} local commit(s) aren't pushed; the platform builds what's on GitHub",
                status.ahead
            ));
        }
        match (&status.github, repo) {
            (None, _) => check
                .problems
                .push("the repository has no GitHub remote the platform could build from".into()),
            (Some(gh), Some(repo)) if !gh.slug().eq_ignore_ascii_case(repo) => {
                check.problems.push(format!(
                    "the linked project builds {repo}, but this site's repository is {}",
                    gh.slug()
                ))
            }
            _ => {}
        }
        let git = match (&status.github, &status.branch, &status.head) {
            (Some(gh), Some(branch), Some(sha)) => Some(GitRef {
                owner: gh.owner.clone(),
                repo: gh.name.clone(),
                branch: branch.clone(),
                sha: sha.clone(),
            }),
            _ => None,
        };
        (check, git)
    }

    async fn linked(&self, site: &str) -> Result<DeployLink, AppError> {
        self.project(site).await?.deploy.ok_or_else(|| {
            AppError::new(
                ErrorCode::NotRunnable,
                format!("{site} isn't linked to a deploy project; `locdev deploy link {site}`"),
            )
        })
    }

    async fn remote_status(&self, site: &str) -> Result<RemoteStatus, AppError> {
        let link = self.linked(site).await?;
        let id = link.project_id.clone();
        let (project, deployments, domains, env) = self
            .remote_call(&link.platform, move |c, scope| {
                Ok((
                    c.project(&id, scope)?,
                    c.deployments(&id, scope)?,
                    c.domains(&id).unwrap_or_default(),
                    c.env(&id).unwrap_or_default(),
                ))
            })
            .await?;
        let (git, _) = self.deploy_git(site, project.repo.as_deref()).await;
        let mut platform = Self::remote_platform(&link.platform)?;
        platform.account = self.accounts().accounts.get(&link.platform).cloned();
        let mut notes = Vec::new();
        if !platform.capabilities.logs {
            notes.push(format!(
                "{}'s public API has no build logs; LocDev links to each deploy's page instead.",
                platform.name
            ));
        }
        notes.push("Changing variables isn't done from LocDev yet: their names are listed here, and the dashboard edits them.".into());
        Ok(RemoteStatus {
            link,
            platform,
            project,
            deployments,
            domains,
            env,
            git,
            notes,
        })
    }

    pub(super) async fn remote_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::DeployPlatforms => Ok(Response::DeployPlatforms {
                platforms: self.remote_platforms(),
            }),
            Request::DeployConnect {
                platform,
                token,
                team,
            } => {
                Self::remote_platform(&platform)?;
                let token = token.trim().to_string();
                if token.is_empty() || token.len() > 512 || token.chars().any(char::is_whitespace) {
                    return Err(AppError::invalid("that doesn't look like a token"));
                }
                if self.settings.get().network.offline {
                    return Err(AppError::new(ErrorCode::Remote, "offline mode is on"));
                }
                let team = team.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
                let base = self.api_base(&platform)?;
                let (p, t, tm) = (platform.clone(), token.clone(), team.clone());
                // Checked against the platform before anything is kept.
                let account =
                    tokio::task::spawn_blocking(move || Client::new(&p, t, tm, base)?.account())
                        .await
                        .map_err(|e| AppError::internal(e.to_string()))?
                        .map_err(|e| AppError::new(ErrorCode::Unauthorized, e))?;
                self.set_token(&platform, Some(&token))?;
                let mut accounts = self.accounts();
                accounts.accounts.insert(platform.clone(), account);
                self.save_accounts(&accounts)?;
                info!(%platform, "deploy platform connected");
                Ok(Response::DeployPlatforms {
                    platforms: self.remote_platforms(),
                })
            }
            Request::DeployDisconnect { platform } => {
                Self::remote_platform(&platform)?;
                self.set_token(&platform, None)?;
                let mut accounts = self.accounts();
                accounts.accounts.remove(&platform);
                self.save_accounts(&accounts)?;
                info!(%platform, "deploy platform disconnected");
                Ok(Response::DeployPlatforms {
                    platforms: self.remote_platforms(),
                })
            }
            Request::DeployRemoteProjects { platform } => {
                Self::remote_platform(&platform)?;
                let projects = self
                    .remote_call(&platform, |c, scope| c.projects(scope))
                    .await?;
                Ok(Response::RemoteProjects { projects })
            }
            Request::DeployLinkSite {
                site,
                platform,
                project,
            } => {
                let link = match (platform, project) {
                    (Some(platform), Some(id)) => {
                        Self::remote_platform(&platform)?;
                        let lookup = id.clone();
                        let remote = self
                            .remote_call(&platform, move |c, scope| c.project(&lookup, scope))
                            .await?;
                        Some(DeployLink {
                            platform,
                            project_id: remote.id,
                            project_name: remote.name,
                        })
                    }
                    (None, None) => None,
                    _ => {
                        return Err(AppError::invalid(
                            "give both the platform and the project, or neither to unlink",
                        ));
                    }
                };
                let mut registry = self.registry.lock().await;
                let entry = registry.get_mut(&site)?;
                entry.deploy = link;
                let project = entry.clone();
                self.commit(&registry)?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }
            Request::DeployRemoteStatus { site } => {
                Ok(Response::RemoteStatus(self.remote_status(&site).await?))
            }
            Request::DeployRun { site, production } => {
                let link = self.linked(&site).await?;
                let id = link.project_id.clone();
                let project = self
                    .remote_call(&link.platform, move |c, scope| c.project(&id, scope))
                    .await?;
                let (check, git) = self.deploy_git(&site, project.repo.as_deref()).await;
                if !check.problems.is_empty() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!("not deploying: {}", check.problems.join("; ")),
                    ));
                }
                let git = git.ok_or_else(|| AppError::invalid("no commit to deploy"))?;
                if production
                    && let Some(branch) = &project.branch
                    && *branch != git.branch
                {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!(
                            "production builds {branch}, but this site is on {}; switch branches or deploy a preview",
                            git.branch
                        ),
                    ));
                }
                let deployment = self
                    .remote_call(&link.platform, move |c, scope| {
                        c.deploy(&project, &git, production, scope)
                    })
                    .await?;
                info!(%site, platform = %link.platform, production, "deploy started");
                Ok(Response::RemoteDeployment { deployment })
            }
            Request::DeployCancel { site, deployment } => {
                let link = self.linked(&site).await?;
                self.remote_call(&link.platform, move |c, _| c.cancel(&deployment))
                    .await?;
                Ok(Response::Ok)
            }
            Request::DeployRollback { site, deployment } => {
                let link = self.linked(&site).await?;
                let id = link.project_id.clone();
                self.remote_call(&link.platform, move |c, _| c.rollback(&id, &deployment))
                    .await?;
                warn!(%site, platform = %link.platform, "production rolled back");
                Ok(Response::Ok)
            }
            Request::DeployLogs { site, deployment } => {
                let link = self.linked(&site).await?;
                let id = link.project_id.clone();
                let log = self
                    .remote_call(&link.platform, move |c, scope| {
                        let project = c.project(&id, scope)?;
                        c.logs(&project, &deployment)
                    })
                    .await?;
                Ok(Response::RemoteLog(log))
            }
            _ => unreachable!("not a deploy platform request"),
        }
    }
}
