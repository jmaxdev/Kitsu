use anyhow::Result;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

pub struct Remote {
    pub url: String,
}

impl Remote {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn connect(&self, password: Option<&str>) -> Result<Session> {
        let url = self.url.trim_start_matches("ssh://");
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        let auth_host = parts[0];
        let _remote_path = if parts.len() > 1 { parts[1] } else { "." };
        let auth_parts: Vec<&str> = auth_host.split('@').collect();
        let (user, host_port) = if auth_parts.len() > 1 {
            (auth_parts[0], auth_parts[1])
        } else {
            ("root", auth_parts[0])
        };
        let host_port_parts: Vec<&str> = host_port.split(':').collect();
        let host = host_port_parts[0];
        let port = if host_port_parts.len() > 1 { host_port_parts[1] } else { "22" };
        let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        if let Ok(_) = sess.userauth_agent(user) {
            return Ok(sess);
        }
        if let Some(p) = password {
            if let Ok(_) = sess.userauth_password(user, p) {
                return Ok(sess);
            }
        }
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
        let id_rsa = home.join(".ssh/id_rsa");
        if id_rsa.exists() {
            if let Ok(_) = sess.userauth_pubkey_file(user, None, &id_rsa, None) {
                return Ok(sess);
            }
        }
        Err(anyhow::anyhow!("Authentication failed for user {}", user))
    }

    pub fn ensure_remote_dir(&self, sess: &Session, remote_path: &str) -> Result<()> {
        let sftp = sess.sftp()?;
        let path = Path::new(remote_path);
        let dirs = ["objects", "seals", "streams"];
        for d in dirs {
            let full_path = path.join(d);
            let _ = sftp.mkdir(&full_path, 0o755);
        }
        Ok(())
    }

    pub fn push_object(&self, sess: &Session, hash: &str, data: &[u8], remote_repo_path: &str) -> Result<()> {
        let sftp = sess.sftp()?;
        let (dir, file) = hash.split_at(2);
        let remote_dir = Path::new(remote_repo_path).join("objects").join(dir);
        let _ = sftp.mkdir(&remote_dir, 0o755);
        let remote_file = remote_dir.join(file);
        let mut f = sftp.create(&remote_file)?;
        f.write_all(data)?;
        Ok(())
    }

    pub fn fetch_object(&self, sess: &Session, hash: &str, remote_repo_path: &str) -> Result<Vec<u8>> {
        let sftp = sess.sftp()?;
        let (dir, file) = hash.split_at(2);
        let remote_file = Path::new(remote_repo_path).join("objects").join(dir).join(file);
        let mut f = sftp.open(&remote_file)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        Ok(data)
    }

    pub fn push_seal(&self, sess: &Session, name: &str, hash: &str, remote_repo_path: &str) -> Result<()> {
        let sftp = sess.sftp()?;
        let remote_file = Path::new(remote_repo_path).join("seals").join(name);
        let mut f = sftp.create(&remote_file)?;
        f.write_all(hash.as_bytes())?;
        Ok(())
    }

    pub fn fetch_seal(&self, sess: &Session, name: &str, remote_repo_path: &str) -> Result<String> {
        let sftp = sess.sftp()?;
        let remote_file = Path::new(remote_repo_path).join("seals").join(name);
        let mut f = sftp.open(&remote_file)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Ok(data.trim().to_string())
    }
}
