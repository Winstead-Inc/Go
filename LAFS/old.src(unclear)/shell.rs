#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::types::{Config_Settings, Container_Kind, Layout_State, Token_Info, Token_Kind};

pub struct Shell_Formatter;

impl Shell_Formatter
{
    pub fn Format(Source: &str, Config: &Config_Settings) -> Result<String, String>
    {
        let mut Lexer = Shell_Lexer::New(Source);
        let Tokens = Lexer.TokenizeAll();

        let mut Builder = Shell_Container_Builder::New(Tokens);
        let Nodes = Builder.BuildTree();

        let Emitter = Shell_Emitter::New(Config);
        let Formatted = Emitter.Emit(&Nodes);

        // Safety gate verification
        if let Err(VerifyErr) = VerifyShellTokenEquivalence(Source, &Formatted)
        {
            return Err(format!("{}. Output aborted to prevent code corruption.", VerifyErr));
        }

        Ok(Formatted)
    }

    pub fn IsFormatted(Source: &str, Config: &Config_Settings) -> bool
    {
        match Self::Format(Source, Config)
        {
            Ok(Formatted) => Formatted == Source,
            Err(_) => false
        }
    }
}

pub struct Shell_Lexer<'a>
{
    Source: &'a str,
    Chars: Vec<char>,
    Index: usize,
    Line: usize,
    Column: usize
}

impl<'a> Shell_Lexer<'a>
{
    pub fn New(Source: &'a str) -> Self
    {
        Shell_Lexer
        {
            Source,
            Chars: Source.chars().collect(),
            Index: 0,
            Line: 1,
            Column: 0
        }
    }

    fn Peek(&self) -> Option<char>
    {
        if self.Index < self.Chars.len() { Some(self.Chars[self.Index]) } else { None }
    }

    fn PeekAhead(&self, Offset: usize) -> Option<char>
    {
        let Target = self.Index + Offset;
        if Target < self.Chars.len() { Some(self.Chars[Target]) } else { None }
    }

    fn Advance(&mut self) -> Option<char>
    {
        if self.Index >= self.Chars.len() { return None; }
        let Ch = self.Chars[self.Index];
        self.Index += 1;
        if Ch == '\n'
        {
            self.Line += 1;
            self.Column = 0;
        }
        else
        {
            self.Column += 1;
        }
        Some(Ch)
    }

    pub fn TokenizeAll(&mut self) -> Vec<Token_Info>
    {
        let mut RawTokens: Vec<Token_Info> = Vec::new();
        let mut PrecedingNewlines: usize = 0;
        let mut CurrentLeadingWhitespace = String::new();
        let mut AtLineStart = true;

        // 1. Shebang on first line
        if self.Index == 0 && self.Peek() == Some('#') && self.PeekAhead(1) == Some('!')
        {
            let ShebangText = self.ScanLineComment();
            RawTokens.push(Token_Info::NewWithIndent(
                Token_Kind::Comment_Line(ShebangText.clone()),
                ShebangText,
                1,
                0,
                0,
                String::new()
            ));
            AtLineStart = false;
        }

        while let Some(Ch) = self.Peek()
        {
            let StartLine = self.Line;
            let StartCol = self.Column;

            // Newline
            if Ch == '\n'
            {
                self.Advance();
                PrecedingNewlines += 1;
                AtLineStart = true;
                CurrentLeadingWhitespace.clear();
                continue;
            }

            // Horizontal whitespace
            if Ch == ' ' || Ch == '\t' || Ch == '\r'
            {
                let WS = self.Advance().unwrap();
                if AtLineStart && (WS == ' ' || WS == '\t')
                {
                    CurrentLeadingWhitespace.push(WS);
                }
                continue;
            }

            let TokenIndent = if AtLineStart
            {
                AtLineStart = false;
                let Indent = CurrentLeadingWhitespace.clone();
                CurrentLeadingWhitespace.clear();
                Indent
            }
            else
            {
                String::new()
            };

            // Comments
            if Ch == '#'
            {
                let CommentText = self.ScanLineComment();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Comment_Line(CommentText.clone()),
                    CommentText,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Heredoc: << or <<-
            if Ch == '<' && self.PeekAhead(1) == Some('<')
            {
                let mut Op = String::from("<<");
                self.Advance();
                self.Advance();
                if self.Peek() == Some('-')
                {
                    Op.push(self.Advance().unwrap());
                }

                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Operator(Op.clone()),
                    Op,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;

                // Skip horizontal spaces to find delimiter tag
                while let Some(SC) = self.Peek()
                {
                    if SC == ' ' || SC == '\t'
                    {
                        self.Advance();
                    }
                    else
                    {
                        break;
                    }
                }

                // Scan delimiter tag
                if let Some(TagCh) = self.Peek()
                {
                    if TagCh != '\n' && TagCh != ';' && TagCh != '&' && TagCh != '|'
                    {
                        let DelimTag = self.ScanHeredocDelimiterTag();
                        let TagLine = self.Line;
                        let TagCol = self.Column;
                        RawTokens.push(Token_Info::NewWithIndent(
                            Token_Kind::String_Lit(DelimTag.clone()),
                            DelimTag.clone(),
                            TagLine,
                            TagCol,
                            0,
                            String::new()
                        ));

                        // Read rest of command line until newline
                        let mut TrailingCmdLine = Vec::new();
                        while let Some(RestCh) = self.Peek()
                        {
                            if RestCh == '\n'
                            {
                                self.Advance();
                                break;
                            }
                            if RestCh == ' ' || RestCh == '\t' || RestCh == '\r'
                            {
                                self.Advance();
                                continue;
                            }
                            let Word = self.ScanWord();
                            TrailingCmdLine.push(Word);
                        }

                        for CmdWord in TrailingCmdLine
                        {
                            RawTokens.push(Token_Info::NewWithIndent(
                                Token_Kind::Identifier(CmdWord.clone()),
                                CmdWord,
                                self.Line,
                                self.Column,
                                0,
                                String::new()
                            ));
                        }

                        // Now scan the heredoc body verbatim
                        let CleanTag = DelimTag.trim_matches('\'').trim_matches('"');
                        let HeredocBody = self.ScanHeredocBody(CleanTag);
                        RawTokens.push(Token_Info::NewWithIndent(
                            Token_Kind::Template_Lit(HeredocBody.clone()),
                            HeredocBody,
                            self.Line,
                            0,
                            1,
                            String::new()
                        ));
                        PrecedingNewlines = 1;
                        AtLineStart = true;
                        CurrentLeadingWhitespace.clear();
                        continue;
                    }
                }
                continue;
            }

            // Single quoted string '...' (verbatim)
            if Ch == '\''
            {
                let mut StrText = self.ScanSingleQuotedString();
                if let Some(NextCh) = self.Peek()
                {
                    if !NextCh.is_whitespace() && NextCh != ';' && NextCh != '&' && NextCh != '|' && NextCh != ')' && NextCh != '('
                    {
                        let RestWord = self.ScanWord();
                        StrText.push_str(&RestWord);
                    }
                }
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::String_Lit(StrText.clone()),
                    StrText,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Double quoted string "..."
            if Ch == '"'
            {
                let mut StrText = self.ScanDoubleQuotedString();
                if let Some(NextCh) = self.Peek()
                {
                    if !NextCh.is_whitespace() && NextCh != ';' && NextCh != '&' && NextCh != '|' && NextCh != ')' && NextCh != '('
                    {
                        let RestWord = self.ScanWord();
                        StrText.push_str(&RestWord);
                    }
                }
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::String_Lit(StrText.clone()),
                    StrText,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // ANSI-C quoted string $'...'
            if Ch == '$' && self.PeekAhead(1) == Some('\'')
            {
                self.Advance(); // '$'
                let StrText = self.ScanSingleQuotedString();
                let Full = format!("${}", StrText);
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::String_Lit(Full.clone()),
                    Full,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Delimiters
            if Ch == '{'
            {
                self.Advance();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Open_Delim(Container_Kind::Brace),
                    "{".to_string(),
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '}'
            {
                self.Advance();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Close_Delim(Container_Kind::Brace),
                    "}".to_string(),
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '('
            {
                self.Advance();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Open_Delim(Container_Kind::Paren),
                    "(".to_string(),
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == ')'
            {
                self.Advance();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Close_Delim(Container_Kind::Paren),
                    ")".to_string(),
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            if Ch == '['
            {
                let IsTestBracket = self.PeekAhead(1) == Some(' ')
                    || self.PeekAhead(1) == Some('\t')
                    || self.PeekAhead(1) == Some('\n')
                    || (self.PeekAhead(1) == Some('[')
                        && (self.PeekAhead(2) == Some(' ')
                            || self.PeekAhead(2) == Some('\t')
                            || self.PeekAhead(2) == Some('\n')));
                if IsTestBracket
                {
                    let mut Text = "[".to_string();
                    self.Advance();
                    if self.Peek() == Some('[')
                    {
                        Text.push(self.Advance().unwrap());
                    }
                    RawTokens.push(Token_Info::NewWithIndent(
                        Token_Kind::Open_Delim(Container_Kind::Bracket),
                        Text,
                        StartLine,
                        StartCol,
                        PrecedingNewlines,
                        TokenIndent
                    ));
                    PrecedingNewlines = 0;
                    continue;
                }
            }

            if Ch == ']'
            {
                let mut Text = "]".to_string();
                self.Advance();
                if self.Peek() == Some(']')
                {
                    Text.push(self.Advance().unwrap());
                }
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Close_Delim(Container_Kind::Bracket),
                    Text,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Semicolons and case terminator ;;
            if Ch == ';'
            {
                let mut Semi = ";".to_string();
                self.Advance();
                if self.Peek() == Some(';')
                {
                    Semi.push(self.Advance().unwrap());
                }
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Operator(Semi.clone()),
                    Semi,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Pipe, redirects, and file descriptor redirects like 2>, 2>&1
            let IsFdRedirect = Ch.is_ascii_digit()
                && (self.PeekAhead(1) == Some('>') || self.PeekAhead(1) == Some('<'));
            if Ch == '|' || Ch == '&' || Ch == '>' || Ch == '<' || IsFdRedirect
            {
                let OpText = self.ScanShellOperator();
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Operator(OpText.clone()),
                    OpText,
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 0;
                continue;
            }

            // Line continuation \
            if Ch == '\\' && self.PeekAhead(1) == Some('\n')
            {
                self.Advance(); // '\'
                self.Advance(); // '\n'
                RawTokens.push(Token_Info::NewWithIndent(
                    Token_Kind::Operator("\\\n".to_string()),
                    "\\\n".to_string(),
                    StartLine,
                    StartCol,
                    PrecedingNewlines,
                    TokenIndent
                ));
                PrecedingNewlines = 1;
                AtLineStart = true;
                CurrentLeadingWhitespace.clear();
                continue;
            }

            // Words, commands, variables, parameter expansions
            let WordText = self.ScanWord();
            let Kind = if IsShellKeyword(&WordText)
            {
                Token_Kind::Keyword(WordText.clone())
            }
            else
            {
                Token_Kind::Identifier(WordText.clone())
            };

            RawTokens.push(Token_Info::NewWithIndent(Kind, WordText, StartLine, StartCol, PrecedingNewlines, TokenIndent));
            PrecedingNewlines = 0;
        }

        RawTokens
    }

    fn ScanLineComment(&mut self) -> String
    {
        let mut Result = String::new();
        while let Some(Ch) = self.Peek()
        {
            if Ch == '\n' { break; }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanSingleQuotedString(&mut self) -> String
    {
        let mut Result = String::new();
        Result.push(self.Advance().unwrap()); // opening '
        while let Some(Ch) = self.Peek()
        {
            Result.push(self.Advance().unwrap());
            if Ch == '\'' { break; }
        }
        Result
    }

    fn ScanDoubleQuotedString(&mut self) -> String
    {
        let mut Result = String::new();
        Result.push(self.Advance().unwrap()); // opening "
        while let Some(Ch) = self.Peek()
        {
            if Ch == '\\'
            {
                Result.push(self.Advance().unwrap());
                if self.Peek().is_some()
                {
                    Result.push(self.Advance().unwrap());
                }
                continue;
            }
            if Ch == '$' && self.PeekAhead(1) == Some('(')
            {
                Result.push(self.Advance().unwrap()); // '$'
                Result.push(self.Advance().unwrap()); // '('
                let mut ParenDepth = 1;
                while let Some(SubCh) = self.Peek()
                {
                    if SubCh == '\\'
                    {
                        Result.push(self.Advance().unwrap());
                        if self.Peek().is_some()
                        {
                            Result.push(self.Advance().unwrap());
                        }
                        continue;
                    }
                    if SubCh == '('
                    {
                        Result.push(self.Advance().unwrap());
                        ParenDepth += 1;
                        continue;
                    }
                    if SubCh == ')'
                    {
                        Result.push(self.Advance().unwrap());
                        ParenDepth -= 1;
                        if ParenDepth == 0
                        {
                            break;
                        }
                        continue;
                    }
                    Result.push(self.Advance().unwrap());
                }
                continue;
            }
            Result.push(self.Advance().unwrap());
            if Ch == '"' { break; }
        }
        Result
    }

    fn ScanHeredocDelimiterTag(&mut self) -> String
    {
        let mut Result = String::new();
        while let Some(Ch) = self.Peek()
        {
            if Ch == ' ' || Ch == '\t' || Ch == '\r' || Ch == '\n' || Ch == ';' || Ch == '&' || Ch == '|'
            {
                break;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanHeredocBody(&mut self, CleanDelimiter: &str) -> String
    {
        let mut Result = String::new();
        let mut CurrentLine = String::new();

        while let Some(Ch) = self.Peek()
        {
            self.Advance();
            if Ch == '\n'
            {
                let Trimmed = CurrentLine.trim_start_matches('\t');
                if Trimmed == CleanDelimiter
                {
                    Result.push_str(&CurrentLine);
                    CurrentLine.clear();
                    break;
                }
                Result.push_str(&CurrentLine);
                Result.push('\n');
                CurrentLine.clear();
            }
            else
            {
                CurrentLine.push(Ch);
            }
        }

        if !CurrentLine.is_empty()
        {
            Result.push_str(&CurrentLine);
        }

        Result
    }

    fn ScanShellOperator(&mut self) -> String
    {
        let mut Result = String::new();
        if let Some(Ch) = self.Peek()
        {
            if Ch.is_ascii_digit()
            {
                Result.push(self.Advance().unwrap());
            }
        }
        let First = self.Advance().unwrap();
        Result.push(First);

        let Next = self.Peek();
        match (First, Next)
        {
            ('|', Some('|')) | ('&', Some('&')) | ('>', Some('>')) | ('<', Some('<')) =>
            {
                Result.push(self.Advance().unwrap());
            }
            ('|', Some('&')) =>
            {
                Result.push(self.Advance().unwrap());
            }
            ('>', Some('&')) =>
            {
                Result.push(self.Advance().unwrap());
                if let Some(Digit) = self.Peek()
                {
                    if Digit.is_ascii_digit()
                    {
                        Result.push(self.Advance().unwrap());
                    }
                }
            }
            _ => {}
        }

        Result
    }

    fn ScanParameterExpansion(&mut self) -> String
    {
        let mut Result = String::new();
        Result.push(self.Advance().unwrap()); // '$'
        Result.push(self.Advance().unwrap()); // '{'

        let mut Depth = 1;
        while let Some(Ch) = self.Peek()
        {
            if Ch == '\\'
            {
                Result.push(self.Advance().unwrap());
                if self.Peek().is_some()
                {
                    Result.push(self.Advance().unwrap());
                }
                continue;
            }
            if Ch == '\''
            {
                let Str = self.ScanSingleQuotedString();
                Result.push_str(&Str);
                continue;
            }
            if Ch == '"'
            {
                let Str = self.ScanDoubleQuotedString();
                Result.push_str(&Str);
                continue;
            }
            if Ch == '$' && self.PeekAhead(1) == Some('{')
            {
                Result.push(self.Advance().unwrap());
                Result.push(self.Advance().unwrap());
                Depth += 1;
                continue;
            }
            if Ch == '}'
            {
                Result.push(self.Advance().unwrap());
                Depth -= 1;
                if Depth == 0
                {
                    break;
                }
                continue;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanWord(&mut self) -> String
    {
        let mut Result = String::new();
        while let Some(Ch) = self.Peek()
        {
            if Ch == ' ' || Ch == '\t' || Ch == '\r' || Ch == '\n'
                || Ch == ';' || Ch == '|' || Ch == '&'
                || Ch == '(' || Ch == ')' || Ch == '<' || Ch == '>'
            {
                break;
            }

            // Comment marker # only breaks at start of word (allowing $#, C#, foo#bar)
            if Ch == '#' && Result.is_empty()
            {
                break;
            }

            // Standalone braces { and } (command grouping and function bodies)
            if (Ch == '{' || Ch == '}') && Result.is_empty()
            {
                break;
            }

            // Test brackets [ and ] vs glob patterns
            if Ch == '['
            {
                let IsTestBracket = self.PeekAhead(1) == Some(' ')
                    || self.PeekAhead(1) == Some('\t')
                    || self.PeekAhead(1) == Some('\n')
                    || (self.PeekAhead(1) == Some('[')
                        && (self.PeekAhead(2) == Some(' ')
                            || self.PeekAhead(2) == Some('\t')
                            || self.PeekAhead(2) == Some('\n')));
                if IsTestBracket && Result.is_empty()
                {
                    break;
                }
            }

            if Ch == ']' && Result.is_empty()
            {
                break;
            }

            // Parameter expansion ${...}
            if Ch == '$' && self.PeekAhead(1) == Some('{')
            {
                let ParamExp = self.ScanParameterExpansion();
                Result.push_str(&ParamExp);
                continue;
            }

            // Single quoted string inside word: prefix'str'suffix
            if Ch == '\''
            {
                let Str = self.ScanSingleQuotedString();
                Result.push_str(&Str);
                continue;
            }

            // Double quoted string inside word: prefix"str"suffix
            if Ch == '"'
            {
                let Str = self.ScanDoubleQuotedString();
                Result.push_str(&Str);
                continue;
            }

            if Ch == '\\'
            {
                Result.push(self.Advance().unwrap());
                if self.Peek().is_some()
                {
                    Result.push(self.Advance().unwrap());
                }
                continue;
            }

            Result.push(self.Advance().unwrap());
        }
        Result
    }
}

fn IsShellKeyword(Word: &str) -> bool
{
    matches!(
        Word,
        "if" | "then" | "elif" | "else" | "fi"
        | "for" | "in" | "do" | "done"
        | "while" | "until"
        | "case" | "esac"
        | "function" | "select" | "time"
    )
}

#[derive(Debug, Clone)]
pub enum Shell_Syntax_Node
{
    Token(Token_Info),
    Container(Shell_Container)
}

#[derive(Debug, Clone)]
pub struct Shell_Container
{
    pub Kind: Container_Kind,
    pub Open_Token: Token_Info,
    pub Close_Token: Option<Token_Info>,
    pub Children: Vec<Shell_Syntax_Node>,
    pub Tail_Glue: Vec<Token_Info>,
    pub Has_Forced_Break: bool,
    pub Statement_Count: usize,
    pub Decl_Indent: String
}

impl Shell_Container
{
    pub fn New(Kind: Container_Kind, Open_Token: Token_Info) -> Self
    {
        Shell_Container
        {
            Kind,
            Open_Token,
            Close_Token: None,
            Children: Vec::new(),
            Tail_Glue: Vec::new(),
            Has_Forced_Break: false,
            Statement_Count: 0,
            Decl_Indent: String::new()
        }
    }

    pub fn IsSoleChildChain(&self) -> bool
    {
        let NonTrivia: Vec<&Shell_Syntax_Node> = self.Children
            .iter()
            .filter(|Node| match Node
            {
                Shell_Syntax_Node::Token(T) => !T.IsTrivia() && !T.IsComment(),
                Shell_Syntax_Node::Container(_) => true
            })
            .collect();

        if NonTrivia.len() == 1
        {
            matches!(NonTrivia[0], Shell_Syntax_Node::Container(_))
        }
        else
        {
            false
        }
    }

    pub fn GetSoleChildContainer(&self) -> Option<&Shell_Container>
    {
        let NonTrivia: Vec<&Shell_Syntax_Node> = self.Children
            .iter()
            .filter(|Node| match Node
            {
                Shell_Syntax_Node::Token(T) => !T.IsTrivia() && !T.IsComment(),
                Shell_Syntax_Node::Container(_) => true
            })
            .collect();

        if NonTrivia.len() == 1
        {
            if let Shell_Syntax_Node::Container(C) = NonTrivia[0]
            {
                return Some(C);
            }
        }
        None
    }

    pub fn CollectChain(&self) -> Vec<&Shell_Container>
    {
        let mut Chain = vec![self];
        let mut Current = self;
        while let Some(Child) = Current.GetSoleChildContainer()
        {
            Chain.push(Child);
            Current = Child;
        }
        Chain
    }

    pub fn CalculateSingleLineSpan(&self) -> usize
    {
        if self.Has_Forced_Break { return usize::MAX; }

        let mut Total = self.Kind.OpenGlyph().len() + self.Kind.CloseGlyph().len();
        for Glue in &self.Tail_Glue
        {
            Total += Glue.Text.len();
        }

        let mut PrevNeedsSpace = matches!(self.Kind, Container_Kind::Brace | Container_Kind::Bracket);
        for Node in &self.Children
        {
            match Node
            {
                Shell_Syntax_Node::Token(T) =>
                {
                    if T.IsTrivia() { continue; }
                    if T.IsComment()
                    {
                        return usize::MAX;
                    }
                    if PrevNeedsSpace
                    {
                        Total += 1;
                    }
                    Total += T.Text.len();
                    PrevNeedsSpace = true;
                }
                Shell_Syntax_Node::Container(C) =>
                {
                    let ChildSpan = C.CalculateSingleLineSpan();
                    if ChildSpan == usize::MAX { return usize::MAX; }
                    if PrevNeedsSpace
                    {
                        Total += 1;
                    }
                    Total += ChildSpan;
                    PrevNeedsSpace = true;
                }
            }
        }

        Total
    }

    pub fn CalculateBlockDepth(&self) -> usize
    {
        let mut MaxChildDepth = 0;
        for Child in &self.Children
        {
            if let Shell_Syntax_Node::Container(Sub) = Child
            {
                let SubDepth = Sub.CalculateBlockDepth();
                if SubDepth > MaxChildDepth
                {
                    MaxChildDepth = SubDepth;
                }
            }
        }
        1 + MaxChildDepth
    }

    pub fn CountTotalContainers(&self) -> usize
    {
        let mut Count = 1;
        for Child in &self.Children
        {
            if let Shell_Syntax_Node::Container(Sub) = Child
            {
                Count += Sub.CountTotalContainers();
            }
        }
        Count
    }
}

pub struct Shell_Container_Builder
{
    Tokens: Vec<Token_Info>,
    Index: usize
}

impl Shell_Container_Builder
{
    pub fn New(Tokens: Vec<Token_Info>) -> Self
    {
        Shell_Container_Builder { Tokens, Index: 0 }
    }

    pub fn BuildTree(&mut self) -> Vec<Shell_Syntax_Node>
    {
        let mut RootNodes = Vec::new();
        while self.Index < self.Tokens.len()
        {
            if let Some(Node) = self.ParseNextNode()
            {
                RootNodes.push(Node);
            }
        }
        RootNodes
    }

    fn FindLineIndent(&self, TokenIdx: usize) -> String
    {
        if TokenIdx == 0 || TokenIdx >= self.Tokens.len()
        {
            return self.Tokens.get(TokenIdx).map(|T| T.Leading_Whitespace.clone()).unwrap_or_default();
        }

        // Case 1: Open delimiter is on the same line as declaration (e.g. `foo() {`)
        if self.Tokens[TokenIdx].Preceding_Newlines == 0
        {
            let mut K = TokenIdx;
            while K > 0 && self.Tokens[K].Preceding_Newlines == 0
            {
                K -= 1;
            }
            return self.Tokens[K].Leading_Whitespace.clone();
        }

        // Case 2: Open delimiter is on a new line (e.g. `foo()\n{`)
        // If preceded by `)` or function name, match the declaration's indentation
        if TokenIdx > 0
        {
            let Prev = &self.Tokens[TokenIdx - 1];
            let IsFuncDecl = Prev.Text == ")"
                || (TokenIdx >= 2 && self.Tokens[TokenIdx - 2].Text == "function");
            if IsFuncDecl
            {
                let mut K = TokenIdx - 1;
                while K > 0 && self.Tokens[K].Preceding_Newlines == 0
                {
                    K -= 1;
                }
                return self.Tokens[K].Leading_Whitespace.clone();
            }
        }

        // Default: use the open delimiter's own leading whitespace
        self.Tokens[TokenIdx].Leading_Whitespace.clone()
    }

    fn ParseNextNode(&mut self) -> Option<Shell_Syntax_Node>
    {
        if self.Index >= self.Tokens.len() { return None; }
        let TokenIdx = self.Index;
        let Token = self.Tokens[self.Index].clone();
        self.Index += 1;

        match &Token.Kind
        {
            Token_Kind::Open_Delim(Kind) =>
            {
                let mut Container = Shell_Container::New(*Kind, Token);
                Container.Decl_Indent = self.FindLineIndent(TokenIdx);
                self.PopulateContainerChildren(&mut Container);
                Some(Shell_Syntax_Node::Container(Container))
            }
            _ => Some(Shell_Syntax_Node::Token(Token))
        }
    }

    fn PopulateContainerChildren(&mut self, Container: &mut Shell_Container)
    {
        let mut StatementCount = 0;

        while self.Index < self.Tokens.len()
        {
            let CurrentToken = self.Tokens[self.Index].clone();

            if CurrentToken.Preceding_Newlines > 0 || CurrentToken.Text.contains('\n')
            {
                Container.Has_Forced_Break = true;
            }

            match &CurrentToken.Kind
            {
                Token_Kind::Close_Delim(CloseKind) if *CloseKind == Container.Kind =>
                {
                    self.Index += 1;
                    Container.Close_Token = Some(CurrentToken);

                    // Consume trailing glue (; or && or ||)
                    while self.Index < self.Tokens.len()
                    {
                        let NextToken = &self.Tokens[self.Index];
                        if NextToken.Preceding_Newlines == 0 && (NextToken.Text == ";" || NextToken.Text == ";;")
                        {
                            Container.Tail_Glue.push(NextToken.clone());
                            self.Index += 1;
                        }
                        else
                        {
                            break;
                        }
                    }
                    break;
                }
                Token_Kind::Open_Delim(_) =>
                {
                    if let Some(ChildNode) = self.ParseNextNode()
                    {
                        Container.Children.push(ChildNode);
                    }
                }
                Token_Kind::Comment_Line(_) =>
                {
                    Container.Has_Forced_Break = true;
                    Container.Children.push(Shell_Syntax_Node::Token(CurrentToken));
                    self.Index += 1;
                }
                _ =>
                {
                    if CurrentToken.Text == ";" || CurrentToken.Preceding_Newlines > 0
                    {
                        StatementCount += 1;
                    }
                    Container.Children.push(Shell_Syntax_Node::Token(CurrentToken));
                    self.Index += 1;
                }
            }
        }

        Container.Statement_Count = StatementCount;
    }
}

pub struct Shell_Layout_Planner<'a>
{
    Config: &'a Config_Settings
}

impl<'a> Shell_Layout_Planner<'a>
{
    pub fn New(Config: &'a Config_Settings) -> Self
    {
        Shell_Layout_Planner { Config }
    }

    pub fn DecideLayout(&self, Container: &Shell_Container, CurrentIndentLevel: usize, CurrentColumn: usize) -> Layout_State
    {
        // 1. Monolith First (Horizontal Density)
        if !Container.Has_Forced_Break
            && Container.Statement_Count <= self.Config.Max_Statements_In_Monolith
            && Container.CalculateBlockDepth() <= self.Config.Max_Block_Depth
            && Container.CountTotalContainers() <= self.Config.Max_Containers_In_Monolith
        {
            let SingleSpan = Container.CalculateSingleLineSpan();
            if SingleSpan != usize::MAX
            {
                let BaseCol = if CurrentColumn > 0
                {
                    CurrentColumn
                }
                else
                {
                    CurrentIndentLevel * self.Config.Indent_Width
                };
                let ProjectedCol = BaseCol + SingleSpan;
                if ProjectedCol <= self.Config.Canvas_Width
                {
                    return Layout_State::Monolith;
                }
            }
        }

        // 2. Compound Delimiters (Sole-Child Chain)
        if self.Config.Allow_Brace_Grouping && Container.IsSoleChildChain()
        {
            let Chain = Container.CollectChain();
            if Chain.len() >= 2
            {
                return Layout_State::Grouped;
            }
        }

        // 3. Expanded Symmetry (Hierarchical Architecture)
        Layout_State::Expanded
    }
}

pub struct Shell_Emitter<'a>
{
    Config: &'a Config_Settings,
    Planner: Shell_Layout_Planner<'a>,
    Output: String,
    Current_Line_Len: usize,
    Line_Has_Content: bool
}

impl<'a> Shell_Emitter<'a>
{
    pub fn New(Config: &'a Config_Settings) -> Self
    {
        Shell_Emitter
        {
            Config,
            Planner: Shell_Layout_Planner::New(Config),
            Output: String::new(),
            Current_Line_Len: 0,
            Line_Has_Content: false
        }
    }

    pub fn Emit(mut self, Nodes: &[Shell_Syntax_Node]) -> String
    {
        self.EmitNodeList(Nodes, 0);
        if !self.Output.ends_with('\n')
        {
            self.Output.push('\n');
        }
        self.Output
    }

    fn WriteNewline(&mut self)
    {
        while self.Output.ends_with(' ')
        {
            self.Output.pop();
        }
        self.Output.push('\n');
        self.Current_Line_Len = 0;
        self.Line_Has_Content = false;
    }

    fn EnsureNewline(&mut self)
    {
        if self.Line_Has_Content
        {
            self.WriteNewline();
        }
    }

    fn EmitIndent(&mut self, Level: usize)
    {
        if !self.Line_Has_Content
        {
            let IndentStr = self.Config.IndentString(Level);
            self.Output.push_str(&IndentStr);
            self.Current_Line_Len = IndentStr.len();
            self.Line_Has_Content = true;
        }
    }

    fn EmitRaw(&mut self, Text: &str)
    {
        self.Output.push_str(Text);
        self.Current_Line_Len += Text.len();
        self.Line_Has_Content = true;
    }

    fn EmitSpace(&mut self)
    {
        if self.Line_Has_Content && self.Current_Line_Len > 0 && !self.Output.ends_with(' ') && !self.Output.ends_with('\n')
        {
            self.Output.push(' ');
            self.Current_Line_Len += 1;
        }
    }

    fn EmitNodeList(&mut self, Nodes: &[Shell_Syntax_Node], Level: usize)
    {
        let Total = Nodes.len();
        let mut Index = 0;

        while Index < Total
        {
            let Node = &Nodes[Index];

            match Node
            {
                Shell_Syntax_Node::Token(T) =>
                {
                    self.EmitSingleToken(T, Level);
                }
                Shell_Syntax_Node::Container(C) =>
                {
                    let Layout = self.Planner.DecideLayout(C, Level, self.Current_Line_Len);
                    self.EmitContainerNodeWithLayout(C, Level, Layout);

                    // If brace block ended, next statement on new line
                    if C.Kind == Container_Kind::Brace || C.Tail_Glue.iter().any(|G| G.Text == ";")
                    {
                        self.EnsureNewline();
                    }
                }
            }

            Index += 1;
        }
    }

    fn EmitSingleToken(&mut self, Token: &Token_Info, Level: usize)
    {
        if Token.IsTrivia()
        {
            return;
        }

        // 1. Preceding newlines MUST be handled before comments or tokens
        if Token.Preceding_Newlines > 0
        {
            let BlankLines = (Token.Preceding_Newlines - 1).min(self.Config.Max_Blank_Lines);
            self.EnsureNewline();
            for _ in 0..BlankLines
            {
                self.WriteNewline();
            }
        }

        // 2. Line comment
        if let Token_Kind::Comment_Line(CommentText) = &Token.Kind
        {
            if self.Line_Has_Content
            {
                self.EmitSpace();
            }
            else
            {
                if !Token.Leading_Whitespace.is_empty()
                {
                    self.EmitRaw(&Token.Leading_Whitespace);
                }
                else
                {
                    self.EmitIndent(Level);
                }
            }
            self.EmitRaw(CommentText);
            self.WriteNewline();
            return;
        }

        // 3. Line start indentation
        if !self.Line_Has_Content
        {
            if !Token.Leading_Whitespace.is_empty()
            {
                self.EmitRaw(&Token.Leading_Whitespace);
            }
            else
            {
                self.EmitIndent(Level);
            }
        }
        else
        {
            self.EmitSpaceBeforeToken(Token);
        }

        // 4. Line continuation
        if Token.Text == "\\\n"
        {
            self.EmitRaw("\\");
            self.WriteNewline();
            return;
        }

        self.EmitRaw(&Token.Text);
        if Token.Text == ";;"
        {
            self.WriteNewline();
        }
    }

    fn EmitSpaceBeforeToken(&mut self, Token: &Token_Info)
    {
        if Token.Text == ";" || Token.Text == ";;" || Token.Text == ")"
        {
            return;
        }
        if self.Output.ends_with('(')
        {
            return;
        }
        self.EmitSpace();
    }

    fn EmitContainerNodeWithLayout(&mut self, Container: &Shell_Container, Level: usize, Layout: Layout_State)
    {
        match Layout
        {
            Layout_State::Monolith =>
            {
                self.EmitMonolith(Container, Level);
            }
            Layout_State::Grouped =>
            {
                self.EmitGrouped(Container, Level);
            }
            Layout_State::Expanded =>
            {
                self.EmitExpanded(Container, Level);
            }
        }
    }

    fn EmitMonolith(&mut self, Container: &Shell_Container, Level: usize)
    {
        if !self.Line_Has_Content
        {
            if !Container.Decl_Indent.is_empty()
            {
                self.EmitRaw(&Container.Decl_Indent);
            }
            else
            {
                self.EmitIndent(Level);
            }
        }
        else if Container.Kind == Container_Kind::Brace || Container.Kind == Container_Kind::Bracket
        {
            self.EmitSpace();
        }
        else if Container.Kind == Container_Kind::Paren
        {
            if !self.Output.ends_with(' ')
            {
                let LastChar = self.Output.chars().last();
                if let Some(Ch) = LastChar
                {
                    if !Ch.is_alphanumeric() && Ch != '_' && Ch != '$' && Ch != ')' && Ch != ']'
                    {
                        self.EmitSpace();
                    }
                }
            }
        }

        self.EmitRaw(Container.Kind.OpenGlyph());
        if Container.Kind == Container_Kind::Brace || Container.Kind == Container_Kind::Bracket
        {
            self.EmitSpace();
        }

        let mut PrevNeedsSpace = false;
        for Child in &Container.Children
        {
            match Child
            {
                Shell_Syntax_Node::Token(T) =>
                {
                    if T.IsTrivia() { continue; }
                    if PrevNeedsSpace && T.Text != ";" && T.Text != ";;"
                    {
                        self.EmitSpace();
                    }
                    self.EmitRaw(&T.Text);
                    PrevNeedsSpace = true;
                }
                Shell_Syntax_Node::Container(Sub) =>
                {
                    if PrevNeedsSpace
                    {
                        self.EmitSpace();
                    }
                    self.EmitMonolith(Sub, Level);
                    PrevNeedsSpace = true;
                }
            }
        }

        if Container.Close_Token.is_some()
        {
            if (Container.Kind == Container_Kind::Brace || Container.Kind == Container_Kind::Bracket)
                && !self.Output.ends_with(' ')
            {
                self.EmitSpace();
            }

            self.EmitRaw(Container.Kind.CloseGlyph());
            for Glue in &Container.Tail_Glue
            {
                self.EmitRaw(&Glue.Text);
            }
        }
    }

    fn EmitGrouped(&mut self, Container: &Shell_Container, Level: usize)
    {
        let Chain = Container.CollectChain();
        let OpenRun: String = Chain.iter().map(|C| C.Kind.OpenGlyph()).collect();
        let CloseRun: String = Chain.iter().rev().map(|C| C.Kind.CloseGlyph()).collect();

        self.EnsureNewline();
        if !Container.Decl_Indent.is_empty()
        {
            self.EmitRaw(&Container.Decl_Indent);
        }
        else
        {
            self.EmitIndent(Level);
        }
        self.EmitRaw(&OpenRun);
        self.WriteNewline();

        if let Some(Innermost) = Chain.last()
        {
            self.EmitNodeList(&Innermost.Children, Level + 1);
        }

        self.EnsureNewline();
        if !Container.Decl_Indent.is_empty()
        {
            self.EmitRaw(&Container.Decl_Indent);
        }
        else
        {
            self.EmitIndent(Level);
        }
        self.EmitRaw(&CloseRun);

        if let Some(Outermost) = Chain.first()
        {
            for Glue in &Outermost.Tail_Glue
            {
                self.EmitRaw(&Glue.Text);
            }
        }
        self.WriteNewline();
    }

    fn EmitExpanded(&mut self, Container: &Shell_Container, Level: usize)
    {
        // OLAFS Detached Delimiter Symmetry
        if Container.Kind == Container_Kind::Paren && self.Output.ends_with('$')
        {
            self.EmitRaw(Container.Kind.OpenGlyph());
            let FirstChildOnSameLine = Container.Children.first().map_or(false, |Node| match Node
            {
                Shell_Syntax_Node::Token(T) => T.Preceding_Newlines == 0,
                Shell_Syntax_Node::Container(C) => C.Open_Token.Preceding_Newlines == 0,
            });
            if !FirstChildOnSameLine
            {
                self.WriteNewline();
            }
        }
        else
        {
            self.EnsureNewline();
            if !Container.Decl_Indent.is_empty()
            {
                self.EmitRaw(&Container.Decl_Indent);
            }
            else
            {
                self.EmitIndent(Level);
            }
            self.EmitRaw(Container.Kind.OpenGlyph());
            self.WriteNewline();
        }

        self.EmitNodeList(&Container.Children, Level + 1);

        if Container.Close_Token.is_some()
        {
            let CloseOnSameLine = Container.Kind == Container_Kind::Paren
                && Container.Close_Token.as_ref().map_or(false, |T| T.Preceding_Newlines == 0);
            if !CloseOnSameLine
            {
                self.EnsureNewline();
                if !Container.Decl_Indent.is_empty()
                {
                    self.EmitRaw(&Container.Decl_Indent);
                }
                else
                {
                    self.EmitIndent(Level);
                }
            }
            self.EmitRaw(Container.Kind.CloseGlyph());
            for Glue in &Container.Tail_Glue
            {
                self.EmitRaw(&Glue.Text);
            }
            if !CloseOnSameLine
            {
                self.WriteNewline();
            }
        }
    }
}

pub fn VerifyShellTokenEquivalence(OriginalSource: &str, FormattedSource: &str) -> Result<(), String>
{
    let mut OriginalScanner = Shell_Lexer::New(OriginalSource);
    let OriginalTokens = OriginalScanner.TokenizeAll();

    let mut FormattedScanner = Shell_Lexer::New(FormattedSource);
    let FormattedTokens = FormattedScanner.TokenizeAll();

    let OrigSignificant: Vec<&str> = OriginalTokens
        .iter()
        .filter(|T| !matches!(T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline))
        .map(|T| T.Text.as_str())
        .collect();

    let FormSignificant: Vec<&str> = FormattedTokens
        .iter()
        .filter(|T| !matches!(T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline))
        .map(|T| T.Text.as_str())
        .collect();

    if OrigSignificant.len() != FormSignificant.len()
    {
        return Err(format!(
            "Safety Gate Failed: Significant token count mismatch! (Original: {}, Formatted: {})",
            OrigSignificant.len(),
            FormSignificant.len()
        ));
    }

    for (Index, (Orig, Form)) in OrigSignificant.iter().zip(FormSignificant.iter()).enumerate()
    {
        if Orig != Form
        {
            return Err(format!(
                "Safety Gate Failed: Token #{} mismatch! (Expected '{}', Got '{}')",
                Index, Orig, Form
            ));
        }
    }

    Ok(())
}
