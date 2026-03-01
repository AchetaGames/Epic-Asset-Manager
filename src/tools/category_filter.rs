/// Evaluate a category filter expression against a predicate.
///
/// The expression language supports:
/// - Simple terms: `"assets"` — evaluated via the predicate
/// - Negation: `"!games"` — inverts the predicate result
/// - AND: `"assets&!games"` — both sides must be true
/// - OR: `"assets|games"` — either side must be true
///
/// Operators are evaluated left-to-right (no precedence). The predicate
/// closure receives individual category names and returns whether the
/// item belongs to that category.
pub fn eval_category_expr(expr: &str, checker: &impl Fn(&str) -> bool) -> bool {
    expr.split(&['|', '&']).next().is_some_and(|c| {
        let result = if let Some(stripped) = c.strip_prefix('!') {
            !checker(stripped)
        } else {
            checker(c)
        };

        expr.chars().nth(c.len()).map_or(result, |operator| {
            let remainder: String = expr.chars().skip(c.len() + 1).collect();
            match operator {
                '&' => result && eval_category_expr(&remainder, checker),
                '|' => result || eval_category_expr(&remainder, checker),
                _ => false,
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: checker that matches a fixed set of categories.
    fn has<'a>(cats: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |c| cats.iter().any(|cat| cat.eq_ignore_ascii_case(c))
    }

    // --- simple terms ---

    #[test]
    fn single_term_match() {
        assert!(eval_category_expr("assets", &has(&["assets"])));
    }

    #[test]
    fn single_term_no_match() {
        assert!(!eval_category_expr("games", &has(&["assets"])));
    }

    #[test]
    fn negation_match() {
        assert!(eval_category_expr("!games", &has(&["assets"])));
    }

    #[test]
    fn negation_no_match() {
        assert!(!eval_category_expr("!assets", &has(&["assets"])));
    }

    // --- AND operator ---

    #[test]
    fn and_both_true() {
        assert!(eval_category_expr(
            "assets&plugins",
            &has(&["assets", "plugins"])
        ));
    }

    #[test]
    fn and_left_false() {
        assert!(!eval_category_expr("games&plugins", &has(&["plugins"])));
    }

    #[test]
    fn and_right_false() {
        assert!(!eval_category_expr("assets&games", &has(&["assets"])));
    }

    #[test]
    fn and_with_negation() {
        assert!(eval_category_expr("assets&!games", &has(&["assets"])));
    }

    #[test]
    fn and_with_negation_fails() {
        assert!(!eval_category_expr(
            "assets&!games",
            &has(&["assets", "games"])
        ));
    }

    // --- OR operator ---

    #[test]
    fn or_first_true() {
        assert!(eval_category_expr("assets|games", &has(&["assets"])));
    }

    #[test]
    fn or_second_true() {
        assert!(eval_category_expr("assets|games", &has(&["games"])));
    }

    #[test]
    fn or_neither_true() {
        assert!(!eval_category_expr("assets|games", &has(&["plugins"])));
    }

    #[test]
    fn or_both_true() {
        assert!(eval_category_expr(
            "assets|games",
            &has(&["assets", "games"])
        ));
    }

    // --- chained operators ---

    #[test]
    fn three_way_and() {
        assert!(eval_category_expr("a&b&c", &has(&["a", "b", "c"])));
    }

    #[test]
    fn three_way_and_one_missing() {
        assert!(!eval_category_expr("a&b&c", &has(&["a", "b"])));
    }

    #[test]
    fn three_way_or() {
        assert!(eval_category_expr("a|b|c", &has(&["c"])));
    }

    #[test]
    fn mixed_and_or_left_to_right() {
        // "a&b|c" with a=true, b=false, c=true
        // left-to-right: (a&b) = false, then false|c = true
        assert!(eval_category_expr("a&b|c", &has(&["a", "c"])));
    }

    #[test]
    fn mixed_and_or_short_circuit() {
        // "a|b&c" with a=true, b=false, c=false
        // left-to-right: a = true, then true|... short-circuits to true
        assert!(eval_category_expr("a|b&c", &has(&["a"])));
    }

    // --- edge cases ---

    #[test]
    fn empty_expression() {
        assert!(!eval_category_expr("", &has(&["anything"])));
    }

    #[test]
    fn just_negation_bang() {
        // "!" → negation of empty string → checker("") → false → !false = true
        assert!(eval_category_expr("!", &has(&["anything"])));
    }

    #[test]
    fn double_negation_not_supported() {
        // "!!a" → first term is "!!a" (split sees no operator)
        // starts with '!' → checker("!a") → false (no category named "!a") → !false = true
        assert!(eval_category_expr("!!a", &has(&["a"])));
    }

    #[test]
    fn trailing_operator() {
        // "assets&" → first term "assets" true, operator '&', remainder ""
        // eval_category_expr("") → false
        assert!(!eval_category_expr("assets&", &has(&["assets"])));
    }

    #[test]
    fn leading_operator() {
        // "&assets" → first term "" (from split) → checker("") → false
        assert!(!eval_category_expr("&assets", &has(&["assets"])));
    }

    #[test]
    fn no_categories_match_nothing() {
        assert!(!eval_category_expr("assets", &has(&[])));
    }
}
