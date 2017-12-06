use toolshed::list::{ListBuilder, GrowableList};

use ast::*;
use parser::Parser;
use lexer::Token;

impl<'ast> Parser<'ast> {
    pub fn contract_definition(&mut self) -> Option<SourceUnitNode<'ast>> {
        let start = self.lexer.start_then_consume();
        let name = self.expect_str_node(Token::Identifier);

        let inherits = if self.allow(Token::KeywordIs) {
            let builder = ListBuilder::new(self.arena, self.expect_str_node(Token::Identifier));

            while self.allow(Token::Comma) {
                builder.push(self.arena, self.expect_str_node(Token::Identifier));
            }

            builder.as_list()
        } else {
            NodeList::empty()
        };

        self.expect(Token::BraceOpen);

        let builder = GrowableList::new();

        while let Some(part) = self.contract_part() {
            builder.push(self.arena, part);
        }

        let end = self.expect_end(Token::BraceClose);

        Some(self.node_at(start, end, ContractDefinition {
            name,
            inherits,
            body: builder.as_list(),
        }))
    }

    fn contract_part(&mut self) -> Option<ContractPartNode<'ast>> {
        match self.lexer.token {
            Token::DeclarationEvent => return self.event_definition(),
            _ => {},
        }

        let type_name  = self.type_name()?;
        let visibility = self.visibility();
        let name       = self.expect_str_node(Token::Identifier);
        let end        = self.expect_end(Token::Semicolon);

        Some(self.node_at(type_name.start, end, StateVariableDeclaration {
            type_name,
            visibility,
            name,
            init: None,
        }))
    }

    fn visibility(&mut self) -> Visibility {
        match self.lexer.token {
            Token::KeywordPublic   => {
                self.lexer.consume();

                Visibility::Public
            },
            Token::KeywordInternal => {
                self.lexer.consume();

                Visibility::Internal
            },
            Token::KeywordPrivate  => {
                self.lexer.consume();

                Visibility::Private
            },
            Token::KeywordConstant => {
                self.lexer.consume();

                Visibility::Constant
            },
            _ => Visibility::Unspecified,
        }
    }

    fn event_definition(&mut self) -> Option<ContractPartNode<'ast>> {
        let start  = self.lexer.start_then_consume();
        let name   = self.expect_str_node(Token::Identifier);

        self.expect(Token::ParenOpen);

        let params = match self.indexed_parameter() {
            Some(param) => {
                let builder = ListBuilder::new(self.arena, param);

                while self.allow(Token::Comma) {
                    match self.indexed_parameter() {
                        Some(param) => builder.push(self.arena, param),
                        None        => self.error(),
                    }
                }

                builder.as_list()
            },
            None => NodeList::empty(),
        };

        self.expect(Token::ParenClose);

        let anonymous = self.allow(Token::KeywordAnonymous);
        let end       = self.expect_end(Token::Semicolon);

        Some(self.node_at(start, end, EventDefinition {
            anonymous,
            name,
            params,
        }))
    }

    fn indexed_parameter(&mut self) -> Option<Node<'ast, IndexedParameter<'ast>>> {
        let type_name = self.type_name()?;
        let indexed   = self.allow(Token::KeywordIndexed);
        let name      = self.expect_str_node(Token::Identifier);

        Some(self.node_at(type_name.start, name.end, IndexedParameter {
            indexed,
            type_name,
            name,
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use parser::mock::{Mock, assert_units};

    #[test]
    fn empty_contract() {
        let m = Mock::new();

        assert_units(r#"

            contract Foo {}
            contract Doge is Amazing {}
            contract This is Silly, Kinda {}

        "#, [
            m.node(14, 29, ContractDefinition {
                name: m.node(23, 26, "Foo"),
                inherits: NodeList::empty(),
                body: NodeList::empty(),
            }),
            m.node(42, 69, ContractDefinition {
                name: m.node(51, 55, "Doge"),
                inherits: m.list([
                    m.node(59, 66, "Amazing"),
                ]),
                body: NodeList::empty(),
            }),
            m.node(82, 114, ContractDefinition {
                name: m.node(91, 95, "This"),
                inherits: m.list([
                    m.node(99, 104, "Silly"),
                    m.node(106, 111, "Kinda"),
                ]),
                body: NodeList::empty(),
            }),
        ]);
    }

    #[test]
    fn empty_events() {
        let m = Mock::new();

        assert_units(r#"

            contract Foo {
                event Horizon();
                event Alcoholics() anonymous;
            }

        "#, [
            m.node(14, 121, ContractDefinition {
                name: m.node(23, 26, "Foo"),
                inherits: NodeList::empty(),
                body: m.list([
                    m.node(45, 61, EventDefinition {
                        anonymous: false,
                        name: m.node(51, 58, "Horizon"),
                        params: NodeList::empty(),
                    }),
                    m.node(78, 107, EventDefinition {
                        anonymous: true,
                        name: m.node(84, 94, "Alcoholics"),
                        params: NodeList::empty(),
                    }),
                ]),
            }),
        ]);
    }

    #[test]
    fn event_with_parameters() {
        let m = Mock::new();

        assert_units(r#"

            contract Foo {
                event Horizon(int32 indexed foo, bool bar);
            }

        "#, [
            m.node(14, 102, ContractDefinition {
                name: m.node(23, 26, "Foo"),
                inherits: NodeList::empty(),
                body: m.list([
                    m.node(45, 88, EventDefinition {
                        anonymous: false,
                        name: m.node(51, 58, "Horizon"),
                        params: m.list([
                            m.node(59, 76, IndexedParameter {
                                indexed: true,
                                type_name: m.node(59, 64, ElementaryTypeName::Int(4)),
                                name: m.node(73, 76, "foo"),
                            }),
                            m.node(78, 86, IndexedParameter {
                                indexed: false,
                                type_name: m.node(78, 82, ElementaryTypeName::Bool),
                                name: m.node(83, 86, "bar"),
                            }),
                        ]),
                    }),
                ]),
            }),
        ]);
    }

    #[test]
    fn state_variable_declaration() {
        let m = Mock::new();

        assert_units(r#"

            contract Foo {
                int32 foo;
                bytes10 public doge;
            }

        "#, [
            m.node(14, 106, ContractDefinition {
                name: m.node(23, 26, "Foo"),
                inherits: NodeList::empty(),
                body: m.list([
                    m.node(45, 55, StateVariableDeclaration {
                        type_name: m.node(45, 50, ElementaryTypeName::Int(4)),
                        visibility: Visibility::Unspecified,
                        name: m.node(51, 54, "foo"),
                        init: None,
                    }),
                    m.node(72, 92, StateVariableDeclaration {
                        type_name: m.node(72, 79, ElementaryTypeName::Byte(10)),
                        visibility: Visibility::Public,
                        name: m.node(87, 91, "doge"),
                        init: None,
                    }),
                ]),
            }),
        ]);
    }
}