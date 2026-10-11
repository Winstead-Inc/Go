#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)] use crate::formatter::Source_Formatter;
use crate::types:: { Config_Settings, Source_Language };

pub struct Polyglot_Formatter;

impl Polyglot_Formatter
{
    pub fn Format(Source: & str, Config: & Config_Settings)->Result<String, String>
    {
        let mut Formatted = String:: new ();
        let mut Remaining = Source;

        while ! Remaining.is_empty()
        {
            let mut FoundMatch = false;
            let mut EarliestIndex = Remaining.len();
            let mut MatchKind = Source_Language::Generic;
            let mut OpenKeyword = "";
            let mut CloseTag = "";
            let mut NeedsRightAngle = false;

            let Tags = 
            [
                ("<script", "</script>", Source_Language::JavaScript, true),("<style", "</style>", Source_Language::Css, true),
                (
                    "<?php", "?>", Source_Language::Generic, false
                ),
            ];

            for (Open, Close, Lang, NeedsAngle) in Tags.iter()
            {
                if let Some(Idx) = Remaining.find(Open)
                {
                    if Idx < EarliestIndex
                    {
                        EarliestIndex = Idx;
                        MatchKind = * Lang;
                        OpenKeyword = Open;
                        CloseTag = Close;
                        NeedsRightAngle = * NeedsAngle;
                        FoundMatch = true;
                    }
                }
            }

            if FoundMatch
            {
                Formatted.push_str( & Remaining[..EarliestIndex]);
                Remaining = & Remaining[EarliestIndex..];
                // Find the end of the opening tag
                let mut ContentStartIdx = OpenKeyword.len();
                if NeedsRightAngle
                {
                    if let Some(AngleIdx) = Remaining[ContentStartIdx..].find('>') { ContentStartIdx += AngleIdx + 1; }
                    else
                    {
                        // Malformed tag, just append and break
                        Formatted.push_str(Remaining);
                        break;
                    }
                }

                if let Some(EndIdx) = Remaining[ContentStartIdx..].find(CloseTag)
                {
                    let AbsoluteEndIdx = ContentStartIdx + EndIdx;
                    let BlockEnd = AbsoluteEndIdx + CloseTag.len();

                    let OpenTagFull = & Remaining[..ContentStartIdx];
                    let InnerContent = & Remaining[ContentStartIdx..AbsoluteEndIdx];

                    let mut SubConfig = Config.clone();
                    SubConfig.Language = MatchKind;

                    Formatted.push_str(OpenTagFull);

                    if InnerContent.trim().is_empty() { Formatted.push_str(InnerContent); }
                    else
                    {
                        match Source_Formatter::Format(InnerContent, & SubConfig)
                        {
                            Ok(mut InnerFormatted) => 
                            {
                                if ! InnerFormatted.starts_with('\n') && ! InnerFormatted.starts_with("\r\n") { Formatted.push('\n'); }
                                if ! InnerFormatted.ends_with('\n') { InnerFormatted.push('\n'); }
                                Formatted.push_str( & InnerFormatted);
                            }
                            Err(e) => { println!("Error in inner formatter: {}", e); Formatted.push_str(InnerContent); }
                        }
                    }

                    Formatted.push_str(CloseTag);
                    Remaining = & Remaining[BlockEnd..];
                }
                else
                {
                    // Unclosed tag
                    Formatted.push_str(Remaining);
                    break;
                }
            }
            else { Formatted.push_str(Remaining); break ; }
        }

        Ok(Formatted)
    }
}
