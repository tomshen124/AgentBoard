---
kind: skill
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
name: Example Skill
description: Demonstrates the directory-based SKILL.md contract.
mode: both
safe_for_background: false
requires_tools:
  - exec
  - filesystem
requires_env: []
requires_models: []
requires_os: []
requires_bins: []
requires_any_bins: []
requires_config: []
attachment_types: []
modes:
  - mixed
tags:
  - example
---

# Example Skill

This skill exists to prove the workspace discovery layout:

- `skills/<name>/SKILL.md`
- frontmatter for machine-readable metadata
- human-readable body for workflow guidance
