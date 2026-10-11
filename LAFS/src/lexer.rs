#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)] use crate::types:: { Container_Kind, Source_Language, Token_Info, Token_Kind };

pub struct Lexer_Scanner<'a> { Source: & 'a str, Chars: Vec<char>, Index: usize, Line: usize, Column: usize, Language: Source_Language, }

impl <'a> Lexer_Scanner<'a>
{
    pub fn New(Source: & 'a str, Language: Source_Language)->Self
    {
        Lexer_Scanner { Source, Chars: Source.chars().collect(), Index: 0, Line: 1, Column: 0, Language, }
    }

    fn Peek( & self)->Option<char> { if self.Index < self.Chars.len() { Some(self.Chars[self.Index]) } else { None } }

    fn PeekAhead( & self, Offset: usize)->Option<char>
    {
        let Target = self.Index + Offset;
        if Target < self.Chars.len() { Some(self.Chars[Target]) } else { None }
    }

    fn Advance( & mut self)->Option<char>
    {
        if self.Index >= self.Chars.len() { return None; }
        let Ch = self.Chars[self.Index];
        self.Index += 1;
        if Ch == '\n' { self.Line += 1; self.Column = 0; } else { self.Column += 1; }
        Some(Ch)
    }

    pub fn TokenizeAll( & mut self)->Vec<Token_Info>
    {
        let mut RawTokens: Vec < Token_Info > = Vec:: new ();
        let mut PrecedingNewlines: usize = 0;

        while let Some(Ch) = self.Peek()
        {
            let StartLine = self.Line;
            let StartCol = self.Column;
            // 1. Newline
            if Ch == '\n' { self.Advance(); PrecedingNewlines += 1; continue ; }
            // 2. Horizontal Whitespace
            if Ch == ' ' || Ch == '\t' || Ch == '\r' { self.Advance(); continue ; }
            // 3. Preprocessor Directive
            if Ch == '#' && StartCol == 0
            {
                let DirectiveText = self.ScanPreprocessor();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Preprocessor(DirectiveText.clone()), DirectiveText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }
            // 4. Comments
            if Ch == '/' && self.PeekAhead(1) == Some('/')
            {
                let CommentText = self.ScanLineComment();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Comment_Line(CommentText.clone()), CommentText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '/' && self.PeekAhead(1) == Some('*')
            {
                let CommentText = self.ScanBlockComment();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Comment_Block(CommentText.clone()), CommentText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }
            // 5. Raw String / Verbatim String / Char Literals
            if (Ch == 'R' || Ch == 'L' || Ch == 'u' || Ch == 'U') && self.PeekAhead(1) == Some('"') && self.Language == Source_Language::Cpp
            {
                let StringText = self.ScanCppRawString();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '@' && self.PeekAhead(1) == Some('"') && self.Language == Source_Language::CSharp
            {
                let StringText = self.ScanCSharpVerbatimString();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if (Ch == '$' || Ch == '@') && self.PeekAhead(1) == Some('@') && self.PeekAhead(2) == Some('"') && self.Language == Source_Language::CSharp
            {
                self.Advance();
                // $
                self.Advance();
                // @
                let StringText = self.ScanCSharpVerbatimString();
                let Full = format!("$@{}", StringText);
                RawTokens.push(Token_Info::New(Token_Kind::String_Lit(Full.clone()), Full, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '"'
            {
                if self.PeekAhead(1) == Some('"') && self.PeekAhead(2) == Some('"') && self.Language == Source_Language::CSharp
                {
                    let StringText = self.ScanCSharpRawString();
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                    );
                }
                else
                {
                    let StringText = self.ScanStandardString();
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                    );
                }
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '`'
            {
                let StringText = self.ScanRawBacktickString();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '\''
            {
                if let Some(NextCh) = self.PeekAhead(1)
                {
                    if (NextCh.is_alphabetic() || NextCh == '_') && self.PeekAhead(2) != Some('\'')
                    {
                        // It's a Rust lifetime!
                        self.Advance();
                        // consume '\''
                        let IdentText = self.ScanIdentifier();
                        let LifetimeText = format!("'{}", IdentText);
                        RawTokens.push
                        (
                            Token_Info::New(Token_Kind::Identifier(LifetimeText.clone()), LifetimeText, StartLine, StartCol, PrecedingNewlines,)
                        );
                        PrecedingNewlines = 0;
                        continue;
                    }
                }

                let CharText = self.ScanCharLiteral();
                RawTokens.push(Token_Info::New(Token_Kind::Char_Lit(CharText.clone()), CharText, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }
            // 6. Delimiters
            if Ch == '('
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Open_Delim(Container_Kind::Paren),"(".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == ')'
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Close_Delim(Container_Kind::Paren),")".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '{'
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Open_Delim(Container_Kind::Brace),"{".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '}'
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Close_Delim(Container_Kind::Brace),"}".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '['
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Open_Delim(Container_Kind::Bracket),"[".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == ']'
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Close_Delim(Container_Kind::Bracket),"]".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                continue;
            }
            // 7. Numbers
            if Ch.is_ascii_digit() || (Ch == '.' && self.PeekAhead(1).map_or(false, | C | C.is_ascii_digit()))
            {
                let NumberText = self.ScanNumber();
                RawTokens.push(Token_Info::New(Token_Kind::Number(NumberText.clone()), NumberText, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }
            // Check for Rust raw string literal
            if Ch == 'r'
            {
                let mut HashCount = 0;
                let mut Lookahead = 1;
                while let Some(c) = self.PeekAhead(Lookahead)
                {
                    if c == '#' { HashCount += 1; Lookahead += 1; }
                    else { break; }
                }
                if let Some(c) = self.PeekAhead(Lookahead)
                {
                    if c == '"'
                    {
                        let RawStringText = self.ScanRawRustString(HashCount);
                        RawTokens.push(Token_Info::New(Token_Kind::String_Lit(RawStringText.clone()), RawStringText, StartLine, StartCol, PrecedingNewlines,));
                        PrecedingNewlines = 0;
                        continue;
                    }
                }
            }
            // 8. Identifiers and Keywords
              if Ch.is_alphabetic() || Ch == '_' || Ch == '$' || (self.Language == Source_Language::Css && (Ch == '@' || Ch == '#' || Ch == '.'))
            {
                let IdentText = self.ScanIdentifier();
                let Kind = if IsKeyword( & IdentText) { Token_Kind::Keyword(IdentText.clone()) } else { Token_Kind::Identifier(IdentText.clone()) };
                RawTokens.push(Token_Info::New(Kind, IdentText, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }
            // 9. Punctuation and compound access operators
            if Ch == ';' || Ch == ',' || Ch == '.' || Ch == '?' || Ch == ':'
            {
                if Ch == ':' && self.PeekAhead(1) == Some(':')
                {
                    self.Advance();
                    self.Advance();
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::Operator("::".to_string()),"::".to_string(), StartLine, StartCol, PrecedingNewlines,)
                    );
                    PrecedingNewlines = 0;
                    continue;
                }
                if Ch == '.' && self.PeekAhead(1) == Some('.') && self.PeekAhead(2) == Some('.')
                {
                    self.Advance();
                    self.Advance();
                    self.Advance();
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::Operator("...".to_string()),"...".to_string(), StartLine, StartCol, PrecedingNewlines,)
                    );
                    PrecedingNewlines = 0;
                    continue;
                }
                if Ch == '?' && self.PeekAhead(1) == Some('.')
                {
                    self.Advance();
                    self.Advance();
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::Operator("?.".to_string()),"?.".to_string(), StartLine, StartCol, PrecedingNewlines,)
                    );
                    PrecedingNewlines = 0;
                    continue;
                }
                self.Advance();
                RawTokens.push(Token_Info::New(Token_Kind::Punctuation(Ch), Ch.to_string(), StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }
            // 10. Operators (including < and > which will be resolved in post-processing)
            let OpText = self.ScanOperator();
            RawTokens.push(Token_Info::New(Token_Kind::Operator(OpText.clone()), OpText, StartLine, StartCol, PrecedingNewlines,));
            PrecedingNewlines = 0;
        }
        // Post-process to resolve generic/template angle brackets < > in C++ and C#
        if self.Language.SupportsAngleGenerics() { ResolveAngleBrackets( & mut RawTokens); }

        RawTokens
    }

    fn ScanPreprocessor( & mut self)->String
    {
        let mut Result = String:: new ();
        while let Some(Ch) = self.Peek()
        {
            if Ch == '\n'
            {
                // Check for line continuation with backslash
                let Trimmed = Result.trim_end();
                if Trimmed.ends_with('\\') { Result.push(self.Advance().unwrap()); continue ; }
                break;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanLineComment( & mut self)->String
    {
        let mut Result = String:: new ();
        while let Some(Ch) = self.Peek() { if Ch == '\n' { break ; } Result.push(self.Advance().unwrap()); }
        Result
    }

    fn ScanBlockComment( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '/'
        Result.push(self.Advance().unwrap());
        // '*'

        while let Some(Ch) = self.Peek()
        {
            if Ch == '*' && self.PeekAhead(1) == Some('/')
            {
                Result.push(self.Advance().unwrap());
                Result.push(self.Advance().unwrap());
                break;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanRawRustString( & mut self, HashCount: usize)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap()); // 'r'
        for _ in 0..HashCount { Result.push(self.Advance().unwrap()); } // '#'
        Result.push(self.Advance().unwrap()); // '"'

        while let Some(Ch) = self.Peek()
        {
            if Ch == '"'
            {
                let mut Match = true;
                for i in 1..=HashCount
                {
                    if self.PeekAhead(i) != Some('#') { Match = false; break; }
                }
                if Match
                {
                    Result.push(self.Advance().unwrap()); // '"'
                    for _ in 0..HashCount { Result.push(self.Advance().unwrap()); } // '#'
                    break;
                }
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanStandardString( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '"'
        let mut Escaped = false;

        while let Some(Ch) = self.Peek()
        {
            if Escaped { Result.push(self.Advance().unwrap()); Escaped = false; }
            else if Ch == '\\' { Result.push(self.Advance().unwrap()); Escaped = true; }
            else if Ch == '"' { Result.push(self.Advance().unwrap()); break ; } else { Result.push(self.Advance().unwrap()); }
        }
        Result
    }

    fn ScanRawBacktickString( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '`'

        while let Some(Ch) = self.Peek()
        {
            if Ch == '`' { Result.push(self.Advance().unwrap()); break ; } else { Result.push(self.Advance().unwrap()); }
        }
        Result
    }

    fn ScanCharLiteral( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '\''
        let mut Escaped = false;

        while let Some(Ch) = self.Peek()
        {
            if Escaped { Result.push(self.Advance().unwrap()); Escaped = false; }
            else if Ch == '\\' { Result.push(self.Advance().unwrap()); Escaped = true; }
            else if Ch == '\'' { Result.push(self.Advance().unwrap()); break ; } else { Result.push(self.Advance().unwrap()); }
        }
        Result
    }

    fn ScanCppRawString( & mut self)->String
    {
        let mut Result = String:: new ();
        // Consume prefix like R or u8R
        while let Some(Ch) = self.Peek()
        {
            if Ch == '"' { Result.push(self.Advance().unwrap()); break ; }
            Result.push(self.Advance().unwrap());
        }
        // Extract delimiter between " and (
        let mut Delim = String:: new ();
        while let Some(Ch) = self.Peek()
        {
            if Ch == '(' { Result.push(self.Advance().unwrap()); break ; }
            Delim.push(Ch);
            Result.push(self.Advance().unwrap());
        }

        let TargetEnd = format!("){}\"", Delim);
        while self.Index < self.Chars.len()
        {
            if self.Source[self.Index..].starts_with( & TargetEnd) { for _ in 0..TargetEnd.len() { Result.push(self.Advance().unwrap()); } break ; }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanCSharpVerbatimString( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '@'
        Result.push(self.Advance().unwrap());
        // '"'

        while let Some(Ch) = self.Peek()
        {
            if Ch == '"'
            {
                if self.PeekAhead(1) == Some('"') { Result.push(self.Advance().unwrap()); Result.push(self.Advance().unwrap()); continue ; }
                Result.push(self.Advance().unwrap());
                break;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanCSharpRawString( & mut self)->String
    {
        let mut Result = String:: new ();
        let mut QuoteCount = 0;
        while self.Peek() == Some('"') { Result.push(self.Advance().unwrap()); QuoteCount += 1; }

        let Target = "\"".repeat(QuoteCount);
        while self.Index < self.Chars.len()
        {
            if self.Source[self.Index..].starts_with( & Target) { for _ in 0..QuoteCount { Result.push(self.Advance().unwrap()); } break ; }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanIdentifier( & mut self)->String
    {
        let mut Result = String:: new ();
        while let Some(Ch) = self.Peek()
        {
              if Ch.is_alphanumeric() || Ch == '_' || Ch == '$' || (self.Language == crate::types::Source_Language::Css && (Ch == '-' || Ch == '.'))
            {
                Result.push(self.Advance().unwrap());
            }
            else { break ; }
        }
        Result
    }

    fn ScanNumber( & mut self)->String
    {
        let mut Result = String:: new ();
        while let Some(Ch) = self.Peek()
        {
            if Ch.is_ascii_alphanumeric() || Ch == '.' || Ch == '_' || Ch == '\'' { Result.push(self.Advance().unwrap()); } else { break ; }
        }
        Result
    }

    fn ScanOperator( & mut self)->String
    {
        let mut Result = String:: new ();
        let OpChars = "=+-*/%&|^!~<>:";
        while let Some(Ch) = self.Peek() { if OpChars.contains(Ch) { Result.push(self.Advance().unwrap()); } else { break ; } }
        if Result.is_empty() { Result.push(self.Advance().unwrap()); }
        Result
    }
}

fn IsKeyword(Text: & str)-> bool
{
    matches!
    (
        Text, "if" | "else" | "while" | "for" | "do" | "switch" | "case" | "default" | "return" | "break" | "continue" | "goto" | "class" | "struct" | "union" | "enum" | "namespace" | "template" | "typename" | "public" | "private" | "protected" | "internal" | "virtual" | "override" | "static" | "const" | "constexpr" | "try" | "catch" | "finally" | "throw" | "new" | "delete" | "typedef" | "using" | "auto" | "var" | "void" | "int" | "bool" | "char" | "float" | "double" | "long" | "short" | "signed" | "unsigned" | "sizeof" | "decltype" | "interface" | "record" | "async" | "await" | "get" | "set" | "init" | "where" | "impl" | "fn" | "trait" | "match" | "loop" | "mod" | "let" | "pub" | "mut" | "use" | "type" | "as" | "ref" | "extern"
    )
}

fn ResolveAngleBrackets(Tokens: & mut Vec<Token_Info>)
{
    let mut I = 0;
    while I < Tokens.len()
    {
        if Tokens[I].Text == "<"
        {
            // Check left context
            let HasValidLeft = if I > 0
            {
                match & Tokens[I - 1].Kind
                {
                    Token_Kind::Identifier(_) => true, Token_Kind::Keyword(K) => matches!
                    (
                        K.as_str(),"template" | "typename" | "class" | "struct" | "interface" | "new" | "where" | "using" | "impl" | "fn" | "trait" | "enum" | "type"
                    ),
                    Token_Kind::Operator(Op) if Op == "::" => true, _ => false, 
                }
            }
            else { false };

            if HasValidLeft
            {
                // Scan forward to find matching '>'
                let mut Depth = 1;
                let mut MatchIndex = None;
                let mut J = I + 1;

                while J < Tokens.len() && J - I < 80
                {
                    if Tokens[J].Text == "<" { Depth += 1; }
                    else if Tokens[J].Text == ">" { Depth -= 1; if Depth == 0 { MatchIndex = Some(J); break ; } }
                    else if Tokens[J].Text == ">>" && Depth >= 1
                    {
                        // Closer splitting per LAFS edge-cases §A.1
                        let Tok = Tokens[J].clone();
                        Tokens[J].Kind = Token_Kind::Operator(">".to_string());
                        Tokens[J].Text = ">".to_string();
                        Tokens.insert
                        (
                            J + 1, Token_Info::New(Token_Kind::Operator(">".to_string()),">".to_string(), Tok.Line, Tok.Column + 1, 0,),
                        );
                        Depth -= 1;
                        if Depth == 0 { MatchIndex = Some(J); break ; }
                    }
                    else if matches!(Tokens[J].Text.as_str(),";" | "{" | "}")
                    {
                        break;
                        // statement boundary aborts
                    }
                    J += 1;
                }

                if let Some(M) = MatchIndex
                {
                    // Check follow-token test after '>' per LAFS edge-cases §A.2
                    let FollowValid = if M + 1 < Tokens.len()
                    {
                        let Next = & Tokens[M + 1];
                        matches!
                        (
                            Next.Text.as_str(),"(" | ")" | "]" | "}" | ":" | ";" | "," | "." | "?" | "==" | "!=" | "[" | "{" | "::" | "&" | "*" | "->" | ">"
                        )
                        || matches!(Next.Kind, Token_Kind::Identifier(_) | Token_Kind::Keyword(_)) || Next.Preceding_Newlines > 0
                    }
                    else { true };

                    if FollowValid
                    {
                        Tokens[I].Kind = Token_Kind::Open_Delim(Container_Kind::Angle);
                        Tokens[M].Kind = Token_Kind::Close_Delim(Container_Kind::Angle);
                    }
                }
            }
        }
        I += 1;
    }
}
