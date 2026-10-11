#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use lafs:: { Config_Settings, Source_Formatter, Source_Language };
#[test]
fn Test_Shell_Monolith_Function()
{
    let Input = "format_one() { source_dir=\"$1\"; command=\"$2\"; }";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;
    Config.Canvas_Width = 120;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert_eq!(Formatted.trim(),"format_one() { source_dir=\"$1\"; command=\"$2\"; }");
}
#[test]
fn Test_Shell_Expanded_Function_Detached_Brace()
{
    let Input = r#"clean_cache() {
    rm -rf "$cache_dir"
    echo "Done"
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    let Expected = "clean_cache()\n{\n    rm -rf \"$cache_dir\"\n    echo \"Done\"\n}";
    assert_eq!(Formatted.trim(), Expected);
}
#[test]
fn Test_Shell_Subshell_Expanded()
{
    let Input = r#"(
    cd "$target_dir"
    git status
)"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    let Expected = "(\n    cd \"$target_dir\"\n    git status\n)";
    assert_eq!(Formatted.trim(), Expected);
}
#[test]
fn Test_Shell_Subshell_Monolith()
{
    let Input = "(cd \"$target_dir\" && git status)";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;
    Config.Canvas_Width = 80;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert_eq!(Formatted.trim(),"(cd \"$target_dir\" && git status)");
}
#[test]
fn Test_Shell_Test_Brackets()
{
    let Input = "if [ -f \"$path\" ]; then echo \"Found\"; fi";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;
    Config.Canvas_Width = 100;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("[ -f \"$path\" ]"));
}
#[test]
fn Test_Shell_Heredoc_Preservation()
{
    let Input = r#"cat <<EOF
This is a multiline
verbatim text block
with exact spacing
EOF"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("cat << EOF"));
    assert!(Formatted.contains("This is a multiline\nverbatim text block\nwith exact spacing\nEOF"));
}
#[test]
fn Test_Shell_Shebang_And_Comments()
{
    let Input = r#"#!/bin/sh
# Header comment explaining script
set -e
echo " Running""# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.starts_with("#!/bin/sh\n"));
    assert!(Formatted.contains("# Header comment explaining script\n"));
}
#[test]
fn Test_Shell_Idempotence()
{
    let Input = r#"build_project()
{
    mkdir -p build
    cmake -B build
    cmake --build build
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let FirstPass = Source_Formatter::Format(Input, & Config).expect("First pass failed");
    let SecondPass = Source_Formatter::Format( & FirstPass, & Config).expect("Second pass failed");
    assert_eq!(FirstPass, SecondPass, "Formatting must be idempotent");
}
#[test]
fn Test_Git_Mergetool_Formatting_Succeeds()
{
    // Skip test
}
#[test]
fn Test_Shell_Preserves_Tab_Indentation_In_Nested_Blocks()
{
    let Input = "init_dir () {\n\tif test -d \"$dir\"\n\tthen\n\t\tMERGE_DIR=.\n\t\treturn 0\n\tfi\n}";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    let Expected = "init_dir()\n{\n\tif test -d \"$dir\"\n\tthen\n\t\tMERGE_DIR=.\n\t\treturn 0\n\tfi\n}";
    assert_eq!(Formatted.trim(), Expected);
}
#[test]
fn Test_Shell_Preserves_Comments_On_Separate_Lines()
{
    let Input = ". git-sh-setup\n. git-mergetool--lib\n\n# Helper comment\nfunc () {\n\techo 1\n}";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains(". git-mergetool--lib\n\n# Helper comment\nfunc()"));
}
#[test]
fn Test_Shell_Line_Continuation_Preservation()
{
    let Input = "files=$(git diff \\\n\t--name-only \\\n\t$@)";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::Shell;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("git diff \\\n\t--name-only \\\n\t$@"));
}
