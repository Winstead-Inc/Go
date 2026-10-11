#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use lafs:: { Config_Settings, Source_Formatter, Source_Language };
#[test]
fn Test_JS_Monolith_Function()
{
    let Input = "function Add(A, B) { return A + B; }";
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 120;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert_eq!(Formatted.trim(),"function Add(A, B) { return A + B; }");
}
#[test]
fn Test_JS_Expanded_Function_Detached_Brace()
{
    let Input = r#"function ExecuteTask(A, B) {
    DoFirst(A);
    DoSecond(B);
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    let Expected = "function ExecuteTask(A, B)\n{\n    DoFirst(A);\n    DoSecond(B);\n}";
    assert_eq!(Formatted.trim(), Expected);
}
#[test]
fn Test_JS_Arrow_Function()
{
    let Input = r#"const Calculate = (A, B) => {
    StepOne(A);
    StepTwo(B);
};"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("const Calculate = (A, B) =>\n{\n"));
    assert!(Formatted.trim_end().ends_with("\n};"));
}
#[test]
fn Test_JS_ASI_Column_Anchored_Hybrid_On_Return()
{
    // OLAFS ASI Rule: after return, the opening brace MUST remain on the return line
    // to prevent JavaScript ASI from returning undefined
    let Input = r#"function GetData() {
    return {
        Status: 200,
        Payload: " OK"
    };
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("return {\n"),"Opening brace after return must remain on the return line (Column-Anchored Hybrid)");
    assert!(Formatted.contains("\n    };\n"),"Closing brace must align with the return statement baseline");
}
#[test]
fn Test_JS_Template_Literal_Preservation()
{
    let Input = r#"const Message = `Hello, ${User.Name}!
Welcome to the system.
Your balance is: ${Account.Balance}.`;"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert_eq!(Formatted.trim(), Input.trim());
}
#[test]
fn Test_JS_Regex_Literal_Vs_Division()
{
    let Input = r#"const IsMatch = /^[a-z0-9_-]+$/i.test(Str);
const Half = Total / 2;"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("/^[a-z0-9_-]+$/i.test(Str);"));
    assert!(Formatted.contains("Total / 2;"));
}
#[test]
fn Test_JS_Compound_Delimiter_Sole_Child_Chain()
{
    let Input = r#"const Config = [{(
    Module: " Security",
    Active: true
)}];"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("[{(\n"));
    assert!(Formatted.contains("\n)}];"));
}
#[test]
fn Test_TS_Interface_And_Enum()
{
    let Input = r#"interface User_Profile {
    Id: number;
    Name: string;
    Active: boolean;
}

enum Status_Code {
    Success,
    Failure
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::TypeScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("interface User_Profile\n{\n"));
    assert!(Formatted.contains("enum Status_Code\n{\n"));
}
#[test]
fn Test_JS_Control_Flow_If_Else()
{
    let Input = r#"if (Condition) {
    DoAlpha();
}
else {
    DoBeta();
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    let Expected = "if (Condition)\n{\n    DoAlpha();\n}\nelse\n{\n    DoBeta();\n}";
    assert_eq!(Formatted.trim(), Expected);
}
#[test]
fn Test_JS_Class_Declaration()
{
    let Input = r#"class Service_Manager {
    constructor(Config) {
        this.Config = Config;
    }
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;
    Config.Canvas_Width = 40;

    let Formatted = Source_Formatter::Format(Input, & Config).expect("Formatting failed");
    assert!(Formatted.contains("class Service_Manager\n{\n"));
    assert!(Formatted.contains("constructor(Config)\n    {\n"));
}
#[test]
fn Test_JS_Idempotence()
{
    let Input = r#"function CalculateTotal(Items)
{
    let Total = 0;
    for (const Item of Items)
    {
        Total += Item.Price;
    }
    return Total;
}"# ;
    let mut Config = Config_Settings:: default();
    Config.Language = Source_Language::JavaScript;

    let FirstPass = Source_Formatter::Format(Input, & Config).expect("First pass failed");
    let SecondPass = Source_Formatter::Format( & FirstPass, & Config).expect("Second pass failed");
    assert_eq!(FirstPass, SecondPass, "Formatting must be idempotent");
}
