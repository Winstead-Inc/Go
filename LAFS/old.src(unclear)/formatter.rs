#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::container_tree::Container_Tree_Builder;
use crate::emitter::Code_Emitter;
use crate::lexer::Lexer_Scanner;
use crate::types::Config_Settings;
use crate::verifier::Safety_Verifier;

pub struct Source_Formatter;

impl Source_Formatter
{
    pub fn Format(Source: & str, Config: & Config_Settings)->Result < String, String > 
    {
        if Config.Language.IsJavaScriptOrTypeScript() { return crate::js_ts::JS_TS_Formatter::Format(Source, Config); }

        if Config.Language.IsShell() { return crate::shell::Shell_Formatter::Format(Source, Config); }

        let mut Scanner = Lexer_Scanner::New(Source, Config.Language);
        let Tokens = Scanner.TokenizeAll();

        let mut Builder = Container_Tree_Builder::New(Tokens);
        let SyntaxNodes = Builder.BuildTree();

        let Emitter = Code_Emitter::New(Config);
        let Formatted = Emitter.Emit( & SyntaxNodes);
        // Safety gate verification
        if let Err(VerifyErr) = Safety_Verifier::VerifyTokenEquivalence(Source, & Formatted, Config.Language)
        {
            // Fail open: return original to prevent corrupting source code
            return Err(format ! ("{}. Output aborted to prevent code corruption.", VerifyErr));
        }

        Ok(Formatted)
    }

    pub fn IsFormatted(Source: & str, Config: & Config_Settings)-> bool
    {
        match Self::Format(Source, Config) { Ok(Formatted) => Formatted == Source, Err(_) => false }
    }
}
