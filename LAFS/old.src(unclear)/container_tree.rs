#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::types:: { Container_Kind, Token_Info, Token_Kind };
#[derive(Debug, Clone)]
pub enum Syntax_Node { Token(Token_Info), Container(Container_Node) }
#[derive(Debug, Clone)]
pub struct Container_Node
{
    pub Kind: Container_Kind, pub Open_Token: Token_Info, pub Close_Token: Option < Token_Info > , pub Children: Vec < Syntax_Node > , pub Tail_Glue: Vec < Token_Info > , pub Has_Forced_Break: bool, pub Statement_Count: usize
}

impl Container_Node
{
    pub fn New(Kind: Container_Kind, Open_Token: Token_Info)->Self
    {
        Container_Node
        {
            Kind, Open_Token, Close_Token: None, Children: Vec:: new (), Tail_Glue: Vec:: new (), Has_Forced_Break: false, Statement_Count: 0
        }
    }

    pub fn IsSoleChildChain( & self)-> bool
    {
        let NonCommentChildren: Vec <& Syntax_Node > = self.Children.iter().filter
        (
            | Node | match Node { Syntax_Node::Token(T) => ! T.IsTrivia() && ! T.IsComment(), Syntax_Node::Container(_) => true }
        )
        .collect();

        if NonCommentChildren.len() == 1 { matches ! (NonCommentChildren[0], Syntax_Node::Container(_)) } else { false }
    }

    pub fn GetSoleChildContainer( & self)->Option <& Container_Node > 
    {
        let NonCommentChildren: Vec <& Syntax_Node > = self.Children.iter().filter
        (
            | Node | match Node { Syntax_Node::Token(T) => ! T.IsTrivia() && ! T.IsComment(), Syntax_Node::Container(_) => true }
        )
        .collect();

        if NonCommentChildren.len() == 1 { if let Syntax_Node::Container(C) = NonCommentChildren[0] { return Some(C); } }
        None
    }

    pub fn CollectChain( & self)->Vec <& Container_Node > 
    {
        let mut Chain = vec![self];
        let mut Current = self;
        while let Some(Child) = Current.GetSoleChildContainer()
        {
            // Angle containers never fuse into runs as per spec (angle.allow_grouping = false)
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
        if self.Kind == Container_Kind::Brace && ! self.Children.is_empty()
        {
            Total += 2;
            // "{ ... }" has inner padding spaces
        }

        for Child in & self.Children
        {
            match Child
            {
                Syntax_Node::Token(T) => { Total += T.Text.len() + 1; }
                Syntax_Node::Container(C) => 
                {
                    let SubSpan = C.CalculateSingleLineSpan();
                    if SubSpan == usize::MAX { return usize::MAX; }
                    Total += SubSpan + 1;
                }
            }
        }

        for Glue in & self.Tail_Glue { Total += Glue.Text.len(); }

        Total
    }

    pub fn CalculateBlockDepth( & self)->usize
    {
        let mut MaxChildDepth = 0;
        for Child in & self.Children
        {
            if let Syntax_Node::Container(C) = Child { let D = C.CalculateBlockDepth(); if D > MaxChildDepth { MaxChildDepth = D; } }
        }
        if self.Kind == Container_Kind::Brace { 1 + MaxChildDepth } else { MaxChildDepth }
    }

    pub fn CountTotalContainers( & self)->usize
    {
        let mut Total = 1;
        for Child in & self.Children { if let Syntax_Node::Container(C) = Child { Total += C.CountTotalContainers(); } }
        Total
    }
}

pub struct Container_Tree_Builder { Tokens: Vec < Token_Info > , Index: usize }

impl Container_Tree_Builder
{
    pub fn New(Tokens: Vec < Token_Info >)->Self { Container_Tree_Builder { Tokens, Index: 0 } }

    pub fn BuildTree( & mut self)->Vec < Syntax_Node > 
    {
        let mut RootNodes = Vec:: new ();
        while self.Index < self.Tokens.len() { if let Some(Node) = self.ParseNode() { RootNodes.push(Node); } }
        RootNodes
    }

    fn ParseNode( & mut self)->Option < Syntax_Node > 
    {
        if self.Index >= self.Tokens.len() { return None; }
        let Current = self.Tokens[self.Index].clone();

        match Current.Kind
        {
            Token_Kind::Open_Delim(Kind) => 
            {
                self.Index += 1;
                let mut Container = Container_Node::New(Kind, Current);

                while self.Index < self.Tokens.len()
                {
                    let Next = & self.Tokens[self.Index];
                    if let Token_Kind::Close_Delim(CloseKind) = Next.Kind
                    {
                        if CloseKind == Kind { Container.Close_Token = Some(self.Tokens[self.Index].clone()); self.Index += 1; break ; }
                    }
                    // Check for forced break conditions inside container per LAFS specification (R1)
                    if matches!(Next.Kind, Token_Kind::Comment_Line(_)) { Container.Has_Forced_Break = true; }
                    if matches!(Next.Kind, Token_Kind::Preprocessor(_)) { Container.Has_Forced_Break = true; }
                    if Next.Text.contains('\n') { Container.Has_Forced_Break = true; }
                    if matches!(Next.Kind, Token_Kind::Keyword(ref K) if K == "public" || K == "private" || K == "protected")
                    {
                        if self.Index + 1 < self.Tokens.len() && self.Tokens[self.Index + 1].Text == ":" { Container.Has_Forced_Break = true; }
                    }
                    if Next.Text == ";" { Container.Statement_Count += 1; }

                    if let Some(ChildNode) = self.ParseNode()
                    {
                        if let Syntax_Node::Container(ref ChildC) = ChildNode { if ChildC.Has_Forced_Break { Container.Has_Forced_Break = true; } }
                        Container.Children.push(ChildNode);
                    }
                }
                // Collect tail glue immediately following closing delimiter (e.g. ';', ',')
                while self.Index < self.Tokens.len()
                {
                    let Next = & self.Tokens[self.Index];
                    if Next.Preceding_Newlines > 0 { break ; }
                    if Next.Text == ";" || Next.Text == "," { Container.Tail_Glue.push(self.Tokens[self.Index].clone()); self.Index += 1; } else { break ; }
                }

                Some(Syntax_Node::Container(Container))
            }
            _ => { self.Index += 1; Some(Syntax_Node::Token(Current)) }
        }
    }
}
