use crate::policy::context::{PolicyContext, SecurityMode};
use crate::policy::types::{
    ActionRequest, ActionTarget, PolicyConstraints, PolicyOutcome, RiskLevel,
};
use crate::security::CommandPolicy;
use std::path::PathBuf;

/// Evaluates command-line and filesystem execution requests.
pub struct CommandAdapter;

impl CommandAdapter {
    pub fn evaluate(
        req: &ActionRequest,
        ctx: &PolicyContext,
        constraints: &PolicyConstraints,
    ) -> (RiskLevel, PolicyOutcome, String) {
        // 1. Check if execution is globally disabled
        if !constraints.allow_command_execution {
            return (
                RiskLevel::High,
                PolicyOutcome::Blocked,
                "Command execution is globally disabled in policy configuration.".to_string(),
            );
        }

        // 2. Extract program and arguments based on target or arguments JSON
        let (program, args) = match &req.target {
            ActionTarget::Command { program, args, .. } => (program.clone(), args.clone()),
            _ => {
                // Fallback: inspect arguments JSON
                if let Some(cmd_val) = req.arguments.get("command").and_then(|v| v.as_str()) {
                    match CommandPolicy::parse_and_validate(cmd_val) {
                        Ok((prog, parsed_args)) => (prog, parsed_args),
                        Err(err) => {
                            return (
                                RiskLevel::Critical,
                                PolicyOutcome::Blocked,
                                format!("Command parsing violation: {}", err),
                            );
                        }
                    }
                } else if let Some(prog_val) = req.arguments.get("program").and_then(|v| v.as_str())
                {
                    let parsed_args = req
                        .arguments
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    (prog_val.to_string(), parsed_args)
                } else {
                    return (
                        RiskLevel::Low,
                        PolicyOutcome::Blocked,
                        "Missing command or program specification.".to_string(),
                    );
                }
            }
        };

        // 3. Prohibit direct invocation of shell interpreters
        let lower_prog = program.trim().to_lowercase();
        let is_shell = matches!(
            lower_prog.as_str(),
            "cmd" | "cmd.exe" | "powershell" | "powershell.exe" | "bash" | "sh" | "zsh"
        );
        if is_shell {
            return (
                RiskLevel::Critical,
                PolicyOutcome::Blocked,
                "Security Policy Violation: Direct invocation of shell interpreters is strictly prohibited."
                    .to_string(),
            );
        }

        // 4. Shell operator and metacharacter checks in arguments
        let dangerous_operators = ["&", "|", ";", ">", "<", "`", "$(", "%"];
        for arg in &args {
            for op in dangerous_operators {
                if arg.contains(op) {
                    return (
                        RiskLevel::Critical,
                        PolicyOutcome::Blocked,
                        format!(
                            "Security Policy Violation: Shell operator '{}' detected in arguments. Multi-command chaining is prohibited.",
                            op
                        ),
                    );
                }
            }
        }

        // 5. Destructive commands detection
        let is_destructive = matches!(
            lower_prog.as_str(),
            "rm" | "del"
                | "rmdir"
                | "format"
                | "fdisk"
                | "kill"
                | "pkill"
                | "shutdown"
                | "reboot"
                | "sudo"
        );

        if is_destructive {
            return (
                RiskLevel::High,
                PolicyOutcome::ConfirmationRequired,
                format!(
                    "Potentially destructive system command '{}' requires explicit operator confirmation.",
                    lower_prog
                ),
            );
        }

        // 6. Path containment check if working directory or path arguments are provided
        if let Some(work_dir) = req.arguments.get("working_dir").and_then(|v| v.as_str()) {
            let path_buf = PathBuf::from(work_dir);
            if !ctx.workspace_roots.is_empty() {
                let inside_workspace = ctx.workspace_roots.iter().any(|root| {
                    if let (Ok(c_path), Ok(c_root)) = (path_buf.canonicalize(), root.canonicalize())
                    {
                        c_path.starts_with(c_root)
                    } else {
                        false
                    }
                });
                if !inside_workspace {
                    return (
                        RiskLevel::High,
                        PolicyOutcome::ConfirmationRequired,
                        format!(
                            "Working directory '{}' is outside designated workspace boundaries.",
                            work_dir
                        ),
                    );
                }
            }
        }

        // 7. Security Mode & Risk Evaluation
        match CommandPolicy::evaluate_risk(&lower_prog, &args) {
            Ok((risk_str, req_approval)) => {
                let risk = match risk_str {
                    "safe" => RiskLevel::Safe,
                    "low" => RiskLevel::Low,
                    "medium" => RiskLevel::Medium,
                    "high" => RiskLevel::High,
                    _ => RiskLevel::Medium,
                };

                if req_approval {
                    (
                        risk,
                        PolicyOutcome::ConfirmationRequired,
                        format!(
                            "Command '{}' requires operator confirmation by policy rule.",
                            lower_prog
                        ),
                    )
                } else {
                    match ctx.security_mode {
                        SecurityMode::Restricted | SecurityMode::Strict => (
                            risk,
                            PolicyOutcome::ConfirmationRequired,
                            "Strict/Restricted security mode requires confirmation for all external commands.".to_string(),
                        ),
                        SecurityMode::Standard => {
                            if risk <= RiskLevel::Low {
                                (
                                    risk,
                                    PolicyOutcome::Allow,
                                    format!("Low-risk diagnostic command '{}' allowed in standard mode.", lower_prog),
                                )
                            } else {
                                (
                                    risk,
                                    PolicyOutcome::ConfirmationRequired,
                                    format!("Elevated command '{}' requires confirmation.", lower_prog),
                                )
                            }
                        }
                        SecurityMode::Autonomous | SecurityMode::Developer => {
                            if risk <= RiskLevel::Medium {
                                (
                                    risk,
                                    PolicyOutcome::Allow,
                                    format!("Command '{}' allowed under operational profile.", lower_prog),
                                )
                            } else {
                                (
                                    risk,
                                    PolicyOutcome::ConfirmationRequired,
                                    format!("High-risk command '{}' requires confirmation.", lower_prog),
                                )
                            }
                        }
                    }
                }
            }
            Err(err) => {
                // Command is outside catalog or has invalid arguments
                (
                    RiskLevel::High,
                    PolicyOutcome::ConfirmationRequired,
                    format!("Unclassified command or non-standard argument pattern: {}. Operator confirmation required.", err),
                )
            }
        }
    }
}
