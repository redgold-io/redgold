use async_trait::async_trait;
use flume::Sender;
use redgold_schema::RgResult;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use std::fs;
use std::fs::File;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::servers::ServerOldFormat;
use std::io::Write;
use redgold_common::flume_send_help::SendErrorInfo;
use crate::cmd::run_command_os;

#[async_trait]
pub trait SSHOrCommandLike {
    async fn execute(&self, command: impl Into<String> + Send, output_handler: Option<Sender<String>>) -> RgResult<String>;
    async fn scp(&self, from: impl Into<String> + Send, to: impl Into<String> + Send, to_dest: bool, output_handler: Option<Sender<String>>) -> RgResult<String>;
    fn output(&self, o: impl Into<String>) -> RgResult<()>;

}

#[async_trait]
impl SSHOrCommandLike for SSHProcessInvoke {

    async fn execute(&self, command: impl Into<String> + Send, output_handler: Option<Sender<String>>) -> RgResult<String> {
        let identity_opt = self.identity_opt();
        let user = self.user_opt();
        let cmd = format!(
            "ssh {} {} {}@{} \"bash -c '{}'\"",
            self.strict_host_key_checking_opt(),
            identity_opt, user, self.host, command.into()
        );
        output_handler.clone().map(|s|
            s.send(format!("{}: {}", self.host, cmd.clone())).expect("send"));
        self.run_cmd(output_handler, cmd).await
    }

    async fn scp(&self, local_file: impl Into<String> + Send, remote_file: impl Into<String> + Send, to_dest: bool, output_handler: Option<Sender<String>>) -> RgResult<String> {
        let identity_opt = self.identity_opt();
        let user = self.user_opt();
        let lf = local_file.into();
        let first_arg = if to_dest { lf.clone() } else { "".to_string() };
        let last_arg = if to_dest { "".to_string() } else { lf };
        let cmd = format!(
            "scp {} {} {} {}@{}:{} {}",
            self.strict_host_key_checking_opt(),
            identity_opt, first_arg, user, self.host, remote_file.into(), last_arg
        );
        self.run_cmd(output_handler, cmd).await
    }

    fn output(&self, o: impl Into<String>) -> RgResult<()> {
        if let Some(s) = self.output_handler.clone() {
            s.send_rg_err(o.into())?;
        };
        Ok(())
    }
}

#[async_trait]
impl SSHOrCommandLike for LocalSSHLike {
    async fn execute(&self, command: impl Into<String> + Send, output_handler: Option<Sender<String>>) -> RgResult<String> {
        self.run_cmd(output_handler, command.into()).await
    }


    async fn scp(&self, local_file: impl Into<String> + Send, remote_file: impl Into<String> + Send, to_dest: bool, output_handler: Option<Sender<String>>) -> RgResult<String> {
        let lf = local_file.into();
        let rf = remote_file.into();
        let first_arg = if to_dest { lf.clone() } else { rf.clone() };
        let last_arg = if to_dest { rf } else { lf };
        let cmd = format!(
            "cp {} {}",
            first_arg, last_arg
        );
        self.run_cmd(output_handler, cmd).await
    }

    fn output(&self, o: impl Into<String>) -> RgResult<()> {
        if let Some(s) = self.output_handler.clone() {
            s.send_rg_err(o.into())?;
        };
        Ok(())
    }
}

pub struct DeployMachine<S: SSHOrCommandLike> {
    pub server: ServerOldFormat,
    pub ssh: S,
}

impl DeployMachine<SSHProcessInvoke> {

    pub fn new(s: &ServerOldFormat, identity_path: Option<String>, output_handler: Option<Sender<String>>) -> Self {
        let ssh = SSHProcessInvoke {
            user: s.username.clone(),
            // TODO: Home dir .join(".ssh").join("id_rsa")
            // Or override with a different path
            identity_path,
            host: s.host.clone(),
            strict_host_key_checking: false,
            output_handler: output_handler.clone()
        };
        Self {
            server: s.clone(),
            ssh,
        }
    }
}

impl<S: SSHOrCommandLike> DeployMachine<S> {

    pub async fn verify(&mut self) -> Result<(), ErrorInfo> {
        let mut info = ErrorInfo::error_info("Cannot verify ssh connection");
        info.with_detail("server", self.server.json_or());
        self.ssh.execute("df", None)
            .await?
            .contains("Filesystem")
            .then(|| Ok(()))
            .unwrap_or(Err(info))
    }

    pub async fn verify_docker_running(&mut self, network_environment: &NetworkEnvironment) -> RgResult<()> {
        let mut info = ErrorInfo::error_info("Cannot find redgold docker container running");
        info.with_detail("server", self.server.json_or());
        let result = self.ssh.execute(
            format!("docker ps | grep redgold-{}",
                    network_environment.to_std_string()
            ), None)
            .await?;
        info.with_detail("docker_ps_result", result.clone());
        let valid = result.contains("Up") && result.contains("redgold-");
        valid
            .then(|| Ok(()))
            .unwrap_or(Err(info))
    }

    // TODO: Migrate output handler to stored in class
    pub async fn install_docker(&mut self, p: &Option<Sender<String>>) -> RgResult<()> {
        let compose = self.exes("docker-compose", p).await?;
        if !(compose.contains("applications")) {
            self.exes("curl -fsSL https://get.docker.com -o get-docker.sh; sh ./get-docker.sh", p).await?;
            self.exes("sudo apt install -y docker-compose", p).await?;
        }
        Ok(())
    }

    pub async fn exes(&mut self, command: impl Into<String> + Send, output_handler: &Option<Sender<String>>) -> RgResult<String> {
        self.ssh.execute(command, output_handler.clone()).await
    }

    pub async fn copy_p(
        &mut self, contents: impl Into<String> + Send, remote_path: impl Into<String> + Send,
        output_handler: &Option<Sender<String>>
    ) -> RgResult<()> {
        let contents = contents.into();
        let remote_path = remote_path.into();
        if let Some(s) = output_handler.clone() {
            s.send(format!("Copying to: {}", remote_path.clone())).expect("send");
        }
        self.exes(format!("rm -f {}", remote_path.clone()), &output_handler.clone()).await?;
        self.copy(contents, remote_path).await?;
        Ok(())
    }
    pub async fn copy(&mut self, contents: impl Into<String> + Send, remote_path: String) -> RgResult<()> {
        // println!("Copying to: {}", remote_path);
        let contents = contents.into();
        let path = "tmpfile";
        fs::remove_file("tmpfile").ok();
        let mut file = File::create(path).expect("create failed");
        file.write_all(contents.as_bytes()).expect("write temp file");
        self.ssh.scp("./tmpfile", &*remote_path, true, None).await?;
        fs::remove_file("tmpfile").unwrap();
        Ok(())
    }


}

#[derive(Clone, Debug)]
pub struct SSHProcessInvoke {
    pub user: Option<String>,
    pub identity_path: Option<String>,
    pub host: String,
    pub strict_host_key_checking: bool,
    pub output_handler: Option<Sender<String>>
}

impl SSHProcessInvoke {

    pub fn new(host: impl Into<String>, output_handler: Option<Sender<String>>) -> Self {
        Self {
            user: None,
            identity_path: None,
            host: host.into(),
            strict_host_key_checking: false,
            output_handler,
        }
    }
    fn identity_opt(&self) -> String {
        let identity_opt = self.identity_path.clone()
            .map(|i| format!("-i {}", i)).unwrap_or("".to_string());
        identity_opt
    }
    fn strict_host_key_checking_opt(&self) -> String {
        if !self.strict_host_key_checking {
            "-o StrictHostKeyChecking=no".to_string()
        } else {
            "".to_string()
        }
    }

    fn user_opt(&self) -> String {
        let user = self.user.clone().unwrap_or("root".to_string());
        user
    }

    async fn run_cmd(&self,
               output_handler: Option<Sender<String>>,
               cmd: String
    ) -> RgResult<String> {
        let (stdout, stderr) = run_command_os(cmd).await?;
        if let Some(s) = output_handler {
            self.output(stdout.clone())?;
            self.output(stderr.clone())?;
        }
        Ok(format!("{}\n{}", stdout, stderr).to_string())
    }

}

pub struct LocalSSHLike {
    pub output_handler: Option<Sender<String>>
}

impl LocalSSHLike {
    pub fn new(output_handler: Option<Sender<String>>) -> Self {
        Self {
            output_handler
        }
    }

    async fn run_cmd(&self,
                     output_handler: Option<Sender<String>>,
                     cmd: String
    ) -> RgResult<String> {
        let (stdout, stderr) = run_command_os(cmd).await?;
        if let Some(s) = output_handler {
            self.output(stdout.clone())?;
            self.output(stderr.clone())?;
        }
        Ok(format!("{}\n{}", stdout, stderr).to_string())
    }

}