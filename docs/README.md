# Valence Programs Documentation

This folder contains the source code for the Valence Programs documentation.

## Building

1. Install mdbook, mdbook-mermaid and mdbook-linkcheck:

```bash
cargo install mdbook mdbook-mermaid mdbook-linkcheck
```

2. Build:

From `/docs` directory, run:

```bash
mdbook build
```

3. The output will be in the book subdirectory. Open it in your web browser. Examples:

- Firefox:

```bash
firefox book/html/index.html                             # Linux
open -a "Firefox" book/html/index.html                   # OS X
Start-Process "firefox.exe" .\book\html\index.html       # Windows (PowerShell)
start firefox.exe .\book\html\index.html                 # Windows (Cmd)
```

- Chrome:

```bash
google-chrome book/html/index.html                      # Linux
open -a "Google Chrome" book/index.html                 # OS X
Start-Process "chrome.exe" .\book\html\index.html       # Windows (PowerShell)
start chrome.exe .\book\html\index.html                 # Windows (Cmd)
```

4. To apply docs changes automatically without rebuilding manually, run:

```bash
mdbook serve
```
