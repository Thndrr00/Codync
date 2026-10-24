//! Bubblewrap (`bwrap`) and classic-BPF seccomp generator for Linux (`linux_bwrap.rs`).

use super::plan::{NetworkIsolationMode, SandboxPlan};

pub const SECCOMP_ARCH_X86_64: u32 = 0xc000_003e;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const EPERM: u32 = 1;
const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ_K: u16 = 0x15;
const BPF_RET_K: u16 = 0x06;
const OFFSET_NR: u32 = 0;
const OFFSET_ARCH: u32 = 4;

/// High-risk Linux x86-64 syscalls denied inside the bot sandbox (matches OpenMausBot `seccomp.ts`).
pub const DENIED_SYSCALLS: &[(&str, u32)] = &[
    ("mount", 165),
    ("umount2", 166),
    ("pivot_root", 155),
    ("swapon", 167),
    ("swapoff", 168),
    ("reboot", 169),
    ("kexec_load", 246),
    ("kexec_file_load", 320),
    ("init_module", 175),
    ("finit_module", 313),
    ("delete_module", 176),
    ("bpf", 321),
    ("perf_event_open", 298),
    ("userfaultfd", 323),
    ("add_key", 248),
    ("request_key", 249),
    ("keyctl", 250),
    ("open_by_handle_at", 304),
    ("setns", 308),
    ("ptrace", 101),
    ("chroot", 161),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BpfInstruction {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Generates `bwrap` command-line arguments that mount system paths read-only,
/// replace `host_data_dir` (`~/.codync`) with an empty `--tmpfs`, and bind-mount
/// `workspace_dir` read-write.
pub fn bwrap_args(plan: &SandboxPlan, home_dir: Option<&std::path::Path>) -> Vec<String> {
    let mut args = Vec::new();
    for ro in &plan.read_only_system_paths {
        let s = ro.to_string_lossy().into_owned();
        args.extend(["--ro-bind-try".into(), s.clone(), s]);
    }
    args.extend(["--proc".into(), "/proc".into(), "--dev".into(), "/dev".into(), "--tmpfs".into(), "/tmp".into()]);
    if let Some(home) = home_dir {
        let h = home.to_string_lossy().into_owned();
        args.extend(["--bind".into(), h.clone(), h]);
    }
    // Replace host_data_dir (~/.codync) with an empty tmpfs so codync.db and identity.json do not exist.
    let data = plan.host_data_dir.to_string_lossy().into_owned();
    args.extend(["--tmpfs".into(), data]);
    // Mount back only this bot's workspace_dir.
    let ws = plan.workspace_dir.to_string_lossy().into_owned();
    args.extend(["--bind".into(), ws.clone(), ws.clone()]);
    args.extend([
        "--unshare-user".into(),
        "--unshare-pid".into(),
        "--unshare-ipc".into(),
        "--unshare-uts".into(),
        "--die-with-parent".into(),
        "--hostname".into(),
        "codync-box".into(),
    ]);
    if let NetworkIsolationMode::LoopbackOnlyWithGate { egress_socket } = &plan.network_mode {
        args.push("--unshare-net".into());
        let sock = egress_socket.to_string_lossy().into_owned();
        args.extend(["--bind".into(), sock, "/run/codync-egress.sock".into()]);
    }
    args.extend(["--chdir".into(), ws]);
    args
}

/// Builds the classic BPF program denying kernel-surface syscalls on x86-64.
pub fn seccomp_program() -> Vec<BpfInstruction> {
    let mut prog = vec![
        BpfInstruction { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: OFFSET_ARCH },
        BpfInstruction { code: BPF_JMP_JEQ_K, jt: 1, jf: 0, k: SECCOMP_ARCH_X86_64 },
        BpfInstruction { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_KILL_PROCESS },
        BpfInstruction { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: OFFSET_NR },
    ];
    let total = DENIED_SYSCALLS.len();
    for (idx, &(_, nr)) in DENIED_SYSCALLS.iter().enumerate() {
        let jump = u8::try_from(total - idx).unwrap_or(1);
        prog.push(BpfInstruction { code: BPF_JMP_JEQ_K, jt: jump, jf: 0, k: nr });
    }
    prog.push(BpfInstruction { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW });
    prog.push(BpfInstruction { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_ERRNO | EPERM });
    prog
}

/// Serializes the BPF instructions into little-endian `sock_filter` bytes for `bwrap --seccomp`.
pub fn seccomp_bytes() -> Vec<u8> {
    let prog = seccomp_program();
    let mut out = Vec::with_capacity(prog.len() * 8);
    for ins in prog {
        out.extend_from_slice(&ins.code.to_le_bytes());
        out.push(ins.jt);
        out.push(ins.jf);
        out.extend_from_slice(&ins.k.to_le_bytes());
    }
    out
}
