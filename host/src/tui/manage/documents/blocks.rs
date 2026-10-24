//! Text-only editor snippets. No resource creation or execution is performed.
use super::Editor;

const BLOCKS: &[(&str, &str)] = &[
    ("doc", "## Document\n\n"),
    ("routine", "## Routine\n\n- Trigger: \n- Steps: \n"),
    ("skill", "## Skill\n\n### Purpose\n\n### Instructions\n\n"),
    ("task", "- [ ] Task\n"),
    ("heading", "## "),
    ("list", "- "),
    ("code", "```\n\n```\n"),
    ("quote", "> "),
];

pub(super) fn complete(editor: &mut Editor) -> bool {
    let cursor = editor.cursor;
    if !editor.text.is_char_boundary(cursor) {
        return false;
    }
    if editor.text[cursor..].chars().next().is_some_and(|ch| ch != '\n' && ch != '\r') {
        return false;
    }
    let start = editor.text[..cursor].rfind('\n').map_or(0, |at| at + 1);
    let Some(query) = editor.text[start..cursor].strip_prefix('/') else {
        return false;
    };
    if !query.bytes().all(|ch| ch.is_ascii_lowercase()) {
        return false;
    }
    let mut fence = None;
    for line in editor.text[..start].lines() {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() > 3 {
            continue;
        }
        let Some(marker) = trimmed.chars().next().filter(|ch| *ch == '`' || *ch == '~') else {
            continue;
        };
        let size = trimmed.chars().take_while(|ch| *ch == marker).count();
        if size < 3 {
            continue;
        }
        match fence {
            None => fence = Some((marker, size)),
            Some((previous, length)) if marker == previous && size >= length && trimmed[size..].trim().is_empty() => {
                fence = None;
            }
            _ => {}
        }
    }
    if fence.is_some() {
        return false;
    }
    let Some((_, markdown)) = BLOCKS.iter().find(|(name, _)| name.starts_with(query)) else {
        return false;
    };
    editor.text.replace_range(start..cursor, markdown);
    editor.cursor = start + markdown.len();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_preserves_markdown_and_ignores_examples() {
        for command in ["doc", "routine", "skill", "task"] {
            let mut editor = Editor::with(&format!("日本語\n/{command}"));
            assert!(complete(&mut editor));
            assert!(editor.text.starts_with("日本語\n"));
        }
        for text in ["```\n/doc", "~~~~\n/task", "````\n```\n/doc", "text/doc", "  /doc", "/unknown"] {
            let mut editor = Editor::with(text);
            assert!(!complete(&mut editor));
            assert_eq!(editor.text, text);
        }
    }
}
