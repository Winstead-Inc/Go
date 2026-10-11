#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)] mod container_tree;
mod emitter;
mod formatter;
mod go;
mod js_ts;
mod lexer;
mod planner;
mod polyglot;
mod shell;
mod types;
mod verifier;

use std::env;
use std::fs;
use std::io:: { self, IsTerminal, Read, Write };
use std::path::Path;
use std::sync::atomic:: { AtomicUsize, Ordering };
use std::sync:: { Arc, Mutex };
use std::time::Instant;

use formatter::Source_Formatter;
use types:: { Config_Settings, Source_Language };

const DEFAULT_EXCLUDED_DIRS: & [ & str] = & 
[
    ".git", ".svn", ".hg", "target", "bin", "obj", "node_modules", "dist", "build", ".vs", ".vscode", ".idea", ".gradle", "venv", ".env", "vendor", "__pycache__", 
];

const DOCUMENT_EXTENSIONS: & [ & str] = & 
[
    "adoc", "asciidoc", "pdf", "docx", "doc", "md", "markdown", "mdown", "mkd", "rst", "rtf", "odt", "pages", "ppt", "pptx", "key", "xls", "xlsx", "numbers", "csv", "tsv", "epub", "tex", "latex", "txt", "org", "textile", "yml", "yaml", 
];

const BINARY_EXTENSIONS: & [ & str] = & 
[
    "exe", "dll", "so", "dylib", "obj", "o", "a", "lib", "bin", "iso", "img", "zip", "tar", "gz", "tgz", "7z", "rar", "bz2", "xz", "zst", "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "tiff", "tif", "psd", "ai", "svgz", "mp3", "mp4", "wav", "ogg", "mkv", "avi", "mov", "flv", "webm", "m4a", "flac", "woff", "woff2", "ttf", "eot", "otf", "pyc", "pyo", "class", "pdb", "idb", "suo", "db", "sqlite", "sqlite3", "lock", "lockb", 
];

const DEFAULT_SUPPORTED_EXTENSIONS: & [ & str] = & 
[
    "c", "h", "cpp", "hpp", "cc", "cxx", "hh", "hxx", "inl", "cu", "cs", "rs", "java", "kt", "scala", "js", "mjs", "cjs", "jsx", "ts", "tsx", "mts", "cts", "go", "dart", "php", "py", "swift", "m", "mm", "sh", "bash", "zsh", "ksh", "json", "jsonc", "json5", "toml", "css", "scss", "less", "sql", "graphql", "proto", "html", "htm", 
];

#[derive(Clone)]
pub struct Terminal_Logger
{
    Out_Buffer: Arc < Mutex < String >>,
    Err_Buffer: Arc < Mutex < String >>,
    Timestamp_Str: String,
    Disable_Log_Files: Arc < Mutex < bool >>,
}

impl Terminal_Logger
{
    pub fn New(Disable_Logs: bool)-> Self
    {
        let Timestamp_Str = chrono::Local::now().format("%Y-%m-%d %H-%M-%S").to_string();
        Terminal_Logger
        {
            Out_Buffer: Arc::new(Mutex::new(String::new())),
            Err_Buffer: Arc::new(Mutex::new(String::new())),
            Timestamp_Str,
            Disable_Log_Files: Arc::new(Mutex::new(Disable_Logs)),
        }
    }

    pub fn Set_Disable_Logs(& self, Disable: bool)
    {
        * self.Disable_Log_Files.lock().unwrap() = Disable;
    }

    pub fn Println(& self, Msg: & str)
    {
        println!("{}", Msg);
        let mut Buf = self.Out_Buffer.lock().unwrap();
        Buf.push_str(Msg);
        Buf.push('\n');
    }

    pub fn Print(& self, Msg: & str)
    {
        print!("{}", Msg);
        io::stdout().flush().ok();
        let mut Buf = self.Out_Buffer.lock().unwrap();
        Buf.push_str(Msg);
    }

    pub fn Eprintln(& self, Msg: & str)
    {
        eprintln!("{}", Msg);
        {
            let mut ErrBuf = self.Err_Buffer.lock().unwrap();
            ErrBuf.push_str(Msg);
            ErrBuf.push('\n');
        }
        {
            let mut OutBuf = self.Out_Buffer.lock().unwrap();
            OutBuf.push_str(Msg);
            OutBuf.push('\n');
        }
    }

    pub fn Flush_To_Files(& self)
    {
        if * self.Disable_Log_Files.lock().unwrap() { return ; }

        let OutText = self.Out_Buffer.lock().unwrap().clone();
        let ErrText = self.Err_Buffer.lock().unwrap().clone();

        if ! OutText.is_empty()
        {
            let OutFileName = format ! ("lafs {}.log", self.Timestamp_Str);
            if let Err(E) = fs::write( & OutFileName, & OutText)
            {
                eprintln!("Failed to write terminal output log '{}': {}", OutFileName, E);
            }
        }

        if ! ErrText.is_empty()
        {
            let ErrFileName = format ! ("lafs error {}.log", self.Timestamp_Str);
            if let Err(E) = fs::write( & ErrFileName, & ErrText)
            {
                eprintln!("Failed to write error log '{}': {}", ErrFileName, E);
            }
        }
    }
}

fn PrintUsage(Logger: & Terminal_Logger)
{
    Logger.Println("LAFS Container Formatter (Language-Agnostic Formatting Style)");
    Logger.Println("Usage:");
    Logger.Println("  lafs [OPTIONS] <FILES... | DIRECTORIES...>");
    Logger.Println("  lafs format [OPTIONS] <FILES... | DIRECTORIES...>");
    Logger.Println("  lafs format failed [OPTIONS]       (Reformat all files listed in failed.txt)");
    Logger.Println("  lafs apply [OPTIONS] <FILES... | DIRECTORIES...>");
    Logger.Println("  lafs check [OPTIONS] <FILES... | DIRECTORIES...>");
    Logger.Println("  lafs [OPTIONS] -                   (Read from stdin, output to stdout)");
    Logger.Println("");
    Logger.Println("Batch & Directory Options:");
    Logger.Println("  -a, --all                   Format all files in current directory and all subdirectories");
    Logger.Println("  -e, --ext <EXTENSIONS>      Only format files with specific extension(s) (e.g. -e c,h or -e sh)");
    Logger.Println("      --all-types             Format all text files regardless of extension (excludes binary and docs)");
    Logger.Println("  -x, --exclude <PATTERN>     Exclude files or folders (e.g. -x target -x vendor -x \"*.min.js\")");
    Logger.Println("  -y, --yes                   Automatic yes to confirmation prompts");
    Logger.Println("  -j, --threads <COUNT>       Number of parallel worker threads (default: CPU cores)");
    Logger.Println("      --verbose               Show details for unchanged clean files");
    Logger.Println("");
    Logger.Println("Formatting Options:");
    Logger.Println("  -w, --write                 Save and overwrite formatted files in-place (default)");
    Logger.Println("      --stdout                Output formatted code to stdout instead of saving files");
    Logger.Println("      --check                 Check if files are formatted (exit code 1 if unformatted)");
    Logger.Println("  -of, --output-failed <FILE> Output failed and unformatted paths to a specific log file");
    Logger.Println("      --no-log                Disable automatic log file generation");
    Logger.Println("  -c, --canvas-width <WIDTH>  Set maximum canvas width before expansion (default: 160)");
    Logger.Println("  -i, --indent <SIZE>         Set indentation width in spaces (default: 4)");
    Logger.Println("  -t, --tabs                  Use tabs instead of spaces for indentation");
    Logger.Println("  -l, --language <LANG>       Force source language: 'c', 'cpp', 'csharp', 'js', 'ts', 'sh'");
    Logger.Println("      --max-statements <NUM>  Max statements in a monolith block (default: 4)");
    Logger.Println("      --stdin-filepath <PATH> Virtual filepath when formatting stdin");
    Logger.Println("  -h, --help                  Show this help message");
    Logger.Println("  -v, --version               Show version information");
}

fn WildcardMatch(Pattern: & str, Text: & str)-> bool
{
    let PChars: Vec < char > = Pattern.chars().collect();
    let TChars: Vec < char > = Text.chars().collect();
    let mut PI = 0;
    let mut TI = 0;
    let mut StarP = None;
    let mut StarT = 0;

    while TI < TChars.len()
    {
        if PI < PChars.len() && (PChars[PI] == '?' || PChars[PI].to_ascii_lowercase() == TChars[TI].to_ascii_lowercase()) { PI += 1; TI += 1; }
        else if PI < PChars.len() && PChars[PI] == '*' { StarP = Some(PI); PI += 1; StarT = TI; }
        else if let Some(SP) = StarP { PI = SP + 1; StarT += 1; TI = StarT; } else { return false; }
    }

    while PI < PChars.len() && PChars[PI] == '*' { PI += 1; }

    PI == PChars.len()
}

fn IsBinaryFile(PathTarget: & Path)-> bool
{
    if let Ok(mut File) = fs::File::open(PathTarget)
    {
        let mut Buffer = [0u8; 8192];
        if let Ok(BytesRead) = File.read( & mut Buffer) { if BytesRead > 0 { return Buffer[..BytesRead].contains( & 0); } }
    }
    false
}

fn IsDirExcluded(DirName: & str, DirPath: & Path, ExcludedPatterns: & [String])-> bool
{
    if DEFAULT_EXCLUDED_DIRS.iter().any( |& D | D.eq_ignore_ascii_case(DirName)) { return true; }

    let DirPathStr = DirPath.to_string_lossy().replace('\\', "/");
    for Pattern in ExcludedPatterns
    {
        let CleanPattern = Pattern.trim_matches('/').trim_matches('\\');
        if WildcardMatch(CleanPattern, DirName) || WildcardMatch(CleanPattern, & DirPathStr) || DirPathStr.ends_with(CleanPattern) || DirPathStr.contains
        (
            & format!("/{}/", CleanPattern)
        )
        || DirPathStr.starts_with( & format ! ("{}/", CleanPattern)) { return true; }
    }

    false
}

fn IsFileEligible(PathTarget: & Path, AllTypes: bool , AllowedExtensions: & [String], ExcludedPatterns: & [String],)-> bool
{
    let FileName = PathTarget.file_name().unwrap_or_default().to_string_lossy();
    let PathStr = PathTarget.to_string_lossy().replace('\\', "/");
    // 1. Check user exclusion patterns
    for Pattern in ExcludedPatterns
    {
        let CleanPattern = Pattern.trim_matches('/').trim_matches('\\');
        if WildcardMatch(CleanPattern, & FileName) || WildcardMatch(CleanPattern, & PathStr) || PathStr.ends_with(CleanPattern) { return false; }
    }

    let Ext = PathTarget.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    // 2. Specific extension filter (-e / --ext / --extension)
    if ! AllowedExtensions.is_empty() { if ! AllowedExtensions.iter().any( | A | A.eq_ignore_ascii_case( & Ext)) { return false; } }
    else if ! AllTypes && ! DEFAULT_SUPPORTED_EXTENSIONS.iter().any( |& S | S == Ext.as_str()) { return false; }
    // 3. Exclude document file types (adoc, pdf, docx, md, yml, etc.)
    if DOCUMENT_EXTENSIONS.iter().any( |& D | D == Ext.as_str()) { return false; }
    // 4. Exclude known binary file types
    if BINARY_EXTENSIONS.iter().any( |& B | B == Ext.as_str()) { return false; }
    // 5. Fast binary content check for NUL bytes
    if IsBinaryFile(PathTarget) { return false; }

    true
}

fn CollectFilesRecursively
(
    DirPath: & Path, AllTypes: bool, AllowedExtensions: & [String], ExcludedPatterns: & [String], Collected: & mut Vec<String>,
)
{
    let DirName = DirPath.file_name().unwrap_or_default().to_string_lossy();
    if ! DirName.is_empty() && DirName != "." && IsDirExcluded( & DirName, DirPath, ExcludedPatterns) { return ; }

    if let Ok(Entries) = fs::read_dir(DirPath)
    {
        for Entry in Entries.flatten()
        {
            let SubPath = Entry.path();
            if SubPath.is_dir()
            {
                let SubName = SubPath.file_name().unwrap_or_default().to_string_lossy();
                if ! IsDirExcluded( & SubName, & SubPath, ExcludedPatterns)
                {
                    CollectFilesRecursively( & SubPath, AllTypes, AllowedExtensions, ExcludedPatterns, Collected,);
                }
            }
            else if SubPath.is_file()
            {
                if IsFileEligible( & SubPath, AllTypes, AllowedExtensions, ExcludedPatterns)
                {
                    Collected.push(SubPath.to_string_lossy().to_string());
                }
            }
        }
    }
}

fn ExpandGlobPattern(Pattern: & str, AllTypes: bool , AllowedExtensions: & [String], ExcludedPatterns: & [String], Collected: & mut Vec<String>,)
{
    let Normalized = Pattern.replace('\\', "/");
    let (DirPart, FilePattern) = if let Some(SlashIdx) = Normalized.rfind('/') { ( & Normalized[..SlashIdx], & Normalized[SlashIdx + 1..]) }
    else { (".", Normalized.as_str()) };

    let DirPath = Path:: new (DirPart);
    if let Ok(Entries) = fs::read_dir(DirPath)
    {
        for Entry in Entries.flatten()
        {
            let P = Entry.path();
            if P.is_file()
            {
                if let Some(FileName) = P.file_name().and_then( | N | N.to_str())
                {
                    if WildcardMatch(FilePattern, FileName) && IsFileEligible( & P, AllTypes, AllowedExtensions, ExcludedPatterns)
                    {
                        Collected.push(P.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

fn PromptConfirmation(Logger: & Terminal_Logger, Count: usize, TargetDesc: & str)-> bool
{
    Logger.Print(& format ! ("Found {} files in '{}'. Apply LAFS formatting and save in-place? [y/N]: ", Count, TargetDesc));

    let mut Input = String:: new ();
    if io::stdin().read_line( & mut Input).is_ok() { let Trimmed = Input.trim().to_lowercase(); matches ! (Trimmed.as_str(),"y" | "yes") } else { false }
}

fn Main()
{
    let Logger = Terminal_Logger::New(false);
    let Args: Vec < String > = env::args().collect();
    let mut OutputToStdout = false;
    let mut CheckOnly = false;
    let mut FormatAll = false;
    let mut AllTypes = false;
    let mut AutoYes = false;
    let mut Verbose = false;
    let mut ReformatFailed = false;
    let mut CustomThreads: Option < usize > = None;
    let mut AllowedExtensions: Vec < String > = Vec:: new ();
    let mut ExcludedPatterns: Vec < String > = Vec:: new ();
    let mut Config = Config_Settings:: default();
    let mut RawTargets: Vec < String > = Vec:: new ();
    let mut ReadFromStdin = false;
    let mut StdinVirtualPath: Option < String > = None;
    let mut OutputFailedPath: Option < String > = None;

    if Args.len() <= 1
    {
        if io::stdin().is_terminal()
        {
            Logger.Print("No target files specified. Format all files in current directory and subdirectories? [y/N]: ");
            let mut Input = String:: new ();
            if io::stdin().read_line( & mut Input).is_ok()
            {
                let Trimmed = Input.trim().to_lowercase();
                if matches!(Trimmed.as_str(),"y" | "yes") { FormatAll = true; AutoYes = true; RawTargets.push(".".to_string()); }
                else { Logger.Set_Disable_Logs(true); PrintUsage( & Logger); return ; }
            }
            else { Logger.Set_Disable_Logs(true); PrintUsage( & Logger); return ; }
        }
        else { ReadFromStdin = true; }
    }

    let mut I = 1;
    while I < Args.len()
    {
        let Arg = & Args[I];
        match Arg.as_str()
        {
            "format" | "apply" | "run" => {}
            "failed" | "--failed" => { ReformatFailed = true; }
            "--no-log" => { Logger.Set_Disable_Logs(true); }
            "all" | "-a" | "--all" => { FormatAll = true; }
            "-e" | "--ext" | "--extension" | "--include-ext" => 
            {
                I += 1;
                if I < Args.len()
                {
                    for Part in Args[I].split(',')
                    {
                        let Clean = Part.trim().trim_start_matches('.').to_lowercase();
                        if ! Clean.is_empty() { AllowedExtensions.push(Clean); }
                    }
                }
            }
            "--all-types" | "--any-type" => { AllTypes = true; }
            "-x" | "--exclude" | "--ignore" => 
            {
                I += 1;
                if I < Args.len()
                {
                    for Part in Args[I].split(',')
                    {
                        let Trimmed = Part.trim();
                        if ! Trimmed.is_empty() { ExcludedPatterns.push(Trimmed.to_string()); }
                    }
                }
            }
            "-y" | "--yes" => { AutoYes = true; }
            "--verbose" => { Verbose = true; }
            "-j" | "--threads" => 
            {
                I += 1;
                if I < Args.len()
                {
                    if let Ok(Count) = Args[I].parse::<usize>() { if Count > 0 { CustomThreads = Some(Count); } }
                }
            }
            "-w" | "--write" => { OutputToStdout = false; }
            "--stdout" => { OutputToStdout = true; }
            "--output-failed" | "-of" => { I += 1; if I < Args.len() { OutputFailedPath = Some(Args[I].clone()); } }
            "--check" => { CheckOnly = true; }
            "-c" | "--canvas-width" | "--width" => 
            {
                I += 1;
                if I < Args.len() { if let Ok(W) = Args[I].parse::<usize>() { Config.Canvas_Width = W; } }
            }
            "--max-statements" => 
            {
                I += 1;
                if I < Args.len() { if let Ok(M) = Args[I].parse::<usize>() { Config.Max_Statements_In_Monolith = M; } }
            }
            "-i" | "--indent" => 
            {
                I += 1;
                if I < Args.len() { if let Ok(S) = Args[I].parse::<usize>() { Config.Indent_Width = S; } }
            }
            "-t" | "--tabs" => { Config.Use_Tabs = true; }
            "-l" | "--language" => 
            {
                I += 1;
                if I < Args.len()
                {
                    let LangStr = Args[I].to_lowercase();
                    Config.Language = match LangStr.as_str()
                    {
                        "c" => Source_Language::C, "cpp" | "c++" => Source_Language::Cpp, "cs" | "csharp" | "c#" => Source_Language::CSharp, "js" | "javascript" => Source_Language::JavaScript, "ts" | "typescript" => Source_Language::TypeScript, "sh" | "shell" | "bash" | "zsh" | "ksh" => Source_Language::Shell, "go" => Source_Language::Go, "php" => Source_Language::Php, "html" | "htm" => Source_Language::Html, "css" => Source_Language::Css, _ => Source_Language::Generic, 
                    };
                }
            }
            "--stdin-filepath" => { I += 1; if I < Args.len() { StdinVirtualPath = Some(Args[I].clone()); } }
            "-" => { ReadFromStdin = true; }
            "-h" | "--help" => { Logger.Set_Disable_Logs(true); PrintUsage( & Logger); return ; }
            "-v" | "--version" => { Logger.Set_Disable_Logs(true); Logger.Println("lafs 0.1.0 (Language-Agnostic Formatting Style Formatter)"); return ; }
            _ => 
            {
                if Arg.starts_with("--ext=") || Arg.starts_with("--extension=") || Arg.starts_with("-e=")
                {
                    if let Some((_, Value)) = Arg.split_once('=')
                    {
                        for Part in Value.split(',')
                        {
                            let Clean = Part.trim().trim_start_matches('.').to_lowercase();
                            if ! Clean.is_empty() { AllowedExtensions.push(Clean); }
                        }
                    }
                }
                else if Arg.starts_with("--exclude=") || Arg.starts_with("-x=") || Arg.starts_with("--ignore=")
                {
                    if let Some((_, Value)) = Arg.split_once('=')
                    {
                        for Part in Value.split(',')
                        {
                            let Trimmed = Part.trim();
                            if ! Trimmed.is_empty() { ExcludedPatterns.push(Trimmed.to_string()); }
                        }
                    }
                }
                else if ! Arg.starts_with('-') { RawTargets.push(Arg.clone()); }
            }
        }
        I += 1;
    }

    if OutputToStdout || ReadFromStdin { Logger.Set_Disable_Logs(true); }

    if ReformatFailed
    {
        AutoYes = true;
        let FailedLogPath = OutputFailedPath.clone().unwrap_or_else( || "failed.txt".to_string());
        match fs::read_to_string( & FailedLogPath)
        {
            Ok(Content) => 
            {
                let mut FoundCount = 0;
                for Line in Content.lines()
                {
                    let Trimmed = Line.trim();
                    if Trimmed.is_empty() { continue ; }
                    let FilePath = if let Some(Idx) = Trimmed.rfind(" (")
                    {
                        if Trimmed.ends_with(')') { & Trimmed[..Idx] } else { Trimmed }
                    }
                    else { Trimmed };
                    let CleanPath = FilePath.trim();
                    if ! CleanPath.is_empty()
                    {
                        RawTargets.push(CleanPath.to_string());
                        FoundCount += 1;
                    }
                }
                if FoundCount == 0
                {
                    Logger.Println(& format ! ("No failed files listed in '{}'.", FailedLogPath));
                    Logger.Flush_To_Files();
                    return ;
                }
                OutputFailedPath = Some(FailedLogPath);
            }
            Err(_) => 
            {
                Logger.Eprintln(& format ! ("Failed files list '{}' not found.", FailedLogPath));
                Logger.Flush_To_Files();
                return ;
            }
        }
    }

    // If a language was specified and no explicit extensions were provided, restrict to that language's extensions
    if Config.Language != Source_Language::Generic && AllowedExtensions.is_empty()
    {
        for Ext in Config.Language.DefaultExtensions() { AllowedExtensions.push(Ext.to_string()); }
    }

    AllowedExtensions.sort();
    AllowedExtensions.dedup();
    // Default to current directory if --all is explicitly specified without paths
    if FormatAll && RawTargets.is_empty() { RawTargets.push(".".to_string()); }

    if RawTargets.is_empty() && ! FormatAll && ! ReadFromStdin
    {
        if io::stdin().is_terminal()
        {
            let LangTag = if Config.Language != Source_Language::Generic { format ! ("{{{}}} ", Config.Language.ShortName()) }
            else if ! AllowedExtensions.is_empty() { format ! ("{{{}}} ", AllowedExtensions.join(", ")) } else { String::new () };

            Logger.Print(& format ! ("No target files specified. Format all {}files in current directory and subdirectories? [y/N]: ", LangTag));
            let mut Input = String:: new ();
            if io::stdin().read_line( & mut Input).is_ok()
            {
                let Trimmed = Input.trim().to_lowercase();
                if matches!(Trimmed.as_str(),"y" | "yes") { FormatAll = true; AutoYes = true; RawTargets.push(".".to_string()); }
                else { Logger.Set_Disable_Logs(true); PrintUsage( & Logger); return ; }
            }
            else { Logger.Set_Disable_Logs(true); PrintUsage( & Logger); return ; }
        }
        else { ReadFromStdin = true; }
    }

    if ReadFromStdin
    {
        // Format from stdin
        let mut Buffer = String:: new ();
        if io::stdin().read_to_string( & mut Buffer).is_ok()
        {
            if let Some(VirtPath) = StdinVirtualPath { Config.Language = Source_Language::FromExtension( & VirtPath); }

            match Source_Formatter::Format( & Buffer, & Config)
            {
                Ok(Formatted) => 
                {
                    if CheckOnly { if Formatted != Buffer { std::process::exit(1); } } else { print ! ("{}", Formatted); }
                }
                Err(ErrMessage) => { eprintln ! ("Error formatting stdin: {}", ErrMessage); std::process::exit(1); }
            }
        }
        return ;
    }
    // Expand targets to file list
    let mut TargetFiles: Vec < String > = Vec:: new ();
    for Target in & RawTargets
    {
        if Target.contains('*') || Target.contains('?')
        {
            ExpandGlobPattern(Target, AllTypes, & AllowedExtensions, & ExcludedPatterns, & mut TargetFiles,);
        }
        else
        {
            let P = Path:: new (Target);
            if P.is_dir() { CollectFilesRecursively(P, AllTypes, & AllowedExtensions, & ExcludedPatterns, & mut TargetFiles,); }
            else if P.is_file()
            {
                if ReformatFailed || IsFileEligible(P, AllTypes, & AllowedExtensions, & ExcludedPatterns)
                {
                    TargetFiles.push(P.to_string_lossy().to_string());
                }
            }
            else { Logger.Eprintln(& format ! ("File or directory not found: {}", Target)); }
        }
    }
    // Remove any duplicate file paths
    TargetFiles.sort();
    TargetFiles.dedup();

    let TotalFiles = TargetFiles.len();
    if TotalFiles == 0 { Logger.Println("No matching files found to format."); Logger.Flush_To_Files(); return ; }
    // Prompt user confirmation if formatting directory or multiple files
    let NeedsPrompt = (FormatAll || RawTargets.iter().any( | T | T == "." || Path::new (T).is_dir())) && ! AutoYes && ! CheckOnly && ! OutputToStdout;

    if NeedsPrompt
    {
        let TargetDesc = if RawTargets.len() == 1 { & RawTargets[0] } else { "selected targets" };
        let PromptDesc = if ! AllowedExtensions.is_empty() { format ! ("{} [ext: {}]", TargetDesc, AllowedExtensions.join(", ")) }
        else { TargetDesc.to_string() };

        if ! PromptConfirmation( & Logger, TotalFiles, & PromptDesc) { Logger.Println("Operation cancelled by user."); Logger.Flush_To_Files(); return ; }
    }

    let FormattedCount = Arc:: new (AtomicUsize::new (0));
    let CleanCount = Arc:: new (AtomicUsize::new (0));
    let ErrorCount = Arc:: new (AtomicUsize::new (0));
    let CurrentIndex = Arc:: new (AtomicUsize::new (0));
    let FailedPaths = Arc:: new (Mutex::new (Vec::new ()));

    let StartTime = Instant::now();
    let ThreadCount = if OutputToStdout || CheckOnly || TotalFiles <= 1 { 1 }
    else { CustomThreads.unwrap_or_else( || std::thread::available_parallelism().map_or(4, | n | n.get ())) };

    if ThreadCount <= 1
    {
        for FilePathStr in & TargetFiles
        {
            let FilePath = Path:: new (FilePathStr);
            let FileContent = match fs::read_to_string(FilePath)
            {
                Ok(C) => C, Err(E) => 
                {
                    if E.kind() == io::ErrorKind::InvalidData { if Verbose { Logger.Println(& format ! ("Skipped non-UTF8 / binary file: {}", FilePathStr)); } continue ; }
                    Logger.Eprintln(& format ! ("Failed to read file {}: {}", FilePathStr, E));
                    ErrorCount.fetch_add(1, Ordering::Relaxed);
                    FailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                    continue ;
                }
            };

            let mut FileConfig = Config.clone();
            if FileConfig.Language == Source_Language::Generic { FileConfig.Language = Source_Language::FromExtension(FilePathStr); }

            match Source_Formatter::Format( & FileContent, & FileConfig)
            {
                Ok(Formatted) => 
                {
                    let IsAlreadyFormatted = Formatted == FileContent;

                    if CheckOnly
                    {
                        if ! IsAlreadyFormatted
                        {
                            Logger.Println(& format ! ("Requires formatting: {}", FilePathStr));
                            ErrorCount.fetch_add(1, Ordering::Relaxed);
                            FailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                        }
                        else if Verbose { Logger.Println(& format ! ("Clean: {}", FilePathStr)); }
                    }
                    else if OutputToStdout { print ! ("{}", Formatted); }
                    else if ! IsAlreadyFormatted
                    {
                        if let Err(E) = fs::write(FilePath, & Formatted)
                        {
                            Logger.Eprintln(& format ! ("Failed to save file {}: {}", FilePathStr, E));
                            ErrorCount.fetch_add(1, Ordering::Relaxed);
                            FailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                        }
                        else { Logger.Println(& format ! ("Formatted & Saved: {}", FilePathStr)); FormattedCount.fetch_add(1, Ordering::Relaxed); }
                    }
                    else { if Verbose { Logger.Println(& format ! ("Clean (Unchanged): {}", FilePathStr)); } CleanCount.fetch_add(1, Ordering::Relaxed); }
                }
                Err(ErrMessage) => 
                {
                    Logger.Eprintln(& format ! ("Error formatting {}: {}", FilePathStr, ErrMessage));
                    ErrorCount.fetch_add(1, Ordering::Relaxed);
                    FailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                }
            }
        }
    }
    else
    {
        std::thread::scope
        (
            | Scope | 
            {
                for _ in 0..ThreadCount
                {
                    let WorkerLogger = Logger.clone();
                    let WorkerFailedPaths = FailedPaths.clone();
                    let WorkerFormattedCount = FormattedCount.clone();
                    let WorkerCleanCount = CleanCount.clone();
                    let WorkerErrorCount = ErrorCount.clone();
                    let WorkerCurrentIndex = CurrentIndex.clone();
                    let WorkerConfig = Config.clone();
                    let WorkerTargetFiles = & TargetFiles;

                    Scope.spawn
                    (
                        move || loop 
                        {
                            let Idx = WorkerCurrentIndex.fetch_add(1, Ordering::Relaxed);
                            if Idx >= TotalFiles { break ; }
                            let FilePathStr = & WorkerTargetFiles[Idx];
                            let FilePath = Path:: new (FilePathStr);
                            let FileContent = match fs::read_to_string(FilePath)
                            {
                                Ok(C) => C, Err(E) => 
                                {
                                    if E.kind() == io::ErrorKind::InvalidData
                                    {
                                        if Verbose { WorkerLogger.Println(& format ! ("Skipped non-UTF8 / binary file: {}", FilePathStr)); }
                                        continue ;
                                    }
                                    WorkerLogger.Eprintln(& format ! ("Failed to read file {}: {}", FilePathStr, E));
                                    WorkerErrorCount.fetch_add(1, Ordering::Relaxed);
                                    WorkerFailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                                    continue ;
                                }
                            };
                            let mut FileConfig = WorkerConfig.clone();
                            if FileConfig.Language == Source_Language::Generic { FileConfig.Language = Source_Language::FromExtension(FilePathStr); }
                            match Source_Formatter::Format( & FileContent, & FileConfig)
                            {
                                Ok(Formatted) => 
                                {
                                    if Formatted != FileContent
                                    {
                                        if let Err(E) = fs::write(FilePath, & Formatted)
                                        {
                                            WorkerLogger.Eprintln(& format ! ("Failed to save file {}: {}", FilePathStr, E));
                                            WorkerErrorCount.fetch_add(1, Ordering::Relaxed);
                                            WorkerFailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                                        }
                                        else { WorkerLogger.Println(& format ! ("Formatted & Saved: {}", FilePathStr)); WorkerFormattedCount.fetch_add(1, Ordering::Relaxed); }
                                    }
                                    else
                                    {
                                        if Verbose { WorkerLogger.Println(& format ! ("Clean (Unchanged): {}", FilePathStr)); }
                                        WorkerCleanCount.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                                Err(ErrMessage) => 
                                {
                                    WorkerLogger.Eprintln(& format ! ("Error formatting {}: {}", FilePathStr, ErrMessage));
                                    WorkerErrorCount.fetch_add(1, Ordering::Relaxed);
                                    WorkerFailedPaths.lock().unwrap().push(format ! ("{} ({})", FilePathStr, Source_Language::FromExtension(FilePathStr).ShortName()));
                                }
                            }
                        }
                    );
                }
            }
        );
    }

    let Elapsed = StartTime.elapsed();
    let FCount = FormattedCount.load(Ordering::Relaxed);
    let CCount = CleanCount.load(Ordering::Relaxed);
    let ECount = ErrorCount.load(Ordering::Relaxed);

    if ! OutputToStdout && ! CheckOnly
    {
        Logger.Println(& format ! ("Summary: {} formatted and saved, {} already clean, {} errors ({:.2?}, {} threads).", FCount, CCount, ECount, Elapsed, ThreadCount));
    }

    if OutputFailedPath.is_none() && ECount > 0 && ! OutputToStdout && ! CheckOnly
    {
        OutputFailedPath = Some("failed.txt".to_string());
    }

    if let Some(FailedFile) = OutputFailedPath
    {
        let mut FailedPathsStr = String:: new ();
        let FailedMutex = FailedPaths.lock().unwrap();
        for Path in FailedMutex.iter() { FailedPathsStr.push_str(Path); FailedPathsStr.push('\n'); }
        if let Err(E) = fs::write( & FailedFile, & FailedPathsStr)
        {
            Logger.Eprintln(& format ! ("Failed to write failed paths to {}: {}", FailedFile, E));
        }
        else if ReformatFailed && FailedMutex.is_empty()
        {
            Logger.Println("All previously failed files were successfully formatted!");
        }
        else if ! FailedMutex.is_empty()
        {
            Logger.Println(& format ! ("Failed file paths recorded to '{}'. Use 'lafs format failed' to retry.", FailedFile));
        }
    }

    Logger.Flush_To_Files();

    if CheckOnly && ECount > 0 { std::process::exit(1); }
}

fn main() { Main(); }
#[cfg(test)]
mod Tests
{
    use super:: * ;
    use std::path::Path;

    # [test] fn Test_Wildcard_Matching()
    {
        assert!(WildcardMatch("*.c", "main.c"));
        assert!(WildcardMatch("*.cpp", "test.cpp"));
        assert!(WildcardMatch("test_*.rs", "test_lexer.rs"));
        assert!( ! WildcardMatch("*.c", "main.rs"));
        assert!(WildcardMatch("vendor", "vendor"));
        assert!(WildcardMatch("target*", "target_output"));
        assert!(WildcardMatch("?at", "cat"));
        assert!(WildcardMatch("?at", "bat"));
        assert!( ! WildcardMatch("?at", "chat"));
    }

    # [test] fn Test_Document_Extension_Filter()
    {
        let EmptyExts: Vec < String > = Vec:: new ();
        let Excludes: Vec < String > = Vec:: new ();
        assert!( ! IsFileEligible(Path::new ("doc.adoc"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("doc.asciidoc"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("paper.pdf"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("notes.docx"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("notes.doc"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("README.md"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("guide.markdown"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("manual.rst"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("todo.txt"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("workflow.yml"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("config.yaml"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("workflow.yml"), false, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("config.yaml"), false, & EmptyExts, & Excludes));
    }

    # [test] fn Test_Binary_Extension_Filter()
    {
        let EmptyExts: Vec < String > = Vec:: new ();
        let Excludes: Vec < String > = Vec:: new ();
        assert!( ! IsFileEligible(Path::new ("app.exe"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("lib.dll"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("archive.zip"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("photo.png"), true, & EmptyExts, & Excludes));
        assert!( ! IsFileEligible(Path::new ("video.mp4"), true, & EmptyExts, & Excludes));
    }

    # [test] fn Test_Directory_Exclusion()
    {
        let CustomExcludes = vec!["vendor".to_string(),"build*".to_string(),"cache".to_string(),];
        // Default exclusions
        assert!(IsDirExcluded(".git", Path::new (".git"), & CustomExcludes));
        assert!(IsDirExcluded("target", Path::new ("target"), & CustomExcludes));
        assert!(IsDirExcluded("node_modules", Path::new ("node_modules"), & CustomExcludes));
        // Custom exclusions
        assert!(IsDirExcluded("vendor", Path::new ("vendor"), & CustomExcludes));
        assert!(IsDirExcluded("build_debug", Path::new ("build_debug"), & CustomExcludes));
        assert!(IsDirExcluded("cache", Path::new ("deeply/nested/cache"), & CustomExcludes));
        // Non-excluded
        assert!( ! IsDirExcluded("src", Path::new ("src"), & CustomExcludes));
        assert!( ! IsDirExcluded("examples", Path::new ("lafs/examples"), & CustomExcludes));
    }

    # [test] fn Test_File_Exclusion_Patterns()
    {
        let EmptyExts: Vec < String > = Vec:: new ();
        let CustomExcludes = vec!["*.min.js".to_string(),"generated_*".to_string(),"secret.key".to_string(),];

        assert!( ! IsFileEligible(Path::new ("bundle.min.js"), true, & EmptyExts, & CustomExcludes));
        assert!( ! IsFileEligible(Path::new ("generated_code.c"), true, & EmptyExts, & CustomExcludes));
        assert!( ! IsFileEligible(Path::new ("secret.key"), true, & EmptyExts, & CustomExcludes));
        // Allowed
        assert!(IsFileEligible(Path::new ("bundle.js"), true, & EmptyExts, & CustomExcludes));
        assert!(IsFileEligible(Path::new ("normal_code.c"), false, & EmptyExts, & CustomExcludes));
    }

    # [test] fn Test_Allowed_Extension_Filter()
    {
        let EmptyExcludes: Vec < String > = Vec:: new ();
        let AllowedC = vec!["c".to_string(),"h".to_string()];
        let AllowedSh = vec!["sh".to_string()];

        assert!(IsFileEligible(Path::new ("main.c"), false, & AllowedC, & EmptyExcludes));
        assert!(IsFileEligible(Path::new ("header.h"), false, & AllowedC, & EmptyExcludes));
        assert!( ! IsFileEligible(Path::new ("source.cpp"), false, & AllowedC, & EmptyExcludes));
        assert!( ! IsFileEligible(Path::new ("script.sh"), false, & AllowedC, & EmptyExcludes));
        assert!( ! IsFileEligible(Path::new ("app.cs"), false, & AllowedC, & EmptyExcludes));
        assert!( ! IsFileEligible(Path::new ("workflow.yml"), false, & AllowedC, & EmptyExcludes));

        assert!(IsFileEligible(Path::new ("deploy.sh"), false, & AllowedSh, & EmptyExcludes));
        assert!( ! IsFileEligible(Path::new ("main.c"), false, & AllowedSh, & EmptyExcludes));
    }
}


