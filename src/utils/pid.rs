use crate::utils::tools::exec_cmd;
use log::error;
use std::{
    fmt::Display,
    fs::{read_link, read_to_string},
};

pub struct PIDInfo {
    pub pid: u64,
    pub exe: String,
    pub root: String,
    pub cwd: String,
    pub cmdline: String,
    pub environ: String,
}

#[cfg(target_os = "linux")]
impl PIDInfo {
    pub fn new(pid: u64) -> Result<PIDInfo, Box<dyn std::error::Error>> {
        let exe = read_link(format!("/proc/{}/exe", pid))?
            .display()
            .to_string();
        let root = read_link(format!("/proc/{}/root", pid))?
            .display()
            .to_string();
        let cwd = read_link(format!("/proc/{}/cwd", pid))?
            .display()
            .to_string();
        let cmdline = read_to_string(format!("/proc/{}/cmdline", pid))?;
        let environ = read_to_string(format!("/proc/{}/environ", pid))?;
        Ok(PIDInfo {
            pid,
            exe,
            root,
            cwd,
            cmdline,
            environ,
        })
    }

    pub fn terminate(&self) {
        if !exec_cmd("kill", &["-9", &self.pid.to_string()], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to terminate PID {}", &self.pid);
        }
    }

    pub fn quarantine(&self) {
        if !exec_cmd("mv", &[&self.exe, "./quarantine"], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to move exe {}", &self.exe);
        }
        if !exec_cmd("chmod", &["444", &self.exe], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to chmod exe {}", &self.exe);
        }
    }
}

#[cfg(target_os = "freebsd")]
impl PIDInfo {
    pub fn new(pid: u64) -> Option<PIDInfo> {
        let exe_cmd = exec_cmd("procstat", &["-b", &pid.to_string()[..]], false)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let exe_stdout = match exe_cmd.status.success() {
            true => exe_cmd.stdout,
            false => {
                error!("Failed to get exe for PID {}", pid);
                return None;
            }
        };
        let exe_full = String::from_utf8_lossy(&exe_stdout);
        let exe = exe_full.split_whitespace().last().unwrap();

        let cwd_cmd = exec_cmd("procstat", &["pwdx", &pid.to_string()[..]], false)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let cwd_stdout = match cwd_cmd.status.success() {
            true => cwd_cmd.stdout,
            false => {
                error!("Failed to get cwd for PID {}", pid);
                return None;
            }
        };
        let cwd_full = String::from_utf8_lossy(&cwd_stdout);
        let cwd = cwd_full.split_whitespace().last().unwrap();

        let cmdline_cmd = exec_cmd("procstat", &["pargs", &pid.to_string()[..]], false)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let cmdline_stdout = match cmdline_cmd.status.success() {
            true => cmdline_cmd.stdout,
            false => {
                error!("Failed to get cmdline for PID {}", pid);
                return None;
            }
        };
        let cmdline_full = String::from_utf8_lossy(&cmdline_stdout);
        let mut cmdline: Vec<String> = Vec::new();
        for line in cmdline_full.split('\n') {
            cmdline.push(line.split_once(':').unwrap().1.trim().to_owned());
        }
        cmdline.remove(0);

        let environ_cmd = exec_cmd("procstat", &["penv", &pid.to_string()[..]], false)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let environ_stdout = match environ_cmd.status.success() {
            true => environ_cmd.stdout,
            false => {
                error!("Failed to get environ for PID {}", pid);
                return None;
            }
        };
        let environ_full = String::from_utf8_lossy(&environ_stdout);
        let mut environ: Vec<String> = Vec::new();
        for line in environ_full.split('\n') {
            environ.push(line.split_once(':').unwrap().1.trim().to_owned());
        }
        environ.remove(0);

        Some(PIDInfo {
            pid,
            exe: exe.to_string(), // -b
            root: String::from("N/A"),
            cwd: cwd.to_string(),              // pwdx
            cmdline: format!("{:?}", cmdline), // pargs
            environ: format!("{:?}", environ), // penv
        })
    }

    pub fn terminate(&self) {
        if !exec_cmd("kill", &["-9", &self.pid.to_string()], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to terminate PID {}", &self.pid);
        }
    }

    pub fn quarantine(&self) {
        if !exec_cmd("mv", &[&self.exe, "./quarantine"], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to move exe {}", &self.exe);
        }
        if !exec_cmd("chmod", &["444", &self.exe], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to chmod exe {}", &self.exe);
        }
    }
}

#[cfg(target_os = "windows")]
impl PIDInfo {
    pub fn new(pid: u64) -> Option<PIDInfo> {
        let mut out = PIDInfo {
            pid,
            exe: String::from("N/A"),
            root: String::from("N/A"),
            cwd: String::from("N/A"),
            cmdline: String::from("N/A"),
            environ: String::from("N/A"),
        };
        let exe_cmd = exec_cmd("powershell", &["-ExecutionPolicy", "Bypass", &format!("Get-WmiObject Win32_Process -Filter \"ProcessId = {}\" | Select-Object ExecutablePath, CommandLine | Format-List", pid)], false)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let exe_stdout = match exe_cmd.status.success() {
            true => exe_cmd.stdout,
            false => {
                error!("Failed to get process info");
                return None;
            }
        };
        let exe = String::from_utf8_lossy(&exe_stdout);

        let comps: Vec<&str> = exe.split("\r\n").collect();
        for comp in comps {
            match comp.split_once(':') {
                Some(key_val) => {
                    let key = key_val.0.trim();
                    let val = key_val.1.trim().to_owned();
                    match key {
                        "ExecutablePath" => {
                            out.exe = val;
                        }
                        "CommandLine" => {
                            out.cmdline = val;
                        }
                        _ => {}
                    }
                }
                None => {}
            }
        }
        Some(out)
    }

    pub fn terminate(&self) {
        if !exec_cmd("taskkill", &["/PID", &self.pid.to_string(), "/F"], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to terminate PID {}", &self.exe);
        }
    }

    pub fn quarantine(&self) {
        if !exec_cmd("move", &[&self.exe, ".\\quarantine"], false)
            .unwrap()
            .wait()
            .unwrap()
            .success()
        {
            error!("Failed to move exe {}", &self.exe);
        }
        println!("Please revoke all execution access, or get this thing out of here");
    }
}

impl Display for PIDInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | {} | {} | {} | {}",
            self.pid, self.exe, self.root, self.cwd, self.cmdline
        )
    }
}
