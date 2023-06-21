(call_expression
     function: (field_expression
       value: (call_expression
         function: (field_expression
           field: (field_identifier) @method_name (#match? @method_name "create_variable_statement_raw|create_variable_declaration_list_raw")
         )
       )
       field: (field_identifier) @wrap (#eq? @wrap "wrap")
     )
)
