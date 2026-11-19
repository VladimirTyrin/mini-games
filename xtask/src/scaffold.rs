use std::fs;
use std::path::Path;

use crate::game_names::GameNames;

struct Template {
    path: String,
    body: &'static str,
}

fn templates(names: &GameNames) -> Vec<Template> {
    let game = format!("crates/games/{}", names.kebab);
    let web = format!("client/src/games/{}", names.kebab);
    let template = |path: String, body| Template { path, body };
    vec![
        template(
            format!("{game}/Cargo.toml"),
            include_str!("../templates/game/Cargo.toml.tmpl"),
        ),
        template(
            format!("{game}/src/lib.rs"),
            include_str!("../templates/game/src/lib.rs.tmpl"),
        ),
        template(
            format!("{game}/src/game.rs"),
            include_str!("../templates/game/src/game.rs.tmpl"),
        ),
        template(
            format!("{game}/src/settings/mod.rs"),
            include_str!("../templates/game/src/settings/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/settings/raw_settings.rs"),
            include_str!("../templates/game/src/settings/raw_settings.rs.tmpl"),
        ),
        template(
            format!("{game}/src/settings/settings.rs"),
            include_str!("../templates/game/src/settings/settings.rs.tmpl"),
        ),
        template(
            format!("{game}/src/model/mod.rs"),
            include_str!("../templates/game/src/model/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/model/seats.rs"),
            include_str!("../templates/game/src/model/seats.rs.tmpl"),
        ),
        template(
            format!("{game}/src/rules/mod.rs"),
            include_str!("../templates/game/src/rules/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/rules/addition.rs"),
            include_str!("../templates/game/src/rules/addition.rs.tmpl"),
        ),
        template(
            format!("{game}/src/rules/refusal.rs"),
            include_str!("../templates/game/src/rules/refusal.rs.tmpl"),
        ),
        template(
            format!("{game}/src/io/mod.rs"),
            include_str!("../templates/game/src/io/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/io/command.rs"),
            include_str!("../templates/game/src/io/command.rs.tmpl"),
        ),
        template(
            format!("{game}/src/io/world_event.rs"),
            include_str!("../templates/game/src/io/world_event.rs.tmpl"),
        ),
        template(
            format!("{game}/src/view/mod.rs"),
            include_str!("../templates/game/src/view/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/view/view.rs"),
            include_str!("../templates/game/src/view/view.rs.tmpl"),
        ),
        template(
            format!("{game}/src/tests/mod.rs"),
            include_str!("../templates/game/src/tests/mod.rs.tmpl"),
        ),
        template(
            format!("{game}/src/tests/conformance.rs"),
            include_str!("../templates/game/src/tests/conformance.rs.tmpl"),
        ),
        template(
            format!("{game}/src/tests/rules.rs"),
            include_str!("../templates/game/src/tests/rules.rs.tmpl"),
        ),
        template(
            format!("proto/minigames/v2/{}.proto", names.snake),
            include_str!("../templates/proto.tmpl"),
        ),
        template(
            format!("crates/protocol/src/games/{}.rs", names.snake),
            include_str!("../templates/protocol.rs.tmpl"),
        ),
        template(
            format!("{web}/index.ts"),
            include_str!("../templates/web/index.ts.tmpl"),
        ),
        template(
            format!("{web}/{}Board.vue", names.pascal),
            include_str!("../templates/web/Board.vue.tmpl"),
        ),
        template(
            format!("{web}/{}Settings.vue", names.pascal),
            include_str!("../templates/web/Settings.vue.tmpl"),
        ),
        template(
            format!("{web}/{}Help.en.vue", names.pascal),
            include_str!("../templates/web/Help.en.vue.tmpl"),
        ),
        template(
            format!("{web}/{}Help.ru.vue", names.pascal),
            include_str!("../templates/web/Help.ru.vue.tmpl"),
        ),
        template(
            format!("client/src/i18n/en/games/{}.ts", names.kebab),
            include_str!("../templates/i18n/en.ts.tmpl"),
        ),
        template(
            format!("client/src/i18n/ru/games/{}.ts", names.kebab),
            include_str!("../templates/i18n/ru.ts.tmpl"),
        ),
    ]
}

pub(crate) fn create_game(root: &Path, names: &GameNames) -> Result<(), String> {
    let crate_dir = root.join("crates/games").join(&names.kebab);
    if crate_dir.exists() {
        return Err(format!("{} already exists", crate_dir.display()));
    }
    // Check every marker before writing anything, so a failure leaves the tree untouched.
    let patches = patches(root, names)?;
    for template in templates(names) {
        let path = root.join(&template.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, names.render(template.body))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    for patch in patches {
        fs::write(&patch.path, patch.contents).map_err(|e| format!("{}: {e}", patch.path))?;
    }
    Ok(())
}

struct Patch {
    path: String,
    contents: String,
}

/// Inserts `line` (plus newline) right before the line containing `marker`, keeping its indent.
fn insert_before_marker(
    contents: &str,
    marker: &str,
    line: &str,
    file: &str,
) -> Result<String, String> {
    let position = contents
        .find(marker)
        .ok_or_else(|| format!("{file}: marker `{marker}` not found"))?;
    let line_start = contents[..position]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let indent = &contents[line_start..position];
    let mut patched = String::with_capacity(contents.len() + line.len() + indent.len() + 1);
    patched.push_str(&contents[..line_start]);
    patched.push_str(indent);
    patched.push_str(line);
    patched.push('\n');
    patched.push_str(&contents[line_start..]);
    Ok(patched)
}

/// Next free field number in the oneof that holds `marker`.
fn next_field_number(contents: &str, marker: &str) -> Result<u32, String> {
    let marker_at = contents
        .find(marker)
        .ok_or_else(|| format!("marker `{marker}` not found"))?;
    let block_start = contents[..marker_at]
        .rfind("oneof")
        .ok_or("oneof not found")?;
    let highest = contents[block_start..marker_at]
        .lines()
        .filter_map(|line| line.split('=').nth(1))
        .filter_map(|number| {
            number
                .trim()
                .trim_end_matches(';')
                .trim()
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    Ok(highest + 1)
}

fn read(root: &Path, file: &str) -> Result<String, String> {
    fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))
}

fn patches(root: &Path, names: &GameNames) -> Result<Vec<Patch>, String> {
    let mut patches = Vec::new();
    let mut patch =
        |file: &str, edit: &dyn Fn(String) -> Result<String, String>| -> Result<(), String> {
            let contents = edit(read(root, file)?)?;
            patches.push(Patch {
                path: root.join(file).display().to_string(),
                contents,
            });
            Ok(())
        };
    let crate_name = format!("mg-{}", names.kebab);

    patch("Cargo.toml", &|c| {
        let line = format!(
            "{crate_name} = {{ path = \"crates/games/{}\" }}",
            names.kebab
        );
        insert_before_marker(&c, "# xtask:game-crates", &line, "Cargo.toml")
    })?;
    for file in ["crates/protocol/Cargo.toml", "crates/registry/Cargo.toml"] {
        patch(file, &|c| {
            insert_before_marker(
                &c,
                "# xtask:game-deps",
                &format!("{crate_name}.workspace = true"),
                file,
            )
        })?;
    }
    patch("crates/protocol/src/games/mod.rs", &|c| {
        Ok(format!("{c}mod {};\n", names.snake))
    })?;
    patch("crates/registry/src/games.rs", &|c| {
        let entry = format!(
            "{0} => mg_{1}::{0}, {0}, no_bots;",
            names.pascal, names.snake
        );
        insert_before_marker(&c, "// xtask:games", &entry, "games.rs")
    })?;
    patch("proto/minigames/v2/game.proto", &|c| {
        let mut c = insert_before_marker(
            &c,
            "// xtask:imports",
            &format!("import \"minigames/v2/{}.proto\";", names.snake),
            "game.proto",
        )?;
        for (oneof, suffix) in [
            ("GameSettings", "Settings"),
            ("GameCommand", "Command"),
            ("GameWorldEvent", "WorldEvent"),
            ("GameView", "View"),
        ] {
            let marker = format!("// xtask:{oneof}");
            let number = next_field_number(&c, &marker)?;
            let field = format!("{}{suffix} {} = {number};", names.pascal, names.snake);
            c = insert_before_marker(&c, &marker, &field, "game.proto")?;
        }
        Ok(c)
    })?;
    patch("proto/minigames/v2/refusal.proto", &|c| {
        let c = insert_before_marker(
            &c,
            "// xtask:imports",
            &format!("import \"minigames/v2/{}.proto\";", names.snake),
            "refusal.proto",
        )?;
        let number = next_field_number(&c, "// xtask:Refusal")?;
        let field = format!("{}Refusal {} = {number};", names.pascal, names.snake);
        insert_before_marker(&c, "// xtask:Refusal", &field, "refusal.proto")
    })?;
    patch("crates/protocol/src/refusal_english.rs", &|c| {
        let arm = format!(
            "Some(Reason::{0}(value)) => game_english::<mg_{1}::Refusal>(value),",
            names.pascal, names.snake
        );
        insert_before_marker(&c, "// xtask:refusal_english", &arm, "refusal_english.rs")
    })?;
    patch("crates/protocol/src/refusal_english_tests.rs", &|c| {
        let check = format!(
            "check_game::<mg_{1}::Refusal>(|value| proto::{0}Refusal::try_from(value).is_ok(), Reason::{0});",
            names.pascal, names.snake
        );
        insert_before_marker(
            &c,
            "// xtask:check_game",
            &check,
            "refusal_english_tests.rs",
        )
    })?;
    // JSON has no comments: the invitation goes before the first text after the invitations.
    patch("client/push-texts.json", &|c| {
        let entry = format!(
            "\"push_invite_body_{0}\": {{\n    \"en\": \"{1}: tap to join.\",\n    \
             \"ru\": \"{1}: нажми, чтобы присоединиться.\"\n  }},",
            names.snake, names.title
        );
        insert_before_marker(
            &c,
            "\"push_invite_accepted_elsewhere\"",
            &entry,
            "push-texts.json",
        )
    })?;
    for language in ["en", "ru"] {
        let file = format!("client/src/i18n/{language}/index.ts");
        patch(&file, &|c| {
            let c = insert_before_marker(
                &c,
                "// xtask:imports",
                &format!(
                    "import {{ {} }} from \"./games/{}\";",
                    names.camel, names.kebab
                ),
                &file,
            )?;
            insert_before_marker(&c, "// xtask:games", &format!("{},", names.camel), &file)
        })?;
        let file = format!("client/src/i18n/{language}/refusals.ts");
        let not_allowed = if language == "en" {
            "That is not allowed"
        } else {
            "Так нельзя"
        };
        let amount = if language == "en" {
            "Add 1, 2 or 3"
        } else {
            "Прибавь 1, 2 или 3"
        };
        patch(&file, &|c| {
            let c = insert_before_marker(
                &c,
                "// xtask:imports",
                &format!(
                    "import {{ {}Refusal }} from \"@/proto/minigames/v2/{}_pb\";",
                    names.pascal, names.snake
                ),
                &file,
            )?;
            let words = format!(
                "{camel}: {{\n    [{pascal}Refusal.UNSPECIFIED]: \"{not_allowed}\",\n    \
                 [{pascal}Refusal.AMOUNT_OUT_OF_RANGE]: \"{amount}\",\n  }}{record},",
                camel = names.camel,
                pascal = names.pascal,
                record = if language == "en" {
                    format!(" satisfies Record<{}Refusal, string>", names.pascal)
                } else {
                    String::new()
                },
            );
            insert_before_marker(&c, "// xtask:refusals", &words, &file)
        })?;
    }
    patch("client/src/util/refusal-text.ts", &|c| {
        let case = format!(
            "case \"{0}\":\n        return texts.{0}[reason.value];",
            names.camel
        );
        insert_before_marker(&c, "// xtask:refusals", &case, "refusal-text.ts")
    })?;
    patch("client/src/games/index.ts", &|c| {
        let c = insert_before_marker(
            &c,
            "// xtask:imports",
            &format!(
                "import {{ {}Plugin }} from \"./{}\";",
                names.camel, names.kebab
            ),
            "index.ts",
        )?;
        insert_before_marker(
            &c,
            "// xtask:plugins",
            &format!("{}Plugin,", names.camel),
            "index.ts",
        )
    })?;
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::{insert_before_marker, next_field_number};

    #[test]
    fn inserts_with_the_marker_indent() {
        let patched = insert_before_marker("a\n    // m\nb\n", "// m", "x;", "f").unwrap();
        assert_eq!(patched, "a\n    x;\n    // m\nb\n");
    }

    #[test]
    fn finds_the_next_field_number() {
        let proto =
            "message A {\n  oneof game {\n    X x = 1;\n    Y y = 7;\n    // xtask:A\n  }\n}\n";
        assert_eq!(next_field_number(proto, "// xtask:A").unwrap(), 8);
    }
}
