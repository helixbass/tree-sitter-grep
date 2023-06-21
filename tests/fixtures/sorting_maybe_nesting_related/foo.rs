impl TransformTypeScript {
    pub(super) fn create_class_declaration_head_with_decorators(
        &self,
        node: &Node, /* ClassDeclaration */
        name: Option<impl Borrow<Node /* Identifier */>>,
    ) -> io::Result<Gc<Node>> {
        let node_as_class_declaration = node.as_class_declaration();
        let location = move_range_past_decorators(node);
        let class_alias = self.get_class_alias_if_needed(node);

        let decl_name = if self.language_version <= ScriptTarget::ES2015 {
            self.factory
                .get_internal_name(node, Some(false), Some(true))
        } else {
            self.factory.get_local_name(node, Some(false), Some(true))
        };

        let heritage_clauses = try_maybe_visit_nodes(
            node_as_class_declaration
                .maybe_heritage_clauses()
                .as_deref(),
            Some(|node: &Node| self.visitor(node)),
            Some(is_heritage_clause),
            None,
            None,
        )?;
        let members = self.transform_class_members(node)?;
        let class_expression = self
            .factory
            .create_class_expression_raw(
                Option::<Gc<NodeArray>>::None,
                Option::<Gc<NodeArray>>::None,
                name.node_wrappered(),
                Option::<Gc<NodeArray>>::None,
                heritage_clauses,
                members,
            )
            .wrap()
            .set_original_node(Some(node.node_wrapper()))
            .set_text_range(Some(&location.to_readonly_text_range()));

        Ok(self
            .factory
            .create_variable_statement_raw(
                Option::<Gc<NodeArray>>::None,
                self.factory
                    .create_variable_declaration_list_raw(
                        vec![self
                            .factory
                            .create_variable_declaration(
                                Some(decl_name),
                                None,
                                None,
                                Some(if let Some(class_alias) = class_alias {
                                    self.factory
                                        .create_assignment(class_alias, class_expression)
                                        .wrap()
                                } else {
                                    class_expression
                                }),
                            )
                            .wrap()],
                        Some(NodeFlags::Let),
                    )
                    .wrap(),
            )
            .wrap()
            .set_original_node(Some(node.node_wrapper()))
            .set_text_range(Some(&location.to_readonly_text_range()))
            .set_comment_range(node))
    }
}
