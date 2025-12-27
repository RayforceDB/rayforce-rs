/*
*   Copyright (c) 2025 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

//! Rayforce REPL (Read-Eval-Print Loop) with rich terminal features.
//!
//! Features:
//! - Syntax highlighting
//! - Auto-completion for Rayforce keywords and functions
//! - Command history (persistent across sessions)
//! - Multi-line editing
//! - Unicode support
//! - Beautiful colors
//!
//! Run with: `cargo run --example repl`

use nu_ansi_term::{Color, Style};
use rayforce::{Rayforce, Result};
use reedline::{
    Completer, Emacs, FileBackedHistory,
    Highlighter, Hinter, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode,
    PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu,
    Signal, Span, StyledText, Suggestion, ValidationResult, Validator,
    ColumnarMenu,
};
use std::borrow::Cow;
use std::collections::HashSet;

// ════════════════════════════════════════════════════════════════════════════════
// Rayforce Keywords and Built-in Functions
// ════════════════════════════════════════════════════════════════════════════════

const KEYWORDS: &[&str] = &[
    // Control flow
    "if", "do", "while", "each", "over", "scan", "peach",
    // Definitions
    "def", "let", "set", "get",
    // Query
    "select", "update", "insert", "upsert", "delete", "exec", "from", "where", "by",
    // Joins
    "inner-join", "left-join", "right-join", "window-join", "asof-join",
];

const FUNCTIONS: &[&str] = &[
    // Arithmetic
    "+", "-", "*", "/", "%", "neg", "abs", "sqrt", "exp", "log", "pow",
    "sin", "cos", "tan", "asin", "acos", "atan",
    // Comparison
    "=", "==", "!=", "<>", "<", ">", "<=", ">=", "~", "like", "match",
    // Logical
    "and", "or", "not", "any", "all",
    // Aggregation
    "sum", "avg", "min", "max", "count", "first", "last", "med", "dev", "var",
    "prd", "sums", "prds", "mins", "maxs", "avgs", "devs", "vars",
    // Sort & Search
    "asc", "desc", "iasc", "idesc", "xasc", "xdesc", "rank", "bin", "binr",
    "distinct", "group", "ungroup", "flip", "rotate",
    // List operations
    "til", "enlist", "raze", "reverse", "cross", "vs", "sv",
    "take", "drop", "cut", "sublist", "inter", "union", "except",
    // String operations
    "lower", "upper", "trim", "ltrim", "rtrim", "ssr", "ss",
    // Type operations
    "type", "null", "count", "cols", "keys", "value", "meta",
    "string", "symbol", "`int", "`long", "`float", "`date", "`time", "`timestamp",
    // Table operations
    "table", "dict", "xkey", "xcol", "xcols", "meta",
    // I/O
    "read", "write", "load", "save", "hopen", "hclose", "read-csv",
    // System
    "show", "exit", "system", "getenv", "setenv",
    // Math
    "floor", "ceil", "round", "signum", "reciprocal",
    // Date/Time
    "date", "time", "timestamp", "year", "month", "day", "hour", "minute", "second",
    // Special
    "eval", "parse", "value", "quote", "list", "concat", "at", "map", "map-left",
];

const COMMANDS: &[&str] = &[
    ":?", ":q", ":t", ":v", ":c",
];

// ════════════════════════════════════════════════════════════════════════════════
// Custom Prompt
// ════════════════════════════════════════════════════════════════════════════════

struct RayPrompt;

impl RayPrompt {
    fn new() -> Self {
        Self
    }
}

impl Prompt for RayPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Owned(format!("{}", Color::Green.paint("❯ ")))
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{}", Color::DarkGray.paint("… ")))
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => Color::Green.paint("◉"),
            PromptHistorySearchStatus::Failing => Color::Red.paint("◉"),
        };
        Cow::Owned(format!(
            "{}{}{}{}",
            prefix,
            Color::Cyan.paint(" search: "),
            Color::White.bold().paint(&history_search.term),
            Color::DarkGray.paint(" │ ")
        ))
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// Syntax Highlighter
// ════════════════════════════════════════════════════════════════════════════════

struct RayHighlighter {
    keywords: HashSet<String>,
    functions: HashSet<String>,
    commands: HashSet<String>,
}

impl RayHighlighter {
    fn new() -> Self {
        Self {
            keywords: KEYWORDS.iter().map(|s| s.to_string()).collect(),
            functions: FUNCTIONS.iter().map(|s| s.to_string()).collect(),
            commands: COMMANDS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Highlighter for RayHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        
        // Simple tokenization for highlighting
        let mut chars = line.chars().peekable();
        let mut current_token = String::new();
        let mut in_string = false;
        let mut string_char = '"';
        
        while let Some(c) = chars.next() {
            if in_string {
                current_token.push(c);
                if c == string_char {
                    styled.push((Style::new().fg(Color::Green), current_token.clone()));
                    current_token.clear();
                    in_string = false;
                }
            } else if c == '"' || c == '\'' {
                // Flush current token
                if !current_token.is_empty() {
                    styled.push(self.style_token(&current_token));
                    current_token.clear();
                }
                in_string = true;
                string_char = c;
                current_token.push(c);
            } else if c == '`' {
                // Symbol literal
                if !current_token.is_empty() {
                    styled.push(self.style_token(&current_token));
                    current_token.clear();
                }
                current_token.push(c);
                // Read until whitespace or delimiter
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || "()[]{}".contains(next) {
                        break;
                    }
                    current_token.push(chars.next().unwrap());
                }
                styled.push((Style::new().fg(Color::Magenta), current_token.clone()));
                current_token.clear();
            } else if c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' {
                // Flush current token
                if !current_token.is_empty() {
                    styled.push(self.style_token(&current_token));
                    current_token.clear();
                }
                styled.push((Style::new().fg(Color::Yellow).bold(), c.to_string()));
            } else if c.is_whitespace() {
                // Flush current token
                if !current_token.is_empty() {
                    styled.push(self.style_token(&current_token));
                    current_token.clear();
                }
                styled.push((Style::new(), c.to_string()));
            } else if c == ';' {
                // Comment
                if !current_token.is_empty() {
                    styled.push(self.style_token(&current_token));
                    current_token.clear();
                }
                let mut comment = c.to_string();
                for remaining in chars.by_ref() {
                    comment.push(remaining);
                }
                styled.push((Style::new().fg(Color::DarkGray).italic(), comment));
            } else {
                current_token.push(c);
            }
        }
        
        // Handle remaining content
        if in_string {
            styled.push((Style::new().fg(Color::Green), current_token));
        } else if !current_token.is_empty() {
            styled.push(self.style_token(&current_token));
        }
        
        styled
    }
}

impl RayHighlighter {
    fn style_token(&self, token: &str) -> (Style, String) {
        if self.commands.contains(token) {
            (Style::new().fg(Color::DarkGray), token.to_string())
        } else if self.keywords.contains(token) {
            (Style::new().fg(Color::Green), token.to_string())
        } else if self.functions.contains(token) {
            (Style::new().fg(Color::Green), token.to_string())
        } else if token.parse::<f64>().is_ok() || token.ends_with('i') || token.ends_with('f') {
            (Style::new().fg(Color::Cyan), token.to_string())
        } else if token.starts_with(':') {
            (Style::new().fg(Color::DarkGray), token.to_string())
        } else {
            (Style::new().fg(Color::White), token.to_string())
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// Auto-completion
// ════════════════════════════════════════════════════════════════════════════════

struct RayCompleter {
    suggestions: Vec<String>,
}

impl RayCompleter {
    fn new() -> Self {
        let mut suggestions: Vec<String> = Vec::new();
        suggestions.extend(KEYWORDS.iter().map(|s| s.to_string()));
        suggestions.extend(FUNCTIONS.iter().map(|s| s.to_string()));
        suggestions.extend(COMMANDS.iter().map(|s| s.to_string()));
        suggestions.sort();
        Self { suggestions }
    }
}

impl Completer for RayCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // Find the start of the current word
        let line_to_pos = &line[..pos];
        let word_start = line_to_pos
            .rfind(|c: char| c.is_whitespace() || "()[]{}".contains(c))
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let current_word = &line[word_start..pos];
        
        if current_word.is_empty() {
            return Vec::new();
        }
        
        self.suggestions
            .iter()
            .filter(|s| s.starts_with(current_word))
            .map(|s| Suggestion {
                value: s.clone(),
                description: Some(self.get_description(s)),
                style: None,
                extra: None,
                span: Span::new(word_start, pos),
                append_whitespace: true,
            })
            .collect()
    }
}

impl RayCompleter {
    fn get_description(&self, item: &str) -> String {
        match item {
            // Commands
            ":?" => "Show help".to_string(),
            ":q" => "Exit REPL".to_string(),
            ":t" => "Toggle timeit".to_string(),
            ":v" => "Show version".to_string(),
            ":c" => "Clear screen".to_string(),
            
            // Keywords
            "if" => "Conditional expression".to_string(),
            "do" => "Execute block".to_string(),
            "while" => "While loop".to_string(),
            "each" => "Apply to each element".to_string(),
            "select" => "Query: select columns".to_string(),
            "update" => "Query: update rows".to_string(),
            "insert" => "Query: insert rows".to_string(),
            "delete" => "Query: delete rows".to_string(),
            "from" => "Query: source table".to_string(),
            "where" => "Query: filter condition".to_string(),
            "by" => "Query: group by".to_string(),
            
            // Aggregations
            "sum" => "∑ Sum of values".to_string(),
            "avg" => "μ Average of values".to_string(),
            "min" => "↓ Minimum value".to_string(),
            "max" => "↑ Maximum value".to_string(),
            "count" => "# Count elements".to_string(),
            "first" => "⊢ First element".to_string(),
            "last" => "⊣ Last element".to_string(),
            "med" => "◊ Median value".to_string(),
            "dev" => "σ Standard deviation".to_string(),
            
            // Sort
            "asc" => "↑ Sort ascending".to_string(),
            "desc" => "↓ Sort descending".to_string(),
            "distinct" => "◇ Unique values".to_string(),
            
            // List
            "til" => "⍳ Generate 0..n-1".to_string(),
            "reverse" => "⌽ Reverse list".to_string(),
            "take" => "↑ Take n elements".to_string(),
            "drop" => "↓ Drop n elements".to_string(),
            
            // Table
            "table" => "⊞ Create table".to_string(),
            "dict" => "⊟ Create dictionary".to_string(),
            "cols" => "Column names".to_string(),
            "keys" => "Dictionary keys".to_string(),
            "meta" => "Table metadata".to_string(),
            
            // I/O
            "read-csv" => "Read CSV file".to_string(),
            "hopen" => "Open connection".to_string(),
            "hclose" => "Close connection".to_string(),
            "show" => "Display value".to_string(),
            
            _ => String::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// Validator (for multi-line input)
// ════════════════════════════════════════════════════════════════════════════════

struct RayValidator;

impl Validator for RayValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let mut parens = 0i32;
        let mut brackets = 0i32;
        let mut braces = 0i32;
        let mut in_string = false;
        let mut escape_next = false;
        
        for c in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match c {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '(' if !in_string => parens += 1,
                ')' if !in_string => parens -= 1,
                '[' if !in_string => brackets += 1,
                ']' if !in_string => brackets -= 1,
                '{' if !in_string => braces += 1,
                '}' if !in_string => braces -= 1,
                _ => {}
            }
        }
        
        if in_string || parens > 0 || brackets > 0 || braces > 0 {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// Hinter (inline suggestions)
// ════════════════════════════════════════════════════════════════════════════════

struct RayHinter {
    completer: RayCompleter,
}

impl RayHinter {
    fn new() -> Self {
        Self {
            completer: RayCompleter::new(),
        }
    }
}

impl Hinter for RayHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        _history: &dyn reedline::History,
        use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        let suggestions = self.completer.complete(line, pos);
        
        if let Some(first) = suggestions.first() {
            let hint = &first.value[pos - first.span.start..];
            if use_ansi_coloring {
                Color::DarkGray.italic().paint(hint).to_string()
            } else {
                hint.to_string()
            }
        } else {
            String::new()
        }
    }

    fn complete_hint(&self) -> String {
        String::new()
    }

    fn next_hint_token(&self) -> String {
        String::new()
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// Banner & Help
// ════════════════════════════════════════════════════════════════════════════════

fn print_banner(version: u8) {
    let bold = Style::new().bold();
    
    // Get system info
    let cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    
    // Get current working directory
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    
    // Get build date (compile time)
    let build_date = env!("CARGO_PKG_VERSION");
    
    println!("{}  Rayforce-rs: {} ({})", bold.paint(""), version, build_date);
    println!("  {} core(s)", cores);
    println!("  Started from: {}", cwd);
    println!("  Documentation: https://rayforcedb.com/");
    println!("  Github: https://github.com/singaraiona/rayforce-rs");
}

fn print_help() {
    let cyan = Color::Cyan;
    let yellow = Color::Yellow;
    let green = Color::Green;
    let dim = Color::DarkGray;
    
    println!();
    println!("  {} {}", cyan.bold().paint("⚡"), cyan.bold().paint("Commands"));
    println!("  {}", dim.paint("─".repeat(50)));
    println!("    {}  {}       {}", cyan.paint(":?"), dim.paint("│"), "Show this help");
    println!("    {}  {}       {}", cyan.paint(":q"), dim.paint("│"), "Quit REPL");
    println!("    {}  {}       {}", cyan.paint(":t"), dim.paint("│"), "Toggle timeit on|off");
    println!("    {}  {}       {}", cyan.paint(":v"), dim.paint("│"), "Show version");
    println!("    {}  {}       {}", cyan.paint(":c"), dim.paint("│"), "Clear screen");
    println!();
    
    println!("  {} {}", yellow.bold().paint("📝"), yellow.bold().paint("Examples"));
    println!("  {}", dim.paint("─".repeat(50)));
    println!("    {}         {}", green.paint("42"), "Integer");
    println!("    {}       {}", green.paint("3.14"), "Float");
    println!("    {}   {}", green.paint("\"hello\""), "String");
    println!("    {}    {}", green.paint("`symbol"), "Symbol");
    println!("    {}  {}", green.paint("1 2 3 4 5"), "Vector");
    println!("    {}   {}", green.paint("(+ 1 2)"), "Addition → 3");
    println!("    {}   {}", green.paint("(* 3 4)"), "Multiplication → 12");
    println!("    {}   {}", green.paint("(til 5)"), "Generate → [0 1 2 3 4]");
    println!("    {}  {}", green.paint("(sum 1 2 3 4 5)"), "Sum → 15");
    println!();
    
    println!("  {} {}", Color::Blue.bold().paint("⌨"), Color::Blue.bold().paint("Keyboard Shortcuts"));
    println!("  {}", dim.paint("─".repeat(50)));
    println!("    {}     {}", dim.paint("Tab"), "Auto-complete");
    println!("    {}  {}", dim.paint("Ctrl+R"), "Search history");
    println!("    {}  {}", dim.paint("Ctrl+C"), "Cancel line");
    println!("    {}  {}", dim.paint("Ctrl+D"), "Exit");
    println!("    {}    {}", dim.paint("↑ / ↓"), "Navigate history");
    println!();
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

// ════════════════════════════════════════════════════════════════════════════════
// Main
// ════════════════════════════════════════════════════════════════════════════════

fn main() -> Result<()> {
    // Initialize Rayforce runtime
    let rf = Rayforce::new()?;
    let version = rf.version();
    
    print_banner(version);
    
    // Setup history file
    let history_path = dirs_next::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rayforce")
        .join("history.txt");
    
    // Create directory if it doesn't exist
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // Setup completion menu
    let completer = Box::new(RayCompleter::new());
    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_text_style(Style::new().fg(Color::White))
            .with_selected_text_style(Style::new().fg(Color::Black).on(Color::Cyan))
            .with_description_text_style(Style::new().fg(Color::DarkGray).italic())
    );
    
    // Setup keybindings
    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    
    // Build the editor
    let edit_mode = Box::new(Emacs::new(keybindings));
    
    let mut line_editor = Reedline::create()
        .with_history(Box::new(
            FileBackedHistory::with_file(1000, history_path)
                .expect("Failed to create history"),
        ))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_highlighter(Box::new(RayHighlighter::new()))
        .with_hinter(Box::new(RayHinter::new()))
        .with_validator(Box::new(RayValidator))
        .with_edit_mode(edit_mode)
        .with_quick_completions(true)
        .with_partial_completions(true);
    
    let prompt = RayPrompt::new();
    
    loop {
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                let input = line.trim();
                
                if input.is_empty() {
                    continue;
                }
                
                // Handle commands (start with :)
                match input {
                    ":q" => {
                        println!("\n  {} {}\n", 
                            Color::Cyan.paint("👋"),
                            Color::DarkGray.paint("Goodbye!"));
                        break;
                    }
                    ":?" => {
                        print_help();
                        continue;
                    }
                    ":t" | ":t 1" => {
                        println!("  {} Timeit is {}",
                            Color::Yellow.paint("."),
                            Color::Green.paint("on"));
                        continue;
                    }
                    ":t 0" => {
                        println!("  {} Timeit is {}",
                            Color::Yellow.paint("."),
                            Color::DarkGray.paint("off"));
                        continue;
                    }
                    ":v" => {
                        println!("  {} Rayforce version: {}", 
                            Color::Cyan.paint("ℹ"),
                            Color::Yellow.bold().paint(format!("{}", version)));
                        continue;
                    }
                    ":c" => {
                        clear_screen();
                        print_banner(version);
                        continue;
                    }
                    _ if input.starts_with(':') && input.len() <= 4 => {
                        println!("  {} Unknown command: {}. Type {} for help.",
                            Color::Red.paint("✗"),
                            Color::Yellow.paint(input),
                            Color::Cyan.paint(":?"));
                        continue;
                    }
                    _ => {}
                }
                
                // Evaluate the expression
                match rf.eval(input) {
                    Ok(result) => {
                        println!("{}", Color::Green.paint(format!("{}", result)));
                    }
                    Err(e) => {
                        println!("  {} {}", 
                            Color::Red.bold().paint("Error:"),
                            Color::Red.paint(format!("{}", e)));
                    }
                }
            }
            Ok(Signal::CtrlC) => {
                println!("  {}", Color::DarkGray.paint("^C"));
            }
            Ok(Signal::CtrlD) => {
                println!("\n  {} {}\n",
                    Color::Cyan.paint("👋"),
                    Color::DarkGray.paint("Goodbye!"));
                break;
            }
            Err(err) => {
                eprintln!("  {} {}",
                    Color::Red.bold().paint("Error:"),
                    Color::Red.paint(format!("{}", err)));
            }
        }
    }
    
    Ok(())
}
