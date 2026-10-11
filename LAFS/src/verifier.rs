#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)] use crate::lexer::Lexer_Scanner;
use crate::types:: { Source_Language, Token_Kind };

pub struct Safety_Verifier;

impl Safety_Verifier
{
    pub fn VerifyTokenEquivalence(OriginalSource: & str, FormattedSource: & str, Language: Source_Language,)->Result<(), String>
    {
        let mut OriginalScanner = Lexer_Scanner::New(OriginalSource, Language);
        let OriginalTokens = OriginalScanner.TokenizeAll();

        let mut FormattedScanner = Lexer_Scanner::New(FormattedSource, Language);
        let FormattedTokens = FormattedScanner.TokenizeAll();

        let OrigSignificant: Vec <& str > = OriginalTokens.iter().filter( | T | ! matches ! (T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline)).map( | T | T.Text.as_str()).collect();

        let FormSignificant: Vec <& str > = FormattedTokens.iter().filter (| T | ! matches!(T.Kind, Token_Kind::Whitespace(_) | Token_Kind::Newline))
        .map( | T | T.Text.as_str()).collect();

        if OrigSignificant.len() != FormSignificant.len()
        {
            std::fs::write("b:/Go/debug.rs", FormattedSource).unwrap();
            let diff_len = std::cmp::min(OrigSignificant.len(), FormSignificant.len());
            for i in 0..diff_len
            {
                if OrigSignificant[i] != FormSignificant[i]
                {
                    println!("Mismatch at index {}:\nOriginal: {:?}\nFormatted: {:?}", i, OrigSignificant[i], FormSignificant[i]);
                    break;
                }
            }
            return Err
            (
                format!
                (
                    "Safety Gate Failed: Significant token count mismatch! (Original: {}, Formatted: {})", OrigSignificant.len(), FormSignificant.len()
                )
            );
        }

        for (Index, (Orig, Form)) in OrigSignificant.iter().zip(FormSignificant.iter()).enumerate()
        {
            if Orig != Form { return Err(format ! ("Safety Gate Failed: Token #{} mismatch! (Expected '{}', Got '{}')", Index, Orig, Form)); }
        }

        Ok(())
    }

    pub fn VerifyIdempotence<F>(FormattedSource: & str, FormatFn: F)->Result<(), String> where F: Fn( & str)->String, 
    {
        let SecondPass = FormatFn(FormattedSource);
        if SecondPass != FormattedSource { return Err("Safety Gate Warning: Formatting is not idempotent on second pass!".to_string(),); }
        Ok(())
    }
}
