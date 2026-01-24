use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct WuiExt;

impl zed::Extension for WuiExt {
	fn new() -> Self {
		Self
	}

	fn language_server_command(
		&mut self,
		_language_server_id: &LanguageServerId,
		worktree: &Worktree,
	) -> Result<Command> {
		let mut path = std::path::PathBuf::from(worktree.root_path());
		path = path.join("target").join("debug").join("wui-lsp");
		if cfg!(windows) {
			path.set_extension("exe");
		}

		Ok(Command {
			command: path.to_string_lossy().to_string(),
			args: vec!["--stdio".into()],
			env: Default::default(),
		})
	}
}

zed::register_extension!(WuiExt);
