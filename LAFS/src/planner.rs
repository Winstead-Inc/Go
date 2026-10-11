#![allow(
non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)] use crate::container_tree::Container_Node;
use crate::types:: { Config_Settings, Layout_State };

pub struct Layout_Planner<'a> { Config: & 'a Config_Settings, }

impl <'a> Layout_Planner<'a>
{
    pub fn New(Config: & 'a Config_Settings)->Self { Layout_Planner { Config } }

    pub fn DecideLayout( & self, Container: & Container_Node, CurrentIndentLevel: usize, CurrentColumn: usize,)->Layout_State
    {
        // 1. Monolith First (Horizontal Density)
        // Check complexity budget: statements <= max_statements, block depth <= max_block_depth, total containers <= max_containers
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
                
                // If it's a data container (no statements) and fits within canvas on its own, don't expand it just because it's pushed right.
                if Container.Statement_Count == 0 && SingleSpan <= self.Config.Canvas_Width { return Layout_State::Monolith; }
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
