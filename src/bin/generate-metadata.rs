use argh::FromArgs;
use clang::{Clang, Index};

#[derive(FromArgs)]
/// CLI Arguments
struct Arguments {
    #[argh(positional)]
    filename: String,
}

fn main() -> Result<(), ()> {
    let args: Arguments = argh::from_env();

    let clang = Clang::new().expect("Failed to create Clang instance");
    let index = Index::new(&clang, false, false);
    let translation_unit = index
        .parser(args.filename)
        .detailed_preprocessing_record(true)
        .parse()
        .expect("Failed to parse translation unit");
    let entities = translation_unit.get_entity().get_children();
    for entity in entities {
        if entity.get_kind() == clang::EntityKind::MacroDefinition {
            let name = entity.get_name().expect("Failed to get macro name");
            if name.starts_with("_") {
                continue;
            }
            let range = entity.get_range().expect("Failed to get macro value");
            let tokens = range.tokenize();
            match tokens.len() {
                1 => {
                    todo!()
                }
                2 => {
                    todo!()
                }
                _ => {
                    continue;
                }
            }
        }
    }

    Ok(())
}
