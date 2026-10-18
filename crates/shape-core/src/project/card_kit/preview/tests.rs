use super::*;

#[test]
fn real_chinese_rule_and_closing_combinations_keep_a_character_with_the_punctuation() {
    let rule = "获得 1 点专注，然后抽 1 张牌。";
    let wrapped = wrap(rule, 30);
    assert!(!wrapped.overflow);
    assert!(wrapped.lines.len() > 1);
    assert!(wrapped.lines.last().unwrap().ends_with("牌。"));
    assert!(
        wrapped
            .lines
            .iter()
            .all(|line| !line.starts_with(closing_punctuation))
    );
    assert_eq!(
        wrapped.lines.concat().replace(' ', ""),
        rule.replace(' ', "")
    );
    for tail in ["，", "。", "？！", "。”】", "……）》"] {
        let text = format!("{}{tail}", "文".repeat(17));
        let wrapped = wrap(&text, 30);
        assert!(!wrapped.overflow);
        assert_eq!(wrapped.lines.concat(), text);
        assert!(
            wrapped
                .lines
                .iter()
                .all(|line| !line.starts_with(closing_punctuation))
        );
        assert!(
            wrapped
                .lines
                .iter()
                .all(|line| line.chars().map(glyph_units).sum::<usize>() <= 638 * 4 / 30)
        );
    }
}

#[test]
fn authored_newline_is_preserved_and_an_unbreakable_punctuation_run_reports_overflow() {
    let authored = wrap("保留手动换行\n。下一段", 30);
    assert!(!authored.overflow);
    assert_eq!(authored.lines, ["保留手动换行", "。下一段"]);
    let source = format!("文{}", "。".repeat(40));
    let wrapped = wrap(&source, 30);
    assert!(wrapped.overflow);
    assert_eq!(wrapped.lines.concat(), source);
    assert!(
        wrapped
            .lines
            .iter()
            .all(|line| line.chars().map(glyph_units).sum::<usize>() <= 638 * 4 / 30)
    );
}
