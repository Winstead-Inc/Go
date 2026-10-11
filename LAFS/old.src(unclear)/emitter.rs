#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::container_tree:: { Container_Node, Syntax_Node };
use crate::planner::Layout_Planner;
use crate::types:: { Config_Settings, Container_Kind, Layout_State, Token_Info, Token_Kind };

pub struct Code_Emitter<'a>
{
    Config: &'a Config_Settings, Planner: Layout_Planner<'a>,
    Output: String,
    Current_Line_Len: usize,
    Line_Has_Content: bool
}

impl<'a > Code_Emitter<'a>
{
    pub fn New(Config: &'a Config_Settings)->Self
{
    Code_Emitter { Config, Planner: Layout_Planner::New(Config), Output: String::new (), Current_Line_Len: 0, Line_Has_Content: false }
}

pub fn Emit(mut self, Nodes: & [Syntax_Node])->String
{
    self.EmitNodeList(Nodes, 0);
    if ! self.Output.ends_with('\n') { self.Output.push('\n'); }
    self.Output
}

fn WriteNewline( & mut self) { self.Output.push('\n'); self.Current_Line_Len = 0; self.Line_Has_Content = false; }

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

fn EmitNodeList( & mut self, Nodes: & [Syntax_Node], Level: usize)
{
    let Total = Nodes.len();
    let mut Index = 0;

    while Index < Total
    {
        let Node = & Nodes[Index];

        match Node
        {
            Syntax_Node::Token(T) => { self.EmitSingleToken(T, Level); }
            Syntax_Node::Container(C) => 
            {
                let IsTemplate = Index > 0 && matches!( & Nodes[Index - 1], Syntax_Node::Token(T) if T.Text == "template");
                let Layout = self.Planner.DecideLayout(C, Level, self.Current_Line_Len);
                self.EmitContainerNodeWithLayout(C, Level, Layout);
                // Check if followed by continuation keyword (else, catch, finally, while)
                let NextContinuation = if Index + 1 < Total
                {
                    match & Nodes[Index + 1]
                    {
                        Syntax_Node::Token(T) if matches!(T.Text.as_str(),"else" | "catch" | "finally" | "while") => Some(T.Text.clone()), _ => None
                    }
                }
                else { None };

                if let Some(ContKw) = NextContinuation
                {
                    if Layout == Layout_State::Monolith
                    {
                        // Check if the rest of the clause fits on the same line
                        let CanInline = if Index + 2 < Total
                        {
                            match & Nodes[Index + 2]
                            {
                                Syntax_Node::Container(NextC) => 
                                {
                                    if ! NextC.Has_Forced_Break
                                    {
                                        let Span = NextC.CalculateSingleLineSpan();
                                        Span != usize::MAX && self.Current_Line_Len + 1 + ContKw.len() + 1 + Span <= self.Config.Canvas_Width
                                    }
                                    else { false }
                                }
                                _ => false
                            }
                        }
                        else { false };

                        if CanInline { self.EmitSpace(); } else { self.EnsureNewline(); }
                    }
                    else { self.EnsureNewline(); }
                }
                else if C.Tail_Glue.iter().any( | G | G.Text == ";") || C.Kind == Container_Kind::Brace || (C.Kind == Container_Kind::Angle && IsTemplate)
                {
                    self.EnsureNewline();
                }
            }
        }

        Index += 1;
    }
}

fn EmitSingleToken( & mut self, Token: & Token_Info, Level: usize)
{
    // 1. Preprocessor directives must always sit on their own line
    if let Token_Kind::Preprocessor(DirectiveText) = & Token.Kind { self.EnsureNewline(); self.EmitRaw(DirectiveText); self.WriteNewline(); return ; }
    // 2. Line comment
    if let Token_Kind::Comment_Line(CommentText) = & Token.Kind
    {
        if self.Line_Has_Content { self.EmitSpace(); } else { self.EmitIndent(Level); }
        self.EmitRaw(CommentText);
        self.WriteNewline();
        return ;
    }
    // 3. Block comment
    if let Token_Kind::Comment_Block(CommentText) = & Token.Kind
    {
        if Token.Preceding_Newlines > 0 && ! self.Line_Has_Content
        {
            if Token.Preceding_Newlines >= 2 { self.WriteNewline(); }
            self.EmitIndent(Level);
        }
        else if self.Line_Has_Content { self.EmitSpace(); }
        self.EmitRaw(CommentText);
        return ;
    }
    // 4. Preserve intentional blank line if preceded by 2 or more newlines, clamped to at most one empty line
    if Token.Preceding_Newlines >= 2 && ! self.Output.ends_with("\n\n")
    {
        self.Output.push('\n');
        self.Current_Line_Len = 0;
        self.Line_Has_Content = false;
    }
    // 5. If line is empty, apply indentation
    if ! self.Line_Has_Content { self.EmitIndent(Level); }
    // 6. Formatting token text and spacing
    match & Token.Kind
    {
        Token_Kind::Punctuation(';') => 
        {
            self.EmitRaw(";");
            // In C-family, statements generally end with newline unless inside a for-header
            self.WriteNewline();
        }
        Token_Kind::Punctuation(',') => { self.EmitRaw(","); self.EmitSpace(); }
        Token_Kind::Punctuation(':') => 
        {
            if self.Output.ends_with("public ") || self.Output.ends_with("private ") || self.Output.ends_with("protected ") || self.Output.ends_with
            (
                "default "
            )
{ self.Output.pop(); self.Current_Line_Len -= 1; self.EmitRaw(":"); self.WriteNewline(); } else { self.EmitRaw(":"); self.EmitSpace(); }
        }
        Token_Kind::Punctuation('.') => { self.EmitRaw("."); }
        Token_Kind::Operator(Op) => 
        {
            if Op == "::" || Op == "->" || Op == "?."
            {
                if self.Output.ends_with(' ') { self.Output.pop(); self.Current_Line_Len -= 1; }
                self.EmitRaw(Op);
            }
            else if Op == "++" || Op == "--" || Op == "!" || Op == "~" { self.EmitRaw(Op); }
            else { self.EmitSpace(); self.EmitRaw(Op); self.EmitSpace(); }
        }
        Token_Kind::Keyword(K) => 
        {
            if self.Line_Has_Content && self.Current_Line_Len > 0 && ! self.Output.ends_with(' ') && ! self.Output.ends_with('\n') { self.EmitSpace(); }
            self.EmitRaw(K);
            if matches!
            (
                K.as_str(),"if" | "while" | "for" | "switch" | "catch" | "return" | "class" | "struct" | "enum" | "namespace" | "using" | "new" | "case" | "throw" | "public" | "private" | "protected" | "static" | "virtual" | "override" | "const"
            )
{ self.EmitSpace(); }
        }
        Token_Kind::Identifier(Ident) => 
        {
            if self.Line_Has_Content && self.Current_Line_Len > 0 && ! self.Output.ends_with(' ') && ! self.Output.ends_with('\n') && ! self.Output.ends_with
            (
                '.'
            )
            && ! self.Output.ends_with("->") && ! self.Output.ends_with("::") { self.EmitSpace(); }
            self.EmitRaw(Ident);
        }
        _ => { self.EmitRaw( & Token.Text); }
    }
}

fn EmitContainerNode( & mut self, Container: & Container_Node, Level: usize)
{
    let Layout = self.Planner.DecideLayout(Container, Level, self.Current_Line_Len);
    self.EmitContainerNodeWithLayout(Container, Level, Layout);
}

fn EmitContainerNodeWithLayout( & mut self, Container: & Container_Node, Level: usize, Layout: Layout_State)
{
    match Layout
    {
        Layout_State::Monolith => { self.EmitMonolith(Container); }
        Layout_State::Grouped => { self.EmitGrouped(Container, Level); }
        Layout_State::Expanded => { self.EmitExpanded(Container, Level); }
    }
}

fn EmitMonolith( & mut self, Container: & Container_Node)
{
    // For Monolith: openers and closers remain horizontal on the same line
    if Container.Kind == Container_Kind::Brace { self.EmitSpace(); }

    self.EmitRaw(Container.Kind.OpenGlyph());

    if Container.Kind == Container_Kind::Brace && ! Container.Children.is_empty() { self.EmitRaw(" "); }

    for Child in & Container.Children
    {
        match Child
        {
            Syntax_Node::Token(T) => 
            {
                if T.Text == ";" { self.EmitRaw(";"); self.EmitSpace(); }
                else if T.Text == "," { self.EmitRaw(","); self.EmitSpace(); }
                else if T.Text == "." { if self.Output.ends_with(' ') { self.Output.pop(); self.Current_Line_Len -= 1; } self.EmitRaw("."); }
                else if T.Text == ":"
                {
                    if self.Output.ends_with("public ") || self.Output.ends_with("private ") || self.Output.ends_with("protected ") || self.Output.ends_with
                    (
                        "default "
                    )
{ self.Output.pop(); self.Current_Line_Len -= 1; self.EmitRaw(":"); self.EmitSpace(); } else { self.EmitRaw(":"); self.EmitSpace(); }
                }
                else if T.Text == "::" || T.Text == "->" || T.Text == "?."
                {
                    if self.Output.ends_with(' ') { self.Output.pop(); self.Current_Line_Len -= 1; }
                    self.EmitRaw( & T.Text);
                }
                else if matches!(T.Kind, Token_Kind::Operator(_)) { self.EmitSpace(); self.EmitRaw( & T.Text); self.EmitSpace(); }
                else
                {
                    let NeedSpace = if self.Output.ends_with('(') || self.Output.ends_with('[') || self.Output.ends_with('<') || self.Output.ends_with('{') || self.Output.ends_with
                    (
                        ' '
                    )
                    || self.Output.ends_with("::") || self.Output.ends_with("->") || self.Output.ends_with('.') { false }
                    else { matches ! (T.Kind, Token_Kind::Keyword(_) | Token_Kind::Identifier(_) | Token_Kind::Number(_)) };

                    if NeedSpace { self.EmitSpace(); }
                    self.EmitRaw( & T.Text);
                    if matches!(T.Kind, Token_Kind::Keyword(_)) { self.EmitSpace(); }
                }
            }
            Syntax_Node::Container(SubC) => { self.EmitMonolith(SubC); }
        }
    }

    if Container.Kind == Container_Kind::Brace && ! Container.Children.is_empty() && ! self.Output.ends_with(' ') { self.EmitRaw(" "); }

    if (Container.Kind == Container_Kind::Paren || Container.Kind == Container_Kind::Bracket || Container.Kind == Container_Kind::Angle) && self.Output.ends_with
    (
        ' '
    )
{ self.Output.pop(); self.Current_Line_Len -= 1; }

    self.EmitRaw(Container.Kind.CloseGlyph());

    for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
}

fn EmitGrouped( & mut self, Container: & Container_Node, Level: usize)
{
    let Chain = Container.CollectChain();
    let OpenRun: String = Chain.iter().map( | C | C.Kind.OpenGlyph()).collect();
    let CloseRun: String = Chain.iter().rev().map( | C | C.Kind.CloseGlyph()).collect();
    let Innermost = Chain.last().unwrap();
    // 1. Head stays on current line, then break line
    self.EnsureNewline();
    // 2. Compound opening run sits on its own line at Level indent (Load-bearing pillar)
    self.EmitIndent(Level);
    self.EmitRaw( & OpenRun);
    self.WriteNewline();
    // 3. Innermost children indented at Level + 1
    self.EmitNodeList( & Innermost.Children, Level + 1);
    self.EnsureNewline();
    // Remove trailing empty line right before compound closer
    if self.Output.ends_with("\n\n") { self.Output.pop(); }
    // 4. Compound closing run sits on its own line at Level indent (Load-bearing pillar)
    self.EmitIndent(Level);
    self.EmitRaw( & CloseRun);
    // 5. Tail glue from outermost container sits right after close run
    for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
    self.WriteNewline();
}

fn EmitExpanded( & mut self, Container: & Container_Node, Level: usize)
{
    // 1. Head stays on current line, then break line (Anti-Hugging Law)
    self.EnsureNewline();
    // 2. Opening delimiter sits on its own line at Level indent (Load-bearing pillar)
    self.EmitIndent(Level);
    self.EmitRaw(Container.Kind.OpenGlyph());
    self.WriteNewline();
    // 3. Inner children indented at Level + 1
    self.EmitNodeList( & Container.Children, Level + 1);
    self.EnsureNewline();
    // Remove trailing empty line right before closer
    if self.Output.ends_with("\n\n") { self.Output.pop(); }
    // 4. Closing delimiter sits on its own line at Level indent (Translational symmetry)
    self.EmitIndent(Level);
    self.EmitRaw(Container.Kind.CloseGlyph());
    // 5. Tail glue (';', ',') sits right after closing delimiter
    for Glue in & Container.Tail_Glue { self.EmitRaw( & Glue.Text); }
    // In C/C++, a closing brace of a function, struct, or block is followed by a newline
    self.WriteNewline();
}
}
