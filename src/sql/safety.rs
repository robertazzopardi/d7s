use sqlparser::{
    ast::Statement,
    dialect::{Dialect, GenericDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlStatement {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementSafety {
    ReadOnly,
    RequiresConfirmation,
}

/// Split SQL text into statements while respecting common SQL quoting/comment rules.
#[must_use]
pub fn split_statements(sql: &str) -> Vec<SqlStatement> {
    parse_with_known_dialects(sql).map_or_else(
        || fallback_split(sql),
        |stmts| {
            stmts
                .into_iter()
                .map(|stmt| SqlStatement {
                    text: stmt.to_string(),
                })
                .collect()
        },
    )
}

#[must_use]
pub fn classify_statement(sql: &str) -> StatementSafety {
    if let Some(mut stmts) = parse_with_known_dialects(sql)
        && let Some(stmt) = stmts.pop()
    {
        return if is_read_only_statement(&stmt) {
            StatementSafety::ReadOnly
        } else {
            StatementSafety::RequiresConfirmation
        };
    }

    StatementSafety::RequiresConfirmation
}

fn parse_with_known_dialects(sql: &str) -> Option<Vec<Statement>> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Some(Vec::new());
    }

    parse_with_dialect(&PostgreSqlDialect {}, sql)
        .or_else(|| parse_with_dialect(&SQLiteDialect {}, sql))
        .or_else(|| parse_with_dialect(&GenericDialect {}, sql))
}

fn parse_with_dialect(
    dialect: &dyn Dialect,
    sql: &str,
) -> Option<Vec<Statement>> {
    Parser::parse_sql(dialect, sql).ok()
}

const fn is_read_only_statement(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::Query(_)
            | Statement::Explain { .. }
            | Statement::ShowVariable { .. }
    )
}

fn fallback_split(sql: &str) -> Vec<SqlStatement> {
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| SqlStatement {
            text: s.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{StatementSafety, classify_statement, split_statements};

    #[test]
    fn splits_multiple_statements() {
        let statements = split_statements("SELECT 1; SELECT 2;");
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn classifies_select_as_read_only() {
        let safety = classify_statement("SELECT * FROM users");
        assert_eq!(safety, StatementSafety::ReadOnly);
    }

    #[test]
    fn classifies_delete_as_mutating() {
        let safety = classify_statement("DELETE FROM users");
        assert_eq!(safety, StatementSafety::RequiresConfirmation);
    }
}
