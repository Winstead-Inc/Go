# LAFS / OLAFS Container Formatter

A pure-Rust, high-performance container formatter implementing the **Omniversal Language-Agnostic Formatting Style (OLAFS) / Improvised Allman Style (IAS)** for **C**, **C++**, **C#**, **Go**, **JavaScript**, **TypeScript**, **PHP**, **HTML**, and **CSS**.

---

## Architectural Principles

LAFS treats code containers (`()`, `{}`, `[]`, `<>`) as structural rooms whose boundaries form load-bearing columns. Every container is evaluated against a deterministic three-tier escalation hierarchy:

1. **State 1 - The Monolith (Horizontal Density)**:
   - When expressions, simple signatures, or atomic blocks fit within the Canvas width (default: 120 columns) and contain zero internal line breaks or comments, they are preserved as single-line horizontal units (`Foo(A, B)`, `int Add(int A, int B) { return A + B; }`).
2. **State 2 - Brace Grouping / Simplified Symmetry (Compound Delimiters)**:
   - When containers form a **Sole-Child Chain** without intervening logic (such as `[{( ... )}]` or `({ ... })`), their delimiters fuse into compound boundaries on a single line at the base indent, with mirrored closing runs (`)}];`).
3. **State 3 - Expanded Symmetry / Hierarchical Architecture (Load-Bearing Columns)**:
   - When control flow, multi-line statements, or canvas breaches occur, delimiters decouple onto dedicated lines.
   - **Zero End-of-Line Hugging**: Opening delimiters never hang at the end of a line (`FunctionName(` or `if (X) {` are strictly prohibited).
   - **Translational Symmetry**: Opening and closing delimiters share the exact same column coordinate.

---

## Features

- **Full Multi-Language Support**:
  - Handles **C / C++ / C# / Go / JS / TS / PHP / CSS**.
  - **Polyglot Files**: Intelligently parses mixed-content files (e.g., HTML containing `<script>`, `<style>`, and `<?php ... ?>` blocks) and formats each language chunk individually without breaking the surrounding HTML.
  - Preprocessor directives (`#include`, `#define`, `#if`, `#endif`, `#region`) preserved intact.
  - String literals (standard `"..."`, C++ raw strings `R"(...)"`, C# verbatim `@"..."`, C# raw `"""..."""`, Go raw strings `` `...` ``, and character literals).
  - Single-line comments (`//`) and multiline block comments (`/* ... */`) preserved without reformatting corruption.
  - Template and generic angle brackets (`<T, U>`) disambiguated using follow-token and context rules.
- **Safety Gate**:
  - Automatically verifies non-whitespace token stream equivalence between input and output.
  - Enforces idempotence (`Format(Format(Source)) == Format(Source)`).
  - Fails open rather than corrupting source code.
- **Zero External C Dependencies**:
  - Implemented in 100% pure Rust.
  - Builds instantly on Windows, Linux, and macOS without requiring MSVC or GCC build tools.

---

## Build & Installation

```powershell
cd b:\Go\LAFS
cargo build --release
```

The compiled binary will be located at:
`b:\Go\LAFS\target\release\lafs.exe`

---

## CLI Usage

### Formatting Files In-Place

```powershell
lafs -w file.c file.cpp file.cs file.php index.html
```

### Checking Files and Logging Failures

Exits with code `0` if all files are clean, or `1` if any file requires formatting. Output failed and unformatted paths to a specific log file:

```powershell
lafs --check --output-failed failed.txt .
```

### Stdin / Stdout Streaming (Editor / Pipe Mode)

```powershell
type MyFile.cpp | lafs -
```

### Custom Canvas Width & Indentation

```powershell
lafs --canvas-width 160 --indent 4 -w MyFile.cs
```

### Retrying Failed Files (`lafs format failed`)

Re-runs formatting on all files recorded in `failed.txt` (or custom `-of <FILE>`):

```powershell
lafs format failed
```

If all previously failed files succeed, `failed.txt` is automatically cleared.

### Automated Terminal & Error Logs

During execution, LAFS automatically logs all console output and errors with generation timestamps:
- `lafs {YYYY-MM-DD HH-MM-SS}.log`: All console output produced during the session.
- `lafs error {YYYY-MM-DD HH-MM-SS}.log`: Captured error diagnostics (generated whenever errors occur).

To disable automatic file logging, supply `--no-log`:
```powershell
lafs --no-log -w .
```

### CLI Reference

| Flag                           | Description                                                   |
| -------------------------------- | --------------------------------------------------------------- |
| `-w`, `--write`                | Overwrite files in-place with formatted output                |
| `--check`                      | Check if files are formatted (exit code 1 if unformatted)     |
| `format failed`                | Reformat all files listed in `failed.txt`                     |
| `-of`, `--output-failed <FILE>`| Log paths (with language) of files that failed or require formatting|
| `--no-log`                     | Disable automatic log file generation                         |
| `-c`, `--canvas-width <WIDTH>` | Maximum column width before vertical expansion (default: 160) |
| `-i`, `--indent <SIZE>`        | Indentation size in spaces (default: 4)                       |
| `-t`, `--tabs`                 | Use tabs instead of spaces for indentation                    |
| `-l`, `--language <LANG>`      | Force language mode (`c`, `cpp`, `cs`, `go`, `php`, `html`, `css`) |
| `--stdin-filepath <PATH>`      | Virtual file path when formatting from stdin                  |
| `-h`, `--help`                 | Show usage help                                               |
| `-v`, `--version`              | Show version info                                             |

