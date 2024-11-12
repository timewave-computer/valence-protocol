# Valence Protocol Documentation

This folder contains the source code for the Valence Protocol documentation.

## Building

1. Install mdbook and mdbook-mermaid:

```bash
cargo install mdbook mdbook-mermaid
```

2. Build:

```bash
mdbook build
```

3. The output will be in the book subdirectory. Open it in your web browser. Examples:

- Firefox:

```bash
firefox book/index.html                       # Linux
open -a "Firefox" book/index.html             # OS X
Start-Process "firefox.exe" .\book\index.html # Windows (PowerShell)
start firefox.exe .\book\index.html           # Windows (Cmd)
```

- Chrome:

```bash
google-chrome book/index.html                 # Linux
open -a "Google Chrome" book/index.html       # OS X
Start-Process "chrome.exe" .\book\index.html  # Windows (PowerShell)
start chrome.exe .\book\index.html            # Windows (Cmd)
```

4. To apply docs changes automatically without rebuilding manually, run:

```bash
mdbook serve
```
