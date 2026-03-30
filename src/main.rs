use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

enum CliCommand {
    Demo,
    DemoAutoplay,
    ImportOsu { folder: PathBuf },
}

fn main() -> anyhow::Result<()> {
    run_with_args(std::env::args_os(), run_osu_import, |autoplay| {
        code_m::app::run_demo_with_options(code_m::app::DemoLaunchOptions { autoplay })
    })
}

fn run_with_args<I, FImport, FDemo>(
    args: I,
    import_handler: FImport,
    demo_handler: FDemo,
) -> anyhow::Result<()>
where
    I: IntoIterator<Item = OsString>,
    FImport: FnOnce(&Path) -> anyhow::Result<()>,
    FDemo: FnOnce(bool) -> anyhow::Result<()>,
{
    match parse_cli_command(args)? {
        CliCommand::Demo => demo_handler(false),
        CliCommand::DemoAutoplay => demo_handler(true),
        CliCommand::ImportOsu { folder } => import_handler(&folder),
    }
}

fn parse_cli_command<I>(args: I) -> anyhow::Result<CliCommand>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Ok(CliCommand::Demo);
    };

    if command == "--import-osu" {
        let Some(folder) = args.next() else {
            bail!("usage: seythm --import-osu <folder>");
        };
        if args.next().is_some() {
            bail!("usage: seythm --import-osu <folder>");
        }
        return Ok(CliCommand::ImportOsu {
            folder: PathBuf::from(folder),
        });
    }

    if command == "autoplay" {
        if args.next().is_some() {
            bail!("usage: seythm autoplay");
        }
        return Ok(CliCommand::DemoAutoplay);
    }

    bail!("unrecognized argument: {}", command.to_string_lossy());
}

fn run_osu_import(folder: &Path) -> anyhow::Result<()> {
    let import_root = code_m::content::prepare_default_import_root()
        .unwrap_or_else(|_| code_m::content::default_import_root());
    let imported = code_m::osu::import::import_osu_mania_folder(folder, &import_root)
        .with_context(|| format!("failed to import osu folder {}", folder.display()))?;

    println!(
        "Imported {} osu!mania beatmap(s) into {}",
        imported.len(),
        import_root.display()
    );
    for entry in imported {
        println!(
            "- {} - {} [{}] -> {}",
            entry.artist,
            entry.title,
            entry.chart_name,
            entry.chart_path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(|arg| OsString::from(arg)).collect()
    }

    #[test]
    fn defaults_to_demo_without_import_flag() {
        let command = parse_cli_command(os_args(&["seythm"])).expect("parse");
        assert!(matches!(command, CliCommand::Demo));
    }

    #[test]
    fn parses_import_command_and_folder() {
        let command = parse_cli_command(os_args(&[
            "seythm",
            "--import-osu",
            "tests/fixtures/osu/valid-6k",
        ]))
        .expect("parse");

        match command {
            CliCommand::ImportOsu { folder } => {
                assert!(folder.ends_with("tests/fixtures/osu/valid-6k"));
            }
            CliCommand::Demo => panic!("expected import command"),
            CliCommand::DemoAutoplay => panic!("expected import command"),
        }
    }

    #[test]
    fn dispatches_to_import_without_starting_demo() {
        let mut demo_called = false;
        let mut import_called = false;

        run_with_args(
            os_args(&["seythm", "--import-osu", "tests/fixtures/osu/valid-6k"]),
            |folder| {
                import_called = true;
                assert!(folder.ends_with("tests/fixtures/osu/valid-6k"));
                Ok(())
            },
            |_| {
                demo_called = true;
                Ok(())
            },
        )
        .expect("dispatch");

        assert!(import_called);
        assert!(!demo_called);
    }

    #[test]
    fn parses_autoplay_command() {
        let command = parse_cli_command(os_args(&["seythm", "autoplay"])).expect("parse");
        assert!(matches!(command, CliCommand::DemoAutoplay));
    }

    #[test]
    fn dispatches_autoplay_to_demo_handler() {
        let mut demo_called = false;

        run_with_args(
            os_args(&["seythm", "autoplay"]),
            |_| panic!("import handler should not be called"),
            |autoplay| {
                demo_called = true;
                assert!(autoplay);
                Ok(())
            },
        )
        .expect("dispatch");

        assert!(demo_called);
    }
}
