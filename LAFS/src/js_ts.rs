#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]
use crate::types:: { Config_Settings, Container_Kind, Layout_State, Source_Language, Token_Info, Token_Kind, };

pub struct JS_TS_Formatter;

impl JS_TS_Formatter
{
    pub fn Format(Source: & str, Config: & Config_Settings)->Result<String, String>
    {
        let mut Lexer = JS_Lexer::New(Source, Config.Language);
        let Tokens = Lexer.TokenizeAll();

        let mut Builder = JS_Container_Builder::New(Tokens);
        let Nodes = Builder.BuildTree();

        let Emitter = JS_Emitter::New(Config);
        let Formatted = Emitter.Emit( & Nodes);
        // Safety gate verification
        if let Err(VerifyErr) = VerifyJSTokenEquivalence(Source, & Formatted, Config.Language)
        {
            return Err(format ! ("{}. Output aborted to prevent code corruption.", VerifyErr));
        }

        Ok(Formatted)
    }

    pub fn IsFormatted(Source: & str, Config: & Config_Settings)-> bool
    {
        match Self::Format(Source, Config) { Ok(Formatted) => Formatted == Source, Err(_) => false, }
    }
}

pub struct JS_Lexer<'a>
{
    Source: & 'a str, Chars: Vec<char>, Index: usize, Line: usize, Column: usize, Language: Source_Language, Expects_Expression: bool, 
}

impl <'a> JS_Lexer<'a>
{
    pub fn New(Source: & 'a str, Language: Source_Language)->Self
    {
        JS_Lexer { Source, Chars: Source.chars().collect(), Index: 0, Line: 1, Column: 0, Language, Expects_Expression: true, }
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
            // 3. Comments
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
            // 4. Regular Expression Literal vs Division
            if Ch == '/' && self.Expects_Expression
            {
                if let Some(RegexText) = self.ScanRegexLiteral()
                {
                    RawTokens.push
                    (
                        Token_Info::New(Token_Kind::Regex_Lit(RegexText.clone()), RegexText, StartLine, StartCol, PrecedingNewlines,)
                    );
                    PrecedingNewlines = 0;
                    self.Expects_Expression = false;
                    continue;
                }
            }
            // 5. Template Literals (Backticks)
            if Ch == '`'
            {
                let TemplateText = self.ScanTemplateLiteral();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Template_Lit(TemplateText.clone()), TemplateText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                self.Expects_Expression = false;
                continue;
            }
            // 6. Strings (Single and Double Quotes)
            if Ch == '"' || Ch == '\''
            {
                let QuoteChar = Ch;
                let StringText = self.ScanQuotedString(QuoteChar);
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::String_Lit(StringText.clone()), StringText, StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                self.Expects_Expression = false;
                continue;
            }
            // 7. Delimiters
            if Ch == '('
            {
                self.Advance();
                RawTokens.push
                (
                    Token_Info::New(Token_Kind::Open_Delim(Container_Kind::Paren),"(".to_string(), StartLine, StartCol, PrecedingNewlines,)
                );
                PrecedingNewlines = 0;
                self.Expects_Expression = true;
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
                self.Expects_Expression = false;
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
                self.Expects_Expression = true;
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
                self.Expects_Expression = false;
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
                self.Expects_Expression = true;
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
                self.Expects_Expression = false;
                continue;
            }
            // 8. Numbers
            if Ch.is_ascii_digit() || (Ch == '.' && self.PeekAhead(1).map_or(false, | C | C.is_ascii_digit()))
            {
                let NumberText = self.ScanNumber();
                RawTokens.push(Token_Info::New(Token_Kind::Number(NumberText.clone()), NumberText, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                self.Expects_Expression = false;
                continue;
            }
            // 9. Identifiers and Keywords
            if Ch.is_alphabetic() || Ch == '_' || Ch == '$'
            {
                let IdentText = self.ScanIdentifier();
                let IsKw = IsJSKeyword( & IdentText);
                let Kind = if IsKw { Token_Kind::Keyword(IdentText.clone()) } else { Token_Kind::Identifier(IdentText.clone()) };
                // Expression context tracking
                if IsKw
                {
                    self.Expects_Expression = matches!
                    (
                        IdentText.as_str(),"return" | "throw" | "yield" | "case" | "typeof" | "delete" | "void" | "in" | "instanceof" | "of" | "new" | "await"
                    );
                }
                else { self.Expects_Expression = false; }

                RawTokens.push(Token_Info::New(Kind, IdentText, StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                continue;
            }
            // 10. Punctuation
            if Ch == ';' || Ch == ',' || Ch == '.' || Ch == '?' || Ch == ':'
            {
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
                    self.Expects_Expression = true;
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
                    self.Expects_Expression = false;
                    continue;
                }

                self.Advance();
                RawTokens.push(Token_Info::New(Token_Kind::Punctuation(Ch), Ch.to_string(), StartLine, StartCol, PrecedingNewlines,));
                PrecedingNewlines = 0;
                self.Expects_Expression = matches!(Ch, ';' | ',' | ':' | '?');
                continue;
            }
            // 11. Operators
            let OpText = self.ScanOperator();
            RawTokens.push(Token_Info::New(Token_Kind::Operator(OpText.clone()), OpText.clone(), StartLine, StartCol, PrecedingNewlines,));
            PrecedingNewlines = 0;
            self.Expects_Expression = true;
        }
        // Post-process generic angle brackets in TypeScript
        if self.Language.SupportsAngleGenerics() { ResolveJSAngleBrackets( & mut RawTokens); }

        RawTokens
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

    fn ScanQuotedString( & mut self, QuoteChar: char)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // Opening quote

        while let Some(Ch) = self.Peek()
        {
            if Ch == '\\'
            {
                Result.push(self.Advance().unwrap());
                if self.Peek().is_some() { Result.push(self.Advance().unwrap()); }
                continue;
            }
            if Ch == QuoteChar { Result.push(self.Advance().unwrap()); break ; }
            if Ch == '\n'
            {
                // Unclosed string on newline
                break;
            }
            Result.push(self.Advance().unwrap());
        }
        Result
    }

    fn ScanTemplateLiteral( & mut self)->String
    {
        let mut Result = String:: new ();
        Result.push(self.Advance().unwrap());
        // '`'

        let mut InterpolationDepth = 0;

        while let Some(Ch) = self.Peek()
        {
            if Ch == '\\'
            {
                Result.push(self.Advance().unwrap());
                if self.Peek().is_some() { Result.push(self.Advance().unwrap()); }
                continue;
            }

            if Ch == '$' && self.PeekAhead(1) == Some('{')
            {
                Result.push(self.Advance().unwrap());
                // '$'
                Result.push(self.Advance().unwrap());
                // '{'
                InterpolationDepth += 1;
                continue;
            }

            if Ch == '{' && InterpolationDepth > 0 { Result.push(self.Advance().unwrap()); InterpolationDepth += 1; continue ; }

            if Ch == '}' && InterpolationDepth > 0 { Result.push(self.Advance().unwrap()); InterpolationDepth -= 1; continue ; }

            if Ch == '`' && InterpolationDepth == 0 { Result.push(self.Advance().unwrap()); break ; }

            Result.push(self.Advance().unwrap());
        }

        Result
    }

    fn ScanRegexLiteral( & mut self)->Option<String>
    {
        let mut TempIndex = self.Index;
        if TempIndex >= self.Chars.len() || self.Chars[TempIndex] != '/' { return None; }
        TempIndex += 1;
        // skip '/'

        let mut InCharClass = false;

        while TempIndex < self.Chars.len()
        {
            let Ch = self.Chars[TempIndex];
            if Ch == '\n'
            {
                // Regex literals cannot contain raw unescaped newlines
                return None;
            }
            if Ch == '\\'
            {
                TempIndex += 2;
                // skip escape and escaped char
                continue;
            }
            if Ch == '[' { InCharClass = true; TempIndex += 1; continue ; }
            if Ch == ']' && InCharClass { InCharClass = false; TempIndex += 1; continue ; }
            if Ch == '/' && ! InCharClass
            {
                TempIndex += 1;
                // Scan trailing flags: g, i, m, s, u, y, d, v
                while TempIndex < self.Chars.len() && self.Chars[TempIndex].is_ascii_alphabetic() { TempIndex += 1; }
                // Valid regex! Advance lexer to TempIndex and build string
                let mut Result = String:: new ();
                while self.Index < TempIndex { Result.push(self.Advance().unwrap()); }
                return Some(Result);
            }
            TempIndex += 1;
        }

        None
    }

    fn ScanNumber( & mut self)->String
    {
        let mut Result = String:: new ();
        // Check for 0x, 0b, 0o
        if self.Peek() == Some('0')
        {
            Result.push(self.Advance().unwrap());
            if let Some(Prefix) = self.Peek()
            {
                if matches!(Prefix, 'x' | 'X' | 'b' | 'B' | 'o' | 'O')
                {
                    Result.push(self.Advance().unwrap());
                    while let Some(Ch) = self.Peek()
                    {
                        if Ch.is_ascii_hexdigit() || Ch == '_' { Result.push(self.Advance().unwrap()); } else { break ; }
                    }
                    if self.Peek() == Some('n') { Result.push(self.Advance().unwrap()); }
                    return Result;
                }
            }
        }

        while let Some(Ch) = self.Peek()
        {
            if Ch.is_ascii_digit() || Ch == '.' || Ch == '_' { Result.push(self.Advance().unwrap()); }
            else if Ch == 'e' || Ch == 'E'
            {
                Result.push(self.Advance().unwrap());
                if let Some(Sign) = self.Peek() { if Sign == '+' || Sign == '-' { Result.push(self.Advance().unwrap()); } }
            }
            else if Ch == 'n'
            {
                // BigInt suffix
                Result.push(self.Advance().unwrap());
                break;
            }
            else { break ; }
        }
        Result
    }

    fn ScanIdentifier( & mut self)->String
    {
        let mut Result = String:: new ();
        while let Some(Ch) = self.Peek()
        {
            if Ch.is_alphanumeric() || Ch == '_' || Ch == '$' { Result.push(self.Advance().unwrap()); } else { break ; }
        }
        Result
    }

    fn ScanOperator( & mut self)->String
    {
        let mut Result = String:: new ();
        let First = self.Advance().unwrap();
        Result.push(First);

        let Next = self.Peek();
        match (First, Next)
        {
            ('=', Some('=')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('!', Some('=')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('=', Some('>')) => { Result.push(self.Advance().unwrap()); }
            ('&', Some('&')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('|', Some('|')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('?', Some('?')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('<', Some('<')) | ('>', Some('>')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('<', Some('=')) | ('>', Some('=')) => { Result.push(self.Advance().unwrap()); }
            ('+', Some('+')) | ('-', Some('-')) => { Result.push(self.Advance().unwrap()); }
            ('*', Some('*')) => 
            {
                Result.push(self.Advance().unwrap());
                if self.Peek() == Some('=') { Result.push(self.Advance().unwrap()); }
            }
            ('+', Some('=')) | ('-', Some('=')) | ('*', Some('=')) | ('/', Some('=')) | ('%', Some('=')) | ('&', Some('=')) | ('|', Some('=')) | 
            (
                '^', Some('=')
            )
            => { Result.push(self.Advance().unwrap()); }
            _ => {}
        }

        Result
    }
}

fn IsJSKeyword(Text: & str)-> bool
{
    matches!
    (
        Text, "async" | "await" | "break" | "case" | "catch" | "class" | "const" | "continue" | "debugger" | "default" | "delete" | "do" | "else" | "enum" | "export" | "extends" | "false" | "finally" | "for" | "from" | "function" | "get" | "if" | "implements" | "import" | "in" | "instanceof" | "interface" | "let" | "new" | "null" | "of" | "package" | "private" | "protected" | "public" | "readonly" | "return" | "set" | "static" | "super" | "switch" | "this" | "throw" | "true" | "try" | "type" | "typeof" | "var" | "void" | "while" | "with" | "yield" | "as" | "declare" | "namespace" | "module"
    )
}

fn ResolveJSAngleBrackets(Tokens: & mut [Token_Info])
{
    let Len = Tokens.len();
    let mut I = 0;

    while I < Len
    {
        if Tokens[I].Text == "<"
        {
            let IsPrecededByGenericContext = I > 0 && match & Tokens[I - 1].Kind
            {
                Token_Kind::Identifier(_) => true, Token_Kind::Keyword(K) => matches!(K.as_str(),"type" | "interface" | "class"), _ => false, 
            };

            if IsPrecededByGenericContext
            {
                let mut Depth = 1;
                let mut J = I + 1;
                let mut IsValidGeneric = false;

                while J < Len
                {
                    if Tokens[J].Text == "<" { Depth += 1; }
                    else if Tokens[J].Text == ">" { Depth -= 1; if Depth == 0 { IsValidGeneric = true; break ; } }
                    else if matches!(Tokens[J].Text.as_str(),";" | "{" | "}") { break ; }
                    J += 1;
                }

                if IsValidGeneric
                {
                    Tokens[I].Kind = Token_Kind::Open_Delim(Container_Kind::Angle);
                    Tokens[J].Kind = Token_Kind::Close_Delim(Container_Kind::Angle);
                }
            }
        }
        I += 1;
    }
}
#[derive(Debug, Clone)]
pub enum JS_Syntax_Node { Token(Token_Info), Container(JS_Container), }
#[derive(Debug, Clone)]
pub struct JS_Container
{
    pub Kind: Container_Kind, pub Open_Token: Token_Info, pub Close_Token: Option<Token_Info>, pub Children: Vec<JS_Syntax_Node>, pub Tail_Glue: Vec
    <
        Token_Info
    >,
    pub Has_Forced_Break: bool, pub Statement_Count: usize, pub Is_ASI_Hybrid: bool, 
}

impl JS_Container
{
    pub fn New(Kind: Container_Kind, Open_Token: Token_Info, Is_ASI_Hybrid: bool)->Self
    {
        JS_Container
        {
            Kind, Open_Token, Close_Token: None, Children: Vec:: new (), Tail_Glue: Vec:: new (), Has_Forced_Break: false, Statement_Count: 0, Is_ASI_Hybrid, 
        }
    }

    pub fn IsSoleChildChain( & self)-> bool
    {
        let NonCommentChildren: Vec <& JS_Syntax_Node > = self.Children.iter().filter
        (
            | Node | match Node { JS_Syntax_Node::Token(T) => ! T.IsTrivia() && ! T.IsComment(), JS_Syntax_Node::Container(_) => true, }
        )
        .collect();

        if NonCommentChildren.len() == 1 { matches ! (NonCommentChildren[0], JS_Syntax_Node::Container(_)) } else { false }
    }

    pub fn GetSoleChildContainer( & self)->Option <& JS_Container > 
    {
        let NonCommentChildren: Vec <& JS_Syntax_Node > = self.Children.iter().filter
        (
            | Node | match Node { JS_Syntax_Node::Token(T) => ! T.IsTrivia() && ! T.IsComment(), JS_Syntax_Node::Container(_) => true, }
        )
        .collect();

        if NonCommentChildren.len() == 1 { if let JS_Syntax_Node::Container(C) = NonCommentChildren[0] { return Some(C); } }
        None
    }

    pub fn CollectChain( & self)->Vec <& JS_Container > 
    {
        let mut Chain = vec![self];
        let mut Current = self;
        while let Some(Child) = Current.GetSoleChildContainer()
        {
            if Current.Kind == Container_Kind::Angle || Child.Kind == Container_Kind::Angle { break ; }
            Chain.push(Child);
            Current = Child;
        }
        Chain
    }

    pub fn CalculateSingleLineSpan( & self)->usize
    {
        if self.Has_Forced_Break { return usize::MAX; }

        let mut Total = self.Kind.OpenGlyph().len() + self.Kind.CloseGlyph().len();
        for Glue in & self.Tail_Glue { Total += Glue.Text.len(); }

        let mut PrevTokenNeedsSpace = false;
        for Node in & self.Children
        {
            match Node
            {
                JS_Syntax_Node::Token(T) => 
                {
                    if T.IsTrivia() { continue ; }
                    if T.IsComment() && matches!(T.Kind, Token_Kind::Comment_Line(_)) { return usize::MAX; }
                    if PrevTokenNeedsSpace { Total += 1; }
                    Total += T.Text.len();
                    PrevTokenNeedsSpace = true;
                }
                JS_Syntax_Node::Container(C) => 
                {
                    let ChildSpan = C.CalculateSingleLineSpan();
                    if ChildSpan == usize::MAX { return usize::MAX; }
                    if PrevTokenNeedsSpace { Total += 1; }
                    Total += ChildSpan;
                    PrevTokenNeedsSpace = true;
                }
            }
        }

        Total
    }

    pub fn CalculateBlockDepth( & self)->usize
    {
        let mut MaxChildDepth = 0;
        for Child in & self.Children
        {
            if let JS_Syntax_Node::Container(Sub) = Child
            {
                let SubDepth = Sub.CalculateBlockDepth();
                if SubDepth > MaxChildDepth { MaxChildDepth = SubDepth; }
            }
        }
        1 + MaxChildDepth
    }

    pub fn CountTotalContainers( & self)->usize
    {
        let mut Count = 1;
        for Child in & self.Children { if let JS_Syntax_Node::Container(Sub) = Child { Count += Sub.CountTotalContainers(); } }
        Count
    }
}

pub struct JS_Container_Builder { Tokens: Vec<Token_Info>, Index: usize, }

impl JS_Container_Builder
{
    pub fn New(Tokens: Vec<Token_Info>)->Self { JS_Container_Builder { Tokens, Index: 0 } }

    pub fn BuildTree( & mut self)->Vec<JS_Syntax_Node>
    {
        let mut RootNodes = Vec:: new ();
        while self.Index < self.Tokens.len() { if let Some(Node) = self.ParseNextNode(false) { RootNodes.push(Node); } }
        RootNodes
    }

    fn ParseNextNode( & mut self, PrecededByReturn: bool)->Option<JS_Syntax_Node>
    {
        if self.Index >= self.Tokens.len() { return None; }
        let Token = self.Tokens[self.Index].clone();
        self.Index += 1;

        match & Token.Kind
        {
            Token_Kind::Open_Delim(Kind) => 
            {
                let mut Container = JS_Container::New( * Kind, Token, PrecededByReturn);
                self.PopulateContainerChildren( & mut Container);
                Some(JS_Syntax_Node::Container(Container))
            }
            _ => Some(JS_Syntax_Node::Token(Token)),
        }
    }

    fn PopulateContainerChildren( & mut self, Container: & mut JS_Container)
    {
        let mut StatementCount = 0;
        let mut LastSignificantWasReturn = false;

        while self.Index < self.Tokens.len()
        {
            let CurrentToken = self.Tokens[self.Index].clone();

            if CurrentToken.Preceding_Newlines > 0 { Container.Has_Forced_Break = true; }

            match & CurrentToken.Kind
            {
                Token_Kind::Close_Delim(CloseKind) if * CloseKind == Container.Kind => 
                {
                    self.Index += 1;
                    Container.Close_Token = Some(CurrentToken);
                    // Consume trailing glue (; or ,)
                    while self.Index < self.Tokens.len()
                    {
                        let NextToken = & self.Tokens[self.Index];
                        if NextToken.Preceding_Newlines == 0 && (NextToken.Text == ";" || NextToken.Text == ",")
                        {
                            Container.Tail_Glue.push(NextToken.clone());
                            self.Index += 1;
                        }
                        else { break ; }
                    }
                    break;
                }
                Token_Kind::Open_Delim(_) => 
                {
                    if let Some(ChildNode) = self.ParseNextNode(LastSignificantWasReturn) { Container.Children.push(ChildNode); }
                    LastSignificantWasReturn = false;
                }
                Token_Kind::Comment_Line(_) => 
                {
                    Container.Has_Forced_Break = true;
                    Container.Children.push(JS_Syntax_Node::Token(CurrentToken));
                    self.Index += 1;
                    LastSignificantWasReturn = false;
                }
                _ => 
                {
                    if CurrentToken.Text == ";" { StatementCount += 1; }

                    if ! CurrentToken.IsTrivia() && ! CurrentToken.IsComment()
                    {
                        LastSignificantWasReturn = matches!(CurrentToken.Text.as_str(),"return" | "throw" | "yield");
                    }

                    Container.Children.push(JS_Syntax_Node::Token(CurrentToken));
                    self.Index += 1;
                }
            }
        }

        Container.Statement_Count = StatementCount;
    }
}

pub struct JS_Layout_Planner<'a> { Config: & 'a Config_Settings, }

impl <'a> JS_Layout_Planner<'a>
{
    pub fn New(Config: & 'a Config_Settings)->Self { JS_Layout_Planner { Config } }

    pub fn DecideLayout( & self, Container: & JS_Container, CurrentIndentLevel: usize, CurrentColumn: usize,)->Layout_State
    {
        // 1. Monolith First (Horizontal Density)
        if ! Container.Has_Forced_Break && Container.Statement_Count <= self.Config.Max_Statements_In_Monolith && Container.CalculateBlockDepth() <= self.Config.Max_Block_Depth && Container.CountTotalContainers
        (
        )
        <= self.Config.Max_Containers_In_Monolith
        {
            let SingleSpan = Container.CalculateSingleLineSpan();
            if SingleSpan != usize::MAX
            {
                let BaseCol = if CurrentColumn > 0 { CurrentColumn } else { CurrentIndentLevel * self.Config.Indent_Width };
                let ProjectedCol = BaseCol + SingleSpan;
                if ProjectedCol <= self.Config.Canvas_Width { return Layout_State::Monolith; }
            }
        }
        // 2. Brace Grouping Second (Sole-Child Chain Compound Delimiters)
        if self.Config.Allow_Brace_Grouping && Container.IsSoleChildChain()
        {
            let Chain = Container.CollectChain();
            if Chain.len() >= 2 { return Layout_State::Grouped; }
        }
        // 3. Expanded Symmetry Last (Hierarchical Architecture)
        Layout_State::Expanded
    }
}

pub struct JS_Emitter<'a>
{
    Config: & 'a Config_Settings, Planner: JS_Layout_Planner<'a>, Output: String, Current_Line_Len: usize, Line_Has_Content: bool, 
}

impl <'a> JS_Emitter<'a>
{
    pub fn New(Config: & 'a Config_Settings)->Self
    {
        JS_Emitter { Config, Planner: JS_Layout_Planner::New(Config), Output: String::new (), Current_Line_Len: 0, Line_Has_Content: false, }
    }

    pub fn Emit(mut self, Nodes: & [JS_Syntax_Node])->String
    {
        self.EmitNodeList(Nodes, 0);
        if ! self.Output.ends_with('\n') { self.Output.push('\n'); }
        self.Output
    }

    fn WriteNewline( & mut self)
    {
        while self.Output.ends_with(' ') { self.Output.pop(); }
        self.Output.push('\n');
        self.Current_Line_Len = 0;
        self.Line_Has_Content = false;
    }

    fn EnsureNewline( & mut self) { if self.Line_Has_Content { self.WriteNewline(); } }

    fn EmitIndent( & mut self, Level: usize)
    {
        if ! self.Line_Has_Content
        {
            let IndentStr = self.Config.IndentString(Level);
            self.Output.push_str( & IndentStr);
            self.Current_Line_Len = IndentStr.len();
            self.Line_Has_Content = true;
        }
    }

    fn EmitRaw( & mut self, Text: & str) { self.Output.push_str(Text); self.Current_Line_Len += Text.len(); self.Line_Has_Content = true; }

    fn EmitSpace( & mut self)
    {
        if self.Line_Has_Content && self.Current_Line_Len > 0 && ! self.Output.ends_with(' ') && ! self.Output.ends_with('\n')
        {
            self.Output.push(' ');
            self.Current_Line_Len += 1;
        }
    }

    fn EmitNodeList( & mut self, Nodes: & [JS_Syntax_Node], Level: usize)
    {
        let Total = Nodes.len();
        let mut Index = 0;

        while Index < Total
        {
            let Node = & Nodes[Index];

            match Node
            {
                JS_Syntax_Node::Token(T) => { self.EmitSingleToken(T, Level); }
                JS_Syntax_Node::Container(C) => 
                {
                    let Layout = self.Planner.DecideLayout(C, Level, self.Current_Line_Len);
                    self.EmitContainerNodeWithLayout(C, Level, Layout);
                    // Check if followed by continuation keyword (else, catch, finally)
                    let NextContinuation = if Index + 1 < Total
                    {
                        match & Nodes[Index + 1]
                        {
                            JS_Syntax_Node::Token(T) if matches!(T.Text.as_str(),"else" | "catch" | "finally") => { Some(T.Text.clone()) }
                            _ => None, 
                        }
                    }
                    else { None };

                    if NextContinuation.is_some() { self.EnsureNewline(); }
                    else if C.Tail_Glue.iter().any( | G | G.Text == ";") || C.Kind == Container_Kind::Brace { self.EnsureNewline(); }
                }
            }

            Index += 1;
        }
    }

    fn EmitSingleToken( & mut self, Token: & Token_Info, Level: usize)
    {
        if let Token_Kind::Comment_Line(CommentText) = & Token.Kind
        {
            if self.Line_Has_Content { self.EmitSpace(); } else { self.EmitIndent(Level); }
            self.EmitRaw(CommentText);
            self.WriteNewline();
            return ;
        }

        if let Token_Kind::Comment_Block(CommentText) = & Token.Kind
        {
            if ! self.Line_Has_Content { self.EmitIndent(Level); } else { self.EmitSpace(); }
            self.EmitRaw(CommentText);
            return ;
        }

        if Token.IsTrivia() { return ; }

        if Token.Preceding_Newlines > 0
        {
            let BlankLines = (Token.Preceding_Newlines - 1).min(self.Config.Max_Blank_Lines);
            self.EnsureNewline();
            for _ in 0..BlankLines { self.WriteNewline(); }
        }

        if ! self.Line_Has_Content { self.EmitIndent(Level); } else { self.EmitSpaceBeforeToken(Token); }

        self.EmitRaw( & Token.Text);
        if Token.Text == ";" { self.WriteNewline(); }
        else if let Token_Kind::Keyword(K) = & Token.Kind
        {
            if matches!
            (
                K.as_str(),"if" | "while" | "for" | "switch" | "catch" | "with" | "return" | "throw" | "yield" | "case" | "new" | "typeof" | "void" | "delete" | "as" | "from" | "export" | "import" | "const" | "let" | "var" | "function" | "class" | "interface" | "type" | "enum" | "extends" | "implements" | "async" | "await" | "default" | "else"
            )
            { self.EmitSpace(); }
        }
        else if let Token_Kind::Operator(Op) = & Token.Kind
        {
            if matches!
            (
                Op.as_str(),"=" | "=>" | "===" | "!==" | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "??" | "+=" | "-=" | "*=" | "/=" | "%=" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
            )
            { self.EmitSpace(); }
        }
    }

    fn EmitSpaceBeforeToken( & mut self, Token: & Token_Info)
    {
        if Token.Text == ";" || Token.Text == "," || Token.Text == "." || Token.Text == "?." { return ; }
        if self.Output.ends_with('.') || self.Output.ends_with("?.") { return ; }
        if self.Output.ends_with('(') || self.Output.ends_with('[') { return ; }
        if Token.Text == ":"
        {
            // Colon in JS objects or TS types usually sits directly after identifier
            return ;
        }
        self.EmitSpace();
    }

    fn EmitContainerNodeWithLayout( & mut self, Container: & JS_Container, Level: usize, Layout: Layout_State,)
    {
        match Layout
        {
            Layout_State::Monolith => { self.EmitMonolith(Container, Level); }
            Layout_State::Grouped => { self.EmitGrouped(Container, Level); }
            Layout_State::Expanded => { self.EmitExpanded(Container, Level); }
        }
    }

    fn EmitMonolith( & mut self, Container: & JS_Container, Level: usize)
    {
        if ! self.Line_Has_Content { self.EmitIndent(Level); }
        else if Container.Kind == Container_Kind::Brace { self.EmitSpace(); }
        else if Container.Kind == Container_Kind::Paren
        {
            if ! self.Output.ends_with(' ')
            {
                let LastChar = self.Output.chars().last();
                if let Some(Ch) = LastChar
                {
                    if ! Ch.is_alphanumeric() && Ch != '_' && Ch != '$' && Ch != ')' && Ch != ']' && Ch != '.' { self.EmitSpace(); }
                }
            }
        }

        self.EmitRaw(Container.Kind.OpenGlyph());

        let mut PrevNeedsSpace = matches!(Container.Kind, Container_Kind::Brace);
        for Child in & Container.Children
        {
            match Child
            {
                JS_Syntax_Node::Token(T) => 
                {
                    if T.IsTrivia() { continue ; }
                    let IsPunct = T.Text == ";" || T.Text == "," || T.Text == "." || T.Text == ":" || T.Text == "?.";
                    if PrevNeedsSpace && ! IsPunct && ! self.Output.ends_with('.') && ! self.Output.ends_with("?.") && ! self.Output.ends_with('(') && ! self.Output.ends_with
                    (
                        '['
                    )
                    { self.EmitSpace(); }
                    self.EmitRaw( & T.Text);
                    PrevNeedsSpace = ! matches!(T.Text.as_str(),"." | "?." | "(" | "[" | "!" | "~");
                }
                JS_Syntax_Node::Container(Sub) => 
                {
                    if PrevNeedsSpace && Sub.Kind == Container_Kind::Brace { self.EmitSpace(); }
                    self.EmitMonolith(Sub, Level);
                    PrevNeedsSpace = true;
                }
            }
        }

        if matches!(Container.Kind, Container_Kind::Brace) && ! self.Output.ends_with(' ') && ! self.Output.ends_with('{') { self.EmitSpace(); }

        self.EmitRaw(Container.Kind.CloseGlyph());
        for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
    }

    fn EmitGrouped( & mut self, Container: & JS_Container, Level: usize)
    {
        let Chain = Container.CollectChain();
        let OpenRun: String = Chain.iter().map( | C | C.Kind.OpenGlyph()).collect();
        let CloseRun: String = Chain.iter().rev().map( | C | C.Kind.CloseGlyph()).collect();

        self.EnsureNewline();
        self.EmitIndent(Level);
        self.EmitRaw( & OpenRun);
        self.WriteNewline();

        if let Some(Innermost) = Chain.last() { self.EmitNodeList( & Innermost.Children, Level + 1); }

        self.EnsureNewline();
        self.EmitIndent(Level);
        self.EmitRaw( & CloseRun);

        if let Some(Outermost) = Chain.first() { for Glue in & Outermost.Tail_Glue { self.EmitRaw( & Glue.Text); } }
        self.WriteNewline();
    }

    fn EmitExpanded( & mut self, Container: & JS_Container, Level: usize)
    {
        // Check Column-Anchored Hybrid Exception (ASI protection after return / throw / yield)
        if Container.Is_ASI_Hybrid
        {
            self.EmitSpace();
            self.EmitRaw(Container.Kind.OpenGlyph());
            self.WriteNewline();

            self.EmitNodeList( & Container.Children, Level + 1);

            self.EnsureNewline();
            self.EmitIndent(Level);
            self.EmitRaw(Container.Kind.CloseGlyph());
            for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
            self.WriteNewline();
            return ;
        }
        // Standard OLAFS Detached Delimiter Symmetry
        self.EnsureNewline();
        self.EmitIndent(Level);
        self.EmitRaw(Container.Kind.OpenGlyph());
        self.WriteNewline();

        self.EmitNodeList( & Container.Children, Level + 1);

        self.EnsureNewline();
        self.EmitIndent(Level);
        self.EmitRaw(Container.Kind.CloseGlyph());
        for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
        self.WriteNewline();
    }
}

pub fn VerifyJSTokenEquivalence(OriginalSource: & str, FormattedSource: & str, Language: Source_Language,)->Result<(), String>
{
    let mut OriginalScanner = JS_Lexer::New(OriginalSource, Language);
    let OriginalTokens = OriginalScanner.TokenizeAll();

    let mut FormattedScanner = JS_Lexer::New(FormattedSource, Language);
    let FormattedTokens = FormattedScanner.TokenizeAll();

    let OrigSignificant: Vec <& str > = OriginalTokens.iter().filter( | T | ! matches ! (T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline)).map
    (
        | T | T.Text.as_str()
    )
    .collect();

    let FormSignificant: Vec <& str > = FormattedTokens.iter().filter( | T | ! matches ! (T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline)).map
    (
        | T | T.Text.as_str()
    )
    .collect();

    if OrigSignificant.len() != FormSignificant.len()
    {
        return Err
        (
            format!("Safety Gate Failed: Significant token count mismatch! (Original: {}, Formatted: {})", OrigSignificant.len(), FormSignificant.len())
        );
    }

    for (Index, (Orig, Form)) in OrigSignificant.iter().zip(FormSignificant.iter()).enumerate()
    {
        if Orig != Form { return Err(format ! ("Safety Gate Failed: Token #{} mismatch! (Expected '{}', Got '{}')", Index, Orig, Form)); }
    }

    Ok(())
}
