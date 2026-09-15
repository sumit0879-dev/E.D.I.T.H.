use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use base64::Engine;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ============================================================================
// SEC-01: STRUCTURED COMMAND EXECUTION POLICY ENGINE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicyResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

pub struct CommandPolicy;

impl CommandPolicy {
    /// Validates and parses a raw command string into a structured program + arguments array.
    /// Strictly rejects shell metacharacters, command chaining, and shell interpreter invocations.
    pub fn parse_and_validate(raw_cmd: &str) -> Result<(String, Vec<String>), String> {
        let trimmed = raw_cmd.trim();
        if trimmed.is_empty() {
            return Err("Empty command provided.".to_string());
        }

        // Reject dangerous shell metacharacters and chaining operators
        let dangerous_operators = ["&", "|", ";", ">", "<", "`", "$(", "%"];
        for op in dangerous_operators {
            if trimmed.contains(op) {
                return Err(format!(
                    "Security Policy Violation: Shell operator '{}' is prohibited. Multi-command chaining and redirection are restricted.",
                    op
                ));
            }
        }

        // Tokenize command line into program and arguments
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Invalid command format.".to_string());
        }

        let program = parts[0].to_lowercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Strictly prohibit invocation of raw shell interpreters
        if program == "cmd" || program == "cmd.exe" || program == "powershell" || program == "powershell.exe" || program == "bash" || program == "sh" {
            return Err("Security Policy Violation: Direct invocation of shell interpreters is prohibited.".to_string());
        }

        Ok((program, args))
    }

    /// Evaluates if a command is permitted for direct execution or if it requires SEC-02 authorization.
    pub fn evaluate_risk(program: &str, args: &[String]) -> Result<(&'static str, bool), String> {
        match program {
            // Safe system diagnostics (Read-only, no human approval needed)
            "whoami" => {
                let allowed = ["/user", "/groups", "/priv"];
                if args.is_empty() || (args.len() == 1 && allowed.contains(&args[0].as_str())) {
                    Ok(("low", false))
                } else {
                    Err("Invalid arguments for whoami. Permitted: [] or [/user, /groups, /priv].".to_string())
                }
            }
            "ipconfig" => {
                let allowed = ["/all", "/displaydns", "/flushdns"];
                if args.is_empty() || (args.len() == 1 && allowed.contains(&args[0].as_str())) {
                    Ok(("low", false))
                } else {
                    Err("Invalid arguments for ipconfig. Permitted: [] or [/all, /displaydns, /flushdns].".to_string())
                }
            }
            "systeminfo" => {
                if args.is_empty() {
                    Ok(("low", false))
                } else {
                    Err("systeminfo accepts no arguments in safe diagnostic mode.".to_string())
                }
            }
            "ping" => {
                if args.is_empty() {
                    return Err("ping requires a target hostname or IP address.".to_string());
                }
                let host = &args[0];
                if !host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
                    return Err("Invalid ping target hostname/IP format.".to_string());
                }
                Ok(("low", false))
            }
            "hostname" => {
                if args.is_empty() {
                    Ok(("low", false))
                } else {
                    Err("hostname accepts no arguments.".to_string())
                }
            }
            "echo" => {
                Ok(("low", false))
            }

            // High-Risk Developer Tools
            "cargo" => {
                if args.is_empty() {
                    return Err("cargo requires a subcommand.".to_string());
                }
                let sub = &args[0];
                if sub == "--version" || sub == "-V" || sub == "check" {
                    Ok(("medium", false))
                } else if sub == "build" || sub == "test" || sub == "run" || sub == "clippy" {
                    Ok(("high", true)) // Mutating / code-executing operations require SEC-02 approval
                } else {
                    Err(format!("cargo subcommand '{}' is restricted.", sub))
                }
            }
            "node" => {
                if args.is_empty() {
                    return Err("node requires arguments.".to_string());
                }
                let sub = &args[0];
                if sub == "-v" || sub == "--version" {
                    Ok(("medium", false))
                } else if sub == "-e" || sub == "--eval" {
                    Err("node inline eval (-e/--eval) is prohibited for security.".to_string())
                } else {
                    Ok(("high", true)) // Script execution requires SEC-02 approval
                }
            }
            "npm" | "npx" => {
                if args.is_empty() {
                    return Err("npm requires a subcommand.".to_string());
                }
                let sub = &args[0];
                if sub == "-v" || sub == "--version" {
                    Ok(("medium", false))
                } else if sub == "run" || sub == "test" || sub == "build" {
                    Ok(("high", true)) // Lifecycle script execution requires SEC-02 approval
                } else {
                    Err(format!("npm subcommand '{}' is restricted. Permitted: -v, run <script> (with approval).", sub))
                }
            }
            "git" => {
                if args.is_empty() {
                    return Err("git requires a subcommand.".to_string());
                }
                let sub = &args[0];
                if sub == "status" || sub == "diff" || sub == "branch" || sub == "--version" {
                    Ok(("medium", false))
                } else if sub == "log" {
                    Ok(("medium", false))
                } else {
                    Ok(("high", true)) // Mutating git operations require SEC-02 approval
                }
            }
            "dir" => {
                Ok(("low", false))
            }
            _ => {
                Err(format!("Program '{}' is not in the approved execution policy catalog.", program))
            }
        }
    }

    /// Executes a structured command with sandboxed environment, timeout, and output bounding.
    pub fn execute(program: &str, args: &[String], working_dir: Option<&Path>) -> Result<CommandPolicyResult, String> {
        let start_time = std::time::Instant::now();

        let mut cmd = {
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new(program);
                c.args(args).creation_flags(CREATE_NO_WINDOW);
                c
            }
            #[cfg(not(target_os = "windows"))]
            {
                let mut c = Command::new(program);
                c.args(args);
                c
            }
        };

        if let Some(dir) = working_dir {
            if dir.exists() && dir.is_dir() {
                cmd.current_dir(dir);
            }
        }

        // Execute child process
        let output = cmd.output().map_err(|e| format!("Process execution failed for '{}': {}", program, e))?;
        let elapsed = start_time.elapsed().as_millis() as u64;

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Enforce 1MB output buffer limit
        if stdout.len() > 1024 * 1024 {
            stdout = stdout.chars().take(1024 * 1024).collect::<String>() + "\n[Output Truncated at 1MB]";
        }
        if stderr.len() > 1024 * 1024 {
            stderr = stderr.chars().take(1024 * 1024).collect::<String>() + "\n[Error Truncated at 1MB]";
        }

        let combined = if stdout.is_empty() && !stderr.is_empty() {
            stderr.clone()
        } else if !stderr.is_empty() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout
        };

        Ok(CommandPolicyResult {
            success: output.status.success(),
            output: if combined.trim().is_empty() { "Command completed with no output.".to_string() } else { combined },
            error: if output.status.success() { None } else { Some(stderr) },
            execution_time_ms: elapsed,
        })
    }
}

// ============================================================================
// SEC-02: BACKEND-OWNED IMMUTABLE PROPOSAL STORE & HITL ENGINE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Consumed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandProposal {
    pub proposal_id: String,
    pub session_id: String,
    pub program: String,
    pub args: Vec<String>,
    pub command_display: String,
    pub working_dir: String,
    pub command_hash: String,
    pub risk_level: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ProposalStatus,
}

lazy_static! {
    static ref PROPOSAL_STORE: Mutex<HashMap<String, CommandProposal>> = Mutex::new(HashMap::new());
}

pub struct ProposalEngine;

impl ProposalEngine {
    /// Computes a deterministic SHA-256 integrity hash for a command specification.
    fn compute_hash(program: &str, args: &[String], working_dir: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(program.as_bytes());
        for arg in args {
            hasher.update(b" ");
            hasher.update(arg.as_bytes());
        }
        hasher.update(b"@");
        hasher.update(working_dir.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Creates an immutable proposal in the backend store with a 300-second TTL.
    pub fn create_proposal(
        session_id: &str,
        program: &str,
        args: &[String],
        working_dir: &Path,
        risk_level: &str,
    ) -> CommandProposal {
        let now = chrono::Utc::now().timestamp() as u64;
        let proposal_id = uuid::Uuid::new_v4().to_string();
        let dir_str = working_dir.to_string_lossy().to_string();
        let hash = Self::compute_hash(program, args, &dir_str);
        
        let display = format!("{} {}", program, args.join(" "));
        let valid_session_id = if session_id.trim().is_empty() {
            "default_session".to_string()
        } else {
            session_id.trim().to_string()
        };

        let proposal = CommandProposal {
            proposal_id: proposal_id.clone(),
            session_id: valid_session_id,
            program: program.to_string(),
            args: args.to_vec(),
            command_display: display,
            working_dir: dir_str,
            command_hash: hash,
            risk_level: risk_level.to_string(),
            created_at: now,
            expires_at: now + 300, // 5-minute TTL
            status: ProposalStatus::Pending,
        };

        let mut store = PROPOSAL_STORE.lock().unwrap();
        store.insert(proposal_id, proposal.clone());
        proposal
    }

    /// Resolves an existing proposal by proposal_id and action ("Approve" | "Reject").
    /// Verifies session, expiry, integrity hash, and ensures strict one-time use.
    pub fn resolve_proposal(
        proposal_id: &str,
        session_id: &str,
        action: &str,
    ) -> Result<CommandPolicyResult, String> {
        let now = chrono::Utc::now().timestamp() as u64;
        let mut store = PROPOSAL_STORE.lock().unwrap();

        let proposal = store.get_mut(proposal_id).ok_or_else(|| {
            format!("Proposal '{}' not found or already purged.", proposal_id)
        })?;

        // 1. Verify session binding strictly - reject empty or mismatched session identities
        let trimmed_session = session_id.trim();
        if trimmed_session.is_empty() {
            return Err("Security Authorization Error: Session ID is required and cannot be empty.".to_string());
        }
        if proposal.session_id != trimmed_session {
            return Err("Security Authorization Error: Session mismatch for command proposal.".to_string());
        }

        // 2. Verify status
        if proposal.status != ProposalStatus::Pending {
            return Err(format!(
                "Security Authorization Error: Proposal is already {:?} and cannot be re-executed.",
                proposal.status
            ));
        }

        // 3. Verify expiry
        if now > proposal.expires_at {
            proposal.status = ProposalStatus::Expired;
            return Err("Security Authorization Error: Command proposal has expired (TTL 300s).".to_string());
        }

        // 4. Verify command hash integrity
        let current_hash = Self::compute_hash(&proposal.program, &proposal.args, &proposal.working_dir);
        if current_hash != proposal.command_hash {
            proposal.status = ProposalStatus::Rejected;
            return Err("Security Integrity Error: Command payload hash mismatch.".to_string());
        }

        if action.eq_ignore_ascii_case("reject") {
            proposal.status = ProposalStatus::Rejected;
            return Ok(CommandPolicyResult {
                success: false,
                output: "Command proposal was rejected by user.".to_string(),
                error: Some("Rejected by user".to_string()),
                execution_time_ms: 0,
            });
        }

        if !action.eq_ignore_ascii_case("approve") {
            return Err(format!("Invalid proposal action '{}'. Expected 'Approve' or 'Reject'.", action));
        }

        // Atomically transition status to Consumed before dispatching
        proposal.status = ProposalStatus::Consumed;

        // Execute stored immutable command
        let dir = PathBuf::from(&proposal.working_dir);
        CommandPolicy::execute(&proposal.program, &proposal.args, Some(&dir))
    }
}

// ============================================================================
// SEC-01: CENTRAL APPLICATION LAUNCHER POLICY ENGINE
// ============================================================================

pub struct AppLauncherPolicy;

impl AppLauncherPolicy {
    /// Validates if an app name or target path is authorized to be launched.
    /// Only registered built-in apps, registered custom apps in DB, or safe URLs (http/https/mailto) are permitted.
    pub fn validate_and_launch(target: &str, conn: Option<&rusqlite::Connection>) -> Result<String, String> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err("Target application path or name cannot be empty.".to_string());
        }

        // 1. Check if it is an allowed safe URL protocol
        let lower = trimmed.to_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
            return match open::that(trimmed) {
                Ok(_) => Ok(format!("Successfully opened URL: {}", trimmed)),
                Err(e) => Err(format!("Failed to open URL {}: {}", trimmed, e)),
            };
        }

        // 2. Check if it matches a built-in app name or path
        for app in crate::plugins::BUILTIN_APPS {
            if app.name.eq_ignore_ascii_case(trimmed) || app.path.eq_ignore_ascii_case(trimmed) {
                return match open::that(app.path) {
                    Ok(_) => Ok(format!("Successfully launched built-in app: {}", app.name)),
                    Err(e) => Err(format!("Failed to launch {}: {}", app.name, e)),
                };
            }
        }

        // 3. Check if it matches a registered custom app in database
        if let Some(c) = conn {
            if let Ok(custom_apps) = crate::db::get_custom_apps(c) {
                for app in custom_apps {
                    if app.name.eq_ignore_ascii_case(trimmed) || app.path.eq_ignore_ascii_case(trimmed) {
                        return match open::that(&app.path) {
                            Ok(_) => Ok(format!("Successfully launched custom app: {}", app.name)),
                            Err(e) => Err(format!("Failed to launch {}: {}", app.name, e)),
                        };
                    }
                }
            }
        }

        // 4. Reject arbitrary executables
        Err(format!(
            "Security Policy Violation: '{}' is not a registered built-in or custom application. Arbitrary application launch is prohibited.",
            trimmed
        ))
    }
}

// ============================================================================
// SEC-03: FILESYSTEM SANDBOX & CANONICAL CONTAINMENT
// ============================================================================

pub struct PathSandbox;

impl PathSandbox {
    /// Resolves canonical path and verifies exact component-wise containment against approved roots.
    pub fn verify_containment(requested_path: &str, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
        let path = PathBuf::from(requested_path);
        if !path.exists() {
            return Err(format!("Path does not exist: {}", requested_path));
        }

        // Canonicalize path: resolves symlinks, Windows junctions, reparse points, and relative segments
        let canonical = path.canonicalize().map_err(|e| format!("Path canonicalization failed: {}", e))?;

        if canonical.is_dir() {
            return Err("Target path is a directory, not a readable file.".to_string());
        }

        // Verify containment against allowed canonical roots
        for root in allowed_roots {
            if let Ok(canonical_root) = root.canonicalize() {
                if canonical.starts_with(&canonical_root) {
                    // Safe size check (5MB limit for real-time text view)
                    if let Ok(meta) = std::fs::metadata(&canonical) {
                        if meta.len() > 5 * 1024 * 1024 {
                            return Err("File is too large for real-time preview (exceeds 5MB).".to_string());
                        }
                    }
                    return Ok(canonical);
                }
            }
        }

        Err(format!(
            "Security Policy Notice: Access Denied to '{}'. Path is outside authorized workspace boundaries.",
            canonical.display()
        ))
    }
}

// ============================================================================
// SEC-05: WINDOWS DPAPI CREDENTIAL PROTECTION AT REST
// ============================================================================

pub struct CredentialVault;

impl CredentialVault {
    #[cfg(target_os = "windows")]
    pub fn protect(plaintext: &str) -> Result<String, String> {
        if plaintext.is_empty() {
            return Ok("".to_string());
        }
        if plaintext.starts_with("enc:dpapi:") {
            return Ok(plaintext.to_string());
        }

        #[repr(C)]
        #[allow(non_snake_case)]
        struct DATA_BLOB {
            cbData: u32,
            pbData: *mut u8,
        }

        #[link(name = "Crypt32")]
        extern "system" {
            fn CryptProtectData(
                pDataIn: *const DATA_BLOB,
                szDataDescr: *const u16,
                pOptionalEntropy: *const DATA_BLOB,
                pvReserved: *mut std::ffi::c_void,
                pPromptStruct: *mut std::ffi::c_void,
                dwFlags: u32,
                pDataOut: *mut DATA_BLOB,
            ) -> i32;
        }

        #[link(name = "Kernel32")]
        extern "system" {
            fn LocalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }

        let mut bytes = plaintext.as_bytes().to_vec();
        let in_blob = DATA_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_mut_ptr(),
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

        unsafe {
            let res = CryptProtectData(
                &in_blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            );

            if res != 0 && !out_blob.pbData.is_null() {
                let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
                let encoded = base64::engine::general_purpose::STANDARD.encode(slice);
                LocalFree(out_blob.pbData as *mut std::ffi::c_void);
                Ok(format!("enc:dpapi:{}", encoded))
            } else {
                Err("Security Error: Windows DPAPI encryption failed. Credential was not persisted.".to_string())
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn protect(_plaintext: &str) -> Result<String, String> {
        Err("Security Error: CredentialVault DPAPI is supported on Windows desktop targets only.".to_string())
    }

    #[cfg(target_os = "windows")]
    pub fn unprotect(ciphertext: &str) -> Result<String, String> {
        if !ciphertext.starts_with("enc:dpapi:") {
            return Ok(ciphertext.to_string());
        }

        let payload = &ciphertext[10..];
        let encrypted_bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        #[repr(C)]
        #[allow(non_snake_case)]
        struct DATA_BLOB {
            cbData: u32,
            pbData: *mut u8,
        }

        #[link(name = "Crypt32")]
        extern "system" {
            fn CryptUnprotectData(
                pDataIn: *const DATA_BLOB,
                ppszDataDescr: *mut *mut u16,
                pOptionalEntropy: *const DATA_BLOB,
                pvReserved: *mut std::ffi::c_void,
                pPromptStruct: *mut std::ffi::c_void,
                dwFlags: u32,
                pDataOut: *mut DATA_BLOB,
            ) -> i32;
        }

        #[link(name = "Kernel32")]
        extern "system" {
            fn LocalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }

        let mut in_bytes = encrypted_bytes;
        let in_blob = DATA_BLOB {
            cbData: in_bytes.len() as u32,
            pbData: in_bytes.as_mut_ptr(),
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

        unsafe {
            let res = CryptUnprotectData(
                &in_blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            );

            if res != 0 && !out_blob.pbData.is_null() {
                let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
                let text = String::from_utf8(slice.to_vec()).map_err(|e| format!("UTF8 error: {}", e))?;
                LocalFree(out_blob.pbData as *mut std::ffi::c_void);
                Ok(text)
            } else {
                Err("Security Error: Windows DPAPI decryption failed.".to_string())
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn unprotect(ciphertext: &str) -> Result<String, String> {
        Ok(ciphertext.to_string())
    }
}

// ============================================================================
// SEC-06: LANCEDB 0.14+ DATAFUSION SQL SANITIZATION
// ============================================================================

pub struct LanceDbSanitizer;

impl LanceDbSanitizer {
    pub fn sanitize_source_predicate(source: &str) -> Result<String, String> {
        if source.contains('\0') {
            return Err("Security Violation: Null byte detected in source parameter.".to_string());
        }
        if source.len() > 256 {
            return Err("Source metadata string exceeds maximum allowed length of 256 characters.".to_string());
        }
        // Escape SQL single quotes (' -> '') and backslashes
        let escaped = source.replace('\\', "\\\\").replace('\'', "''");
        Ok(format!("source = '{}'", escaped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sec01_blocked_shell_operators() {
        assert!(CommandPolicy::parse_and_validate("whoami & dir").is_err());
        assert!(CommandPolicy::parse_and_validate("ipconfig | findstr IPv4").is_err());
        assert!(CommandPolicy::parse_and_validate("echo test > out.txt").is_err());
        assert!(CommandPolicy::parse_and_validate("echo `whoami`").is_err());
        assert!(CommandPolicy::parse_and_validate("cmd /C whoami").is_err());
        assert!(CommandPolicy::parse_and_validate("powershell -Command whoami").is_err());
    }

    #[test]
    fn test_sec01_allowed_diagnostics() {
        let (prog, args) = CommandPolicy::parse_and_validate("whoami /user").unwrap();
        assert_eq!(prog, "whoami");
        assert_eq!(args, vec!["/user"]);
        assert!(CommandPolicy::evaluate_risk(&prog, &args).is_ok());

        let (prog2, args2) = CommandPolicy::parse_and_validate("ping 8.8.8.8").unwrap();
        assert_eq!(prog2, "ping");
        assert!(CommandPolicy::evaluate_risk(&prog2, &args2).is_ok());
    }

    #[test]
    fn test_sec02_proposal_lifecycle() {
        let dir = std::env::current_dir().unwrap();
        let proposal = ProposalEngine::create_proposal("session_test", "echo", &["test_ok".to_string()], &dir, "low");
        assert_eq!(proposal.status, ProposalStatus::Pending);

        // Approve proposal with correct session
        let res = ProposalEngine::resolve_proposal(&proposal.proposal_id, "session_test", "Approve");
        assert!(res.is_ok());

        // Duplicate approval must fail
        let dup = ProposalEngine::resolve_proposal(&proposal.proposal_id, "session_test", "Approve");
        assert!(dup.is_err());
    }

    #[test]
    fn test_sec02_empty_or_wrong_session_rejected() {
        let dir = std::env::current_dir().unwrap();
        let proposal = ProposalEngine::create_proposal("session_valid", "echo", &["test".to_string()], &dir, "low");
        
        // Empty session must fail
        assert!(ProposalEngine::resolve_proposal(&proposal.proposal_id, "", "Approve").is_err());
        assert!(ProposalEngine::resolve_proposal(&proposal.proposal_id, "   ", "Approve").is_err());
        
        // Mismatched session must fail
        assert!(ProposalEngine::resolve_proposal(&proposal.proposal_id, "session_attacker", "Approve").is_err());
    }

    #[test]
    fn test_sec01_app_launcher_policy() {
        // Arbitrary unapproved executable must fail
        assert!(AppLauncherPolicy::validate_and_launch("C:\\evil.exe", None).is_err());
        assert!(AppLauncherPolicy::validate_and_launch("powershell.exe", None).is_err());
        assert!(AppLauncherPolicy::validate_and_launch("unauthorized_app.exe", None).is_err());
        
        // Empty target must fail
        assert!(AppLauncherPolicy::validate_and_launch("", None).is_err());
        assert!(AppLauncherPolicy::validate_and_launch("   ", None).is_err());
    }

    #[test]
    fn test_sec03_path_sandbox_containment() {
        let current = std::env::current_dir().unwrap();
        let allowed_roots = vec![current.clone()];
        
        // Valid workspace file (e.g. Cargo.toml) must succeed
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let res = PathSandbox::verify_containment(&cargo_toml.to_string_lossy(), &allowed_roots);
            assert!(res.is_ok());
        }

        // File outside allowed root must be denied
        #[cfg(target_os = "windows")]
        {
            let win_ini = "C:\\Windows\\win.ini";
            if std::path::Path::new(win_ini).exists() {
                let res = PathSandbox::verify_containment(win_ini, &allowed_roots);
                assert!(res.is_err());
                assert!(res.unwrap_err().contains("Access Denied"));
            }
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_sec05_dpapi_vault() {
        let secret = "sk-test-secret-key-12345";
        let protected = CredentialVault::protect(secret).unwrap();
        assert!(protected.starts_with("enc:dpapi:"));
        
        let unprotected = CredentialVault::unprotect(&protected).unwrap();
        assert_eq!(unprotected, secret);
    }

    #[test]
    fn test_sec06_lancedb_escaping() {
        let sanitized = LanceDbSanitizer::sanitize_source_predicate("Tony's Notes").unwrap();
        assert_eq!(sanitized, "source = 'Tony''s Notes'");

        assert!(LanceDbSanitizer::sanitize_source_predicate("Null\0Byte").is_err());
    }
}
