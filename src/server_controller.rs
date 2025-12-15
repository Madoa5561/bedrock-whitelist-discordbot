use std::process::{Stdio, Child, ChildStdin};
use std::sync::{Arc, Mutex};
use std::io::Write;
use std::process::Command;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct ServerController {
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    server_path: String,
}

impl ServerController {
    pub fn new(server_path: String) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            server_path,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut process_guard = self.process.lock().unwrap();
        if process_guard.is_some() {
            println!("Server is already running.");
            return Ok(());
        }

        let path = std::path::Path::new(&self.server_path);
        let (work_dir, exe_path) = if path.is_file() {
            (
                path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
                path.to_path_buf()
            )
        } else {
            // Assume directory
            (
                path.to_path_buf(),
                path.join("bedrock_server.exe")
            )
        };

        println!("Starting {:?} from {:?}", exe_path, work_dir);
        
        let mut child = Command::new(&exe_path)
            .current_dir(&work_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit()) // Pipe stdout to parent to see logs
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;

        *process_guard = Some(child);
        *self.stdin.lock().unwrap() = Some(stdin);

        println!("Bedrock Server started successfully.");
        Ok(())
    }

    pub fn stop(&self) {
        println!("Stopping server...");
        if let Err(e) = self.send_command("stop") {
            eprintln!("Failed to send stop command: {}", e);
        }

        // Wait for process to exit
        let mut process_guard = self.process.lock().unwrap();
        if let Some(mut child) = process_guard.take() {
            match child.wait() {
                Ok(status) => println!("Server exited with status: {}", status),
                Err(e) => eprintln!("Error waiting for server exit: {}", e),
            }
        }
        
        // Clear stdin
        *self.stdin.lock().unwrap() = None;
        println!("Server stopped.");
    }

    pub fn restart(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.stop();
        // Give a small buffer time if needed, though wait() should handle it
        std::thread::sleep(std::time::Duration::from_secs(2));
        self.start()
    }

    pub fn send_command(&self, cmd: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut stdin_guard = self.stdin.lock().unwrap();
        if let Some(stdin) = stdin_guard.as_mut() {
            writeln!(stdin, "{}", cmd)?;
            stdin.flush()?;
            // println!("Sent command: {}", cmd); // Debug log
            Ok(())
        } else {
            Err("Server stdin is not available (server not running?)".into())
        }
    }
    
    pub fn is_running(&self) -> bool {
        let mut process_guard = self.process.lock().unwrap();
        if let Some(child) = process_guard.as_mut() {
             match child.try_wait() {
                Ok(Some(_)) => false, // Exited
                Ok(None) => true,     // Still running
                Err(_) => false,
            }
        } else {
            false
        }
    }
}
