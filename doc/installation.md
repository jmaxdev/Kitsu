# Installation Guide

To use Kitsu globally in your terminal across your operating system, you need to install it. The easiest way is to download a pre-built binary from the GitHub Releases page.

## Method 1: Pre-built Binaries (Recommended)

You can download the compiled executable directly without needing to install Rust or compile from source.

1. Go to the [GitHub Releases page](https://github.com/jmaxdev/Kitsu/releases) for the project.
2. Download the appropriate `.zip` or `.tar.gz` archive for your operating system (Windows, macOS, or Linux).
3. Extract the downloaded archive. You will find the `kitsu` (or `kitsu.exe` on Windows) executable inside.

4. **Make it usable globally**:

   **On Windows:**
   - Create a folder for your custom CLI tools (e.g., `C:\CLI-Tools`).
   - Move or copy `kitsu.exe` into this folder.
   - Press the Windows Key, search for **"Environment Variables"** and select "Edit the system environment variables".
   - Click the "Environment Variables" button.
   - Under "System variables" (or "User variables"), find the `Path` variable, select it, and click "Edit".
   - Click "New" and add the path to your folder (e.g., `C:\CLI-Tools`).
   - Click OK on all windows. Restart your terminal (PowerShell/Command Prompt) and test by running `kitsu --version`.

   **On Linux / macOS:**
   - Move the extracted executable to a directory that is already in your PATH, such as `/usr/local/bin`:
     ```bash
     sudo mv kitsu /usr/local/bin/
     ```
   - Alternatively, add it to `~/.local/bin` (if it's in your PATH):
     ```bash
     mkdir -p ~/.local/bin
     mv kitsu ~/.local/bin/
     ```
   - Test by running `kitsu --version`.

---

## Method 2: Cargo Install (Build from Source)

If you prefer to compile from source and have the Rust toolchain installed, you can use `cargo install`.

**Prerequisites:**
- **Rust**: Stable Rust toolchain (edition 2024 compatible). Install via [rustup](https://rustup.rs/).
- **Linux**: `libssh2-1-dev`, `libssl-dev`, `pkg-config`
- **macOS**: `libssh2`, `openssl`, `pkg-config` (Homebrew)
- **Windows**: No additional system dependencies.

1. Clone the repository:
   ```bash
   git clone https://github.com/jmaxdev/Kitsu.git
   cd Kitsu
   ```

2. Run the cargo install command:
   ```bash
   cargo install --path .
   ```

3. Ensure that your Cargo bin directory (`~/.cargo/bin` or `C:\Users\<YourUsername>\.cargo\bin`) is added to your system's PATH.

## Autocompletion (Optional)

*(Future enhancement: Shell autocompletion scripts for Bash, Zsh, and PowerShell will be documented here once implemented).*
