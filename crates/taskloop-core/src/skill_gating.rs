use std::{collections::BTreeSet, env, path::PathBuf};

use crate::registry::RegisteredSkill;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatingContext {
    pub os: String,
    pub available_bins: Vec<String>,
    pub available_env: Vec<String>,
    pub available_models: Vec<String>,
    pub available_config_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillGatingReport {
    pub enabled: bool,
    pub status: String,
    pub reasons: Vec<String>,
}

impl GatingContext {
    pub fn detect() -> Self {
        let available_env = env::vars().map(|(key, _)| key).collect::<Vec<_>>();
        Self {
            os: env::consts::OS.to_string(),
            available_bins: Vec::new(),
            available_env,
            available_models: Vec::new(),
            available_config_keys: Vec::new(),
        }
    }
}

pub fn evaluate_load_time_gating(
    skill: &RegisteredSkill,
    ctx: &GatingContext,
) -> SkillGatingReport {
    let mut reasons = Vec::new();

    if !skill.requires_os.is_empty()
        && !skill
            .requires_os
            .iter()
            .any(|required| required.eq_ignore_ascii_case(&ctx.os))
    {
        reasons.push(format!(
            "requires os: {} (current: {})",
            skill.requires_os.join(", "),
            ctx.os
        ));
    }

    for bin in &skill.requires_bins {
        if !has_bin(ctx, bin) {
            reasons.push(format!("missing bin: {bin}"));
        }
    }

    if !skill.requires_any_bins.is_empty()
        && !skill.requires_any_bins.iter().any(|bin| has_bin(ctx, bin))
    {
        reasons.push(format!(
            "requires one of bins: {}",
            skill.requires_any_bins.join(", ")
        ));
    }

    let env_keys = ctx.available_env.iter().collect::<BTreeSet<_>>();
    for key in &skill.requires_env {
        if !env_keys.contains(key) {
            reasons.push(format!("missing env: {key}"));
        }
    }

    if !skill.requires_models.is_empty()
        && !skill
            .requires_models
            .iter()
            .any(|model| ctx.available_models.iter().any(|value| value == model))
    {
        reasons.push(format!(
            "requires model: {}",
            skill.requires_models.join(", ")
        ));
    }

    let config_keys = ctx.available_config_keys.iter().collect::<BTreeSet<_>>();
    for key in &skill.requires_config {
        if !config_keys.contains(key) {
            reasons.push(format!("missing config: {key}"));
        }
    }

    SkillGatingReport {
        enabled: reasons.is_empty(),
        status: if reasons.is_empty() {
            "available".into()
        } else {
            "disabled".into()
        },
        reasons,
    }
}

pub fn evaluate_request_time_gating(
    skill: &RegisteredSkill,
    ctx: &GatingContext,
    attachments: &[String],
) -> SkillGatingReport {
    let mut report = evaluate_load_time_gating(skill, ctx);
    if !skill.attachment_types.is_empty()
        && !attachments
            .iter()
            .any(|attachment| skill.attachment_types.iter().any(|value| value == attachment))
    {
        report.reasons.push(format!(
            "requires attachment type: {}",
            skill.attachment_types.join(", ")
        ));
    }
    report.enabled = report.reasons.is_empty();
    report.status = if report.enabled {
        "available".into()
    } else {
        "disabled".into()
    };
    report
}

fn has_bin(ctx: &GatingContext, bin: &str) -> bool {
    ctx.available_bins.iter().any(|value| value == bin) || bin_on_path(bin)
}

fn bin_on_path(bin: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|path| executable_candidate(path, bin).is_file())
}

fn executable_candidate(mut path: PathBuf, bin: &str) -> PathBuf {
    path.push(bin);
    path
}
